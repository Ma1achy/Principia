//! The lock's solver core (design `principia_chart_composition.md` § 17.8 and § 10.6; TASK-M2-94): a least-squares
//! solve over a map, "which chart point's state is nearest this one", the one solve the lock's re-solve (TASK-M2-65)
//! and placement's nearest-point route (TASK-M2-99) both call.
//!
//! **The problem.** A map `F : ℝ² → ℝᴺ` (a chart's forward code, or a frame of a family of them, a morph), a target
//! `y ∈ ℝᴺ` and a diagonal weight `w`: find a local minimiser of `½‖r(x)‖²`, `rᵢ(x) = wᵢ(Fᵢ(x) − yᵢ)`. The map gives
//! its value and its slopes ([`Map`]); the solver never differentiates. The weight is the caller's, because the metric
//! "nearest" is measured in is placement's definition to write (§ 10.6, owed by TASK-M2-99).
//!
//! **The step** ([`solve`]): damped Newton (Levenberg) on the 2×2 system `(H + μI)δ = −Jᵀr`, solved in closed form.
//! `H = JᵀJ + S`, where `S`, the residual's own curvature `Σ rᵢ∇²Fᵢ`, is estimated by a secant update from the steps
//! taken ([`secant`]), 0 at the seed, and used only where `H` is positive definite (else `H = JᵀJ`, Gauss–Newton's).
//! Gauss–Newton alone converges at the rate `ρ((JᵀJ)⁻¹S)`, which tends to 1 as a minimiser off the image nears a fold,
//! exactly where the lock's follower must still converge to see the minimiser end; the secant term restores Newton's
//! rate there. A trial is accepted only where the map is finite and the cost falls strictly; otherwise `μ` grows. So
//! the cost never rises while it can be resolved, and the minimiser reached is the one whose basin the seed's descent
//! is in. A rejected trial grows `μ` from at least its floor, `initial` times the seed's scale, so the trials of one
//! iteration are bounded whatever an accepted step's `decrease` has done to `μ`; settings that could not terminate
//! (`initial ≤ 0`, `increase ≤ 1`, `decrease ≤ 0`, `ceiling ≤ initial`) are refused, tagged, before anything is
//! evaluated. Where `μ` passes its ceiling the cost is, near a minimiser, flat to its rounding (within `√ε` of it in
//! `x`), and the solve polishes ([`polish`]): the undamped step with `H`, else with `JᵀJ`, accepted only where `‖Jᵀr‖`
//! falls, so first-order optimality is reached to the gradient's own rounding; where neither lowers it, the solve has
//! stalled. After a polishing step `μ` starts again from its floor, so the next step is a damped one, on the cost,
//! wherever the cost can be resolved. The ceiling is also passed where no trial is finite (at the edge of the map's
//! domain); a polish there descends `‖Jᵀr‖` alone and can stop at a stationary point that is not a minimiser, which
//! no test of this module rules out.
//!
//! **Convergence, per call** ([`Report`]): `Converged(OnImage)` where `‖r‖ ≤` the residual tolerance (the state is on
//! the chart's image), else `Converged(Stationary)` where first-order optimality holds, `‖P r‖ ≤ tol·‖r‖`, `P` the
//! projection onto the tangent plane, `‖P r‖² = (Jᵀr)ᵀ(JᵀJ)⁻¹(Jᵀr)` ([`projected`]; the sine of the residual's angle
//! from the plane's normal, the same whatever the scale of either chart coordinate): off the image the residual is the
//! projection distance, a measured quantity, never read as solver error (§ 10.6). Every call reports its point, status, branch, iterations,
//! evaluations, residual, optimality, conditioning and travel.
//!
//! **Failures, tagged, never dropped** ([`Failure`]): settings that cannot terminate; the map not finite at the seed; the iteration cap; a stall (no
//! damping up to the ceiling lowers the cost and no polishing step lowers `‖Jᵀr‖`, short of convergence); a converged
//! point whose `JᵀJ` is ill-conditioned, its smallest eigenvalue over its largest below the caller's floor, so the
//! minimiser is not isolated; and, for a continued solve, the followed minimiser ending.
//!
//! **The branch rule, recorded** ([`Branch`]). *Continued*: the solve starts at the previous frame's point and keeps
//! the minimiser its descent reaches ([`follow`]). *Restart*: the solve runs from each of `K` seeds in order and keeps
//! the converged one of least cost, the lowest index on a tie (strict `<`), so the choice is a pure function of the
//! seeds and the map ([`restart`]). The report names the branch taken, its seed and, for a restart, its index.
//!
//! **Following the local minimiser, and where it ends** ([`follow`]; design § 17.8: under a lock the anchor follows its
//! local minimiser continuously and stops at the last frame where that minimiser exists). A continued frame is
//! accepted only where its own step passes the continuity test the whole path is held to (G8's criterion: the largest
//! step falls to at most 0.6 of the previous each time the frame step halves, twice), nested inside the frame: the frame
//! is solved directly from the last point; then its interval is halved and the two halves solved in a chain, and the
//! larger half-step must fall to at most `ratio` of the interval's step (or the interval's step be of rounding size,
//! at or below `floor`), with the chain ending where the interval's own solve did, within `agree`. Where the halves
//! fail, the test descends into the half with the larger step and asks again, up to `depth` times. A minimiser whose
//! step is Lipschitz in `s` (away from a degenerate minimiser) passes once the descent reaches an interval over which
//! its speed is nearly constant (its step then halves with the interval), or its step falls to `floor`; one that moves
//! with unbounded speed at a degenerate minimiser, as `(s − s₀)^{1/3}`, is tagged `Ended` there; how deep that is depends on how unevenly it moves inside
//! the frame, not on whether it ends. One that ends inside the frame fails at every depth: the half holding the end
//! always holds the descent's slide into another basin, a step of finite size that does not shrink; and a probe solve
//! inside the frame that fails to converge (at a fold, where the minimiser is degenerate) is an ending too, whatever
//! its own tag (a stall, the iteration cap, an ill-conditioned point, the map's domain edge). At the tests' fold the
//! ending comes from such a probe hitting the iteration cap, so the frame it fires on depends on `max_iterations`;
//! were the probe to converge, the slide would be caught as a step that does not shrink. So a frame is an
//! ending only where the step has shrunk at none of `depth` levels, and `depth` is a bound on the work, not a
//! threshold: the tests run 32. Measured at depth 2 (`lock_solver_core_follow_smooth`'s control and the code review of
//! this rule): a minimiser moving as `s^2.5` or `s^3` from rest, or one turning back inside a frame (17 of 28 hairpins
//! in the review's sweep), was tagged `Ended` falsely; at depth 3 and more, none was. A jump smaller than the
//! continuous motion around it at the frame's resolution is not seen, as the whole-path test would not see it.
//! A chain through an interval's midpoint that ends elsewhere than the interval's own solve is an ending too: no one
//! minimiser joins the interval's ends. Such a frame is tagged [`Failure::Ended`], and its report is the frame's start,
//! its point and its numbers there: the last frame where the followed minimiser exists. Near a fold the minimiser's
//! speed grows as `1/√(s_fold − s)`, a half-step ratio of `1/√2 > 0.6` at every depth, so the test fires on the frame
//! that reaches to within about `2^−depth` of a frame of the fold: at depth 32, the frame that crosses it.
//!
//! The plan is unclear here; chose this nested test as the definition of "the followed minimiser ends" because the
//! definition the lock's paths are to read (TASK-M2-65's, under physics review) is not written yet, and this one is
//! G8's own criterion (the step falls as the frame step halves) applied inside one frame, descending until the step
//! either shrinks or is seen not to: an ending is a step that does not shrink, a jump, whatever the frame's size. Steps
//! are measured in chart coordinates, the solve's own unknowns. A chain through an interval's midpoint that ends
//! elsewhere than the interval's own solve is taken as an ending too (the frame's two routes disagree on which
//! minimiser it reaches), conservatively; no test reaches that arm.
//!
//! **Pure, generic, CPU-side.** Generic over the float type: the f64 path is the CPU's; the f32 path compiles and runs
//! on the host. No allocation, no precision-dependent constant: every tolerance, the damping and the continuity ratio
//! are the caller's ([`Spec`], [`Continuity`]). Off the SPIR-V build, as `nav` is: the CPU evaluates charts, and the
//! GPU integrates records and holds no chart code.

use spirv_std::num_traits::Float;

use crate::real::Real;

/// A map from a chart point `x = (u, v)` to `N` numbers, with its slopes. `None` where it is not defined (outside its
/// domain, or not finite).
pub trait Map<R, const N: usize> {
    /// `F(x)`.
    fn value(&self, x: [R; 2]) -> Option<[R; N]>;
    /// `∂F/∂x`: one column of `N` per coordinate, `[∂F/∂u, ∂F/∂v]`.
    fn jacobian(&self, x: [R; 2]) -> Option<[[R; N]; 2]>;
}

/// A one-parameter family of maps `F_s`, a morph's frames: frame `s` is the map `x ↦ F_s(x)`.
pub trait Family<R, const N: usize> {
    /// `F_s(x)`.
    fn value(&self, s: R, x: [R; 2]) -> Option<[R; N]>;
    /// `∂F_s/∂x`.
    fn jacobian(&self, s: R, x: [R; 2]) -> Option<[[R; N]; 2]>;
}

/// Frame `s` of a family, as a [`Map`].
pub struct Frame<'a, R, Fam> {
    /// The family.
    pub family: &'a Fam,
    /// The frame's parameter.
    pub s: R,
}

impl<R: Copy, const N: usize, Fam: Family<R, N>> Map<R, N> for Frame<'_, R, Fam> {
    #[inline]
    fn value(&self, x: [R; 2]) -> Option<[R; N]> {
        self.family.value(self.s, x)
    }

    #[inline]
    fn jacobian(&self, x: [R; 2]) -> Option<[[R; N]; 2]> {
        self.family.jacobian(self.s, x)
    }
}

/// The target `y` and the diagonal weight `w` of the residual `wᵢ(Fᵢ(x) − yᵢ)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target<R, const N: usize> {
    /// `y`, the state sought (under a lock, the anchor).
    pub value: [R; N],
    /// `w`, per component: the metric's, the caller's.
    pub weight: [R; N],
}

/// The damping of the Levenberg step, the caller's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Damping<R> {
    /// `μ` at the first iteration, as a fraction of the scale: the larger diagonal entry of `JᵀJ` at the seed, or 1 if
    /// that is smaller. Also `μ`'s floor on a rejected trial.
    pub initial: R,
    /// The factor `μ` grows by on a rejected trial.
    pub increase: R,
    /// The factor `μ` shrinks by on an accepted one.
    pub decrease: R,
    /// The largest `μ`, as a fraction of the same scale as `initial`; past it the solve polishes, and may stall.
    pub ceiling: R,
}

/// The solve's settings, the caller's: no tolerance is the solver's own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec<R> {
    /// The most accepted steps a call takes.
    pub max_iterations: u32,
    /// `‖r‖` at or below it: converged on the image.
    pub residual_tolerance: R,
    /// `‖P r‖ ≤` it `·‖r‖`, `P` the projection onto the tangent plane: converged by first-order optimality.
    pub optimality_tolerance: R,
    /// `JᵀJ`'s smallest eigenvalue over its largest, below it at a converged point: ill-conditioned.
    pub conditioning_floor: R,
    /// The damping.
    pub damping: Damping<R>,
    /// Under the `controls` feature only, and set only by a test's control: `true` drops the trial loop's exit on a
    /// μ that is no longer finite, the guard against `ceiling·scale` overflowing, so the control can show the
    /// termination test catches that hang. Per call, so tests running beside it are untouched.
    #[cfg(feature = "controls")]
    pub unguarded: bool,
    /// Under the `controls` feature only: `true` accepts a polishing step where `‖P r‖² = gᵀ(JᵀJ)⁻¹g` falls, in place
    /// of `‖Jᵀr‖²`, a diagnostic's one variable changed. Per call.
    #[cfg(feature = "controls")]
    pub polish_projected: bool,
}

impl<R> Spec<R> {
    /// The settings, with any control-only field at its guarded default. Callers outside the kernel build a `Spec`
    /// through this, not a struct literal: the `controls` field exists when the kernel's own feature is on, which a
    /// dependent crate's feature does not track (cargo unifies the kernel's features across the build).
    pub const fn new(
        max_iterations: u32,
        residual_tolerance: R,
        optimality_tolerance: R,
        conditioning_floor: R,
        damping: Damping<R>,
    ) -> Self {
        Self {
            max_iterations,
            residual_tolerance,
            optimality_tolerance,
            conditioning_floor,
            damping,
            #[cfg(feature = "controls")]
            unguarded: false,
            #[cfg(feature = "controls")]
            polish_projected: false,
        }
    }
}

/// The continuity test a continued frame is held to ([`follow`]), the caller's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Continuity<R> {
    /// The largest step must fall to at most this fraction of the previous level's at each halving (G8: 0.6).
    pub ratio: R,
    /// A step at or below it is of rounding size and passes whatever the ratio. Set it at or above the rounding of
    /// the chart coordinates, `ε·|x|`: below that (1e-9 at f32, say) a near-stationary step is noise and never reaches
    /// it.
    pub floor: R,
    /// A chain through an interval's midpoint must end within it of the interval's own solve.
    pub agree: R,
    /// The most nested halvings the test descends through; a frame is an ending only if its step has shrunk at none of
    /// them. Deep enough that a continuous minimiser's step either halves or falls to `floor` first (the tests: 32).
    pub depth: u32,
}

/// How a call converged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Criterion {
    /// `‖r‖` at or below the residual tolerance: the target is on the map's image.
    OnImage,
    /// First-order optimality: the residual is normal to the tangent plane, to the tolerance.
    Stationary,
}

/// Why a call failed. The report keeps its point and its numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The caller's settings cannot terminate: the damping's `initial` not positive, `increase` not above 1,
    /// `decrease` not positive, or `ceiling` not above `initial` (or any of them not finite). Nothing is evaluated.
    Settings,
    /// The map, or its slopes, not finite at the seed.
    NonFinite,
    /// The iteration cap reached short of convergence.
    IterationCap,
    /// No damping up to the ceiling lowers the cost, nor the polishing step `‖Jᵀr‖`, short of convergence.
    Stalled,
    /// Converged, but `JᵀJ` is below the conditioning floor: the minimiser is not isolated.
    IllConditioned,
    /// A continued frame failed the continuity test: the followed minimiser ends in this frame.
    Ended,
}

/// A call's outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Converged, by the criterion named.
    Converged(Criterion),
    /// Failed, tagged.
    Failed(Failure),
}

/// The branch rule a call took, recorded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Branch<R> {
    /// A single solve from `seed`: the minimiser its descent reaches (a continued solve seeds at the previous frame).
    Continued {
        /// The seed.
        seed: [R; 2],
    },
    /// Seed `index` of `count`, kept as the converged one of least cost, the lowest index on a tie.
    Restart {
        /// The seed kept.
        seed: [R; 2],
        /// Its index among the seeds.
        index: u32,
        /// How many seeds were tried.
        count: u32,
    },
}

/// A call's report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Report<R> {
    /// The point reached: a stationary point of the cost where converged (the descent's own, accepted only where the
    /// cost fell, so in practice its basin's minimiser, but a seed at a stationary point that is not a minimiser is
    /// reported there, converged, with no step taken); the last accepted point where not; for an ended frame, the frame's
    /// start; the seed where nothing was evaluated.
    pub x: [R; 2],
    /// The outcome.
    pub status: Status,
    /// The branch rule taken.
    pub branch: Branch<R>,
    /// Accepted steps.
    pub iterations: u32,
    /// Map evaluations (value and slopes counted once per point).
    pub evaluations: u32,
    /// `‖r‖` at `x`: the weighted distance from the target to `F(x)`, the projection distance off the image.
    pub residual: R,
    /// `‖P r‖ / ‖r‖` at `x`, `P` the projection onto the tangent plane, 0 where `r = 0`.
    pub optimality: R,
    /// `JᵀJ`'s smallest eigenvalue over its largest at `x`.
    pub conditioning: R,
    /// `‖x − seed‖`: how far the solve moved.
    pub travel: R,
}

impl<R> Report<R> {
    /// Whether the call converged.
    #[inline]
    pub fn converged(&self) -> bool {
        matches!(self.status, Status::Converged(_))
    }
}

/// Finite: `x − x` is 0 only for a finite `x` (∞ − ∞ and NaN − NaN are NaN), no epsilon or largest value read.
#[inline]
#[allow(clippy::eq_op)] // `x − x` is the test itself: 0 for a finite `x`, NaN otherwise.
fn finite<R: Real>(x: R) -> bool {
    x - x == R::default()
}

/// The weighted residual, `rᵢ = wᵢ(Fᵢ − yᵢ)`, and `‖r‖²`; `None` where not finite.
#[inline]
fn residual<R: Real, const N: usize>(f: [R; N], y: &[R; N], w: &[R; N]) -> Option<([R; N], R)> {
    let mut r = [R::default(); N];
    let mut cost = R::default();
    for i in 0..N {
        r[i] = w[i] * (f[i] - y[i]);
        cost = r[i].fma(r[i], cost);
    }
    finite(cost).then_some((r, cost))
}

/// `JᵀJ` as `(a₁₁, a₁₂, a₂₂)` and `Jᵀr`, of the weighted slopes; `None` where not finite.
#[inline]
fn normal<R: Real, const N: usize>(
    j: &[[R; N]; 2],
    r: &[R; N],
    w: &[R; N],
) -> Option<([R; 3], [R; 2])> {
    let (mut a11, mut a12, mut a22) = (R::default(), R::default(), R::default());
    let (mut g1, mut g2) = (R::default(), R::default());
    for i in 0..N {
        let ju = w[i] * j[0][i];
        let jv = w[i] * j[1][i];
        a11 = ju.fma(ju, a11);
        a12 = ju.fma(jv, a12);
        a22 = jv.fma(jv, a22);
        g1 = ju.fma(r[i], g1);
        g2 = jv.fma(r[i], g2);
    }
    let all = a11 + a12 + a22 + g1 + g2;
    finite(all).then_some(([a11, a12, a22], [g1, g2]))
}

/// The larger of two.
#[inline]
fn max<R: Real>(a: R, b: R) -> R {
    if a < b {
        b
    } else {
        a
    }
}

/// `JᵀJ`'s smallest eigenvalue over its largest, from `(a₁₁, a₁₂, a₂₂)`; 0 where `JᵀJ = 0`.
#[inline]
fn conditioning<R: Real + Float>(a: [R; 3]) -> R {
    let [a11, a12, a22] = a;
    let two = R::one() + R::one();
    let half_trace = (a11 + a22) / two;
    let half_gap = (a11 - a22) / two;
    let root = half_gap.fma(half_gap, a12 * a12).square_root();
    let large = half_trace + root;
    if large > R::default() {
        // λ_min = det/λ_max, not half_trace − root, which cancels when JᵀJ is near singular.
        let small = a11.fma(a22, -(a12 * a12)) / large;
        max(small, R::default()) / large
    } else {
        R::default()
    }
}

/// The evaluated point: its cost `‖r‖²` and normal equations.
struct Point<R> {
    x: [R; 2],
    cost: R,
    a: [R; 3],
    g: [R; 2],
}

/// The map's residual and normal equations at `x`, or `None` where any is not finite.
#[inline]
fn evaluate<R: Real, const N: usize, M: Map<R, N>>(
    map: &M,
    y: &[R; N],
    w: &[R; N],
    x: [R; 2],
) -> Option<Point<R>> {
    if !(finite(x[0]) && finite(x[1])) {
        return None;
    }
    let (r, cost) = residual(map.value(x)?, y, w)?;
    let (a, g) = normal(&map.jacobian(x)?, &r, w)?;
    Some(Point { x, cost, a, g })
}

/// The convergence test at `p`: `Some(criterion)` where it holds.
#[inline]
fn converged<R: Real + Float>(p: &Point<R>, spec: &Spec<R>) -> Option<Criterion> {
    let tol = spec.residual_tolerance;
    if p.cost <= tol * tol {
        return Some(Criterion::OnImage);
    }
    let o = spec.optimality_tolerance;
    (projected(p) <= o * o * p.cost).then_some(Criterion::Stationary)
}

/// `‖P r‖² = gᵀ(JᵀJ)⁻¹g`, `g = Jᵀr`: the squared length of the residual's projection onto the tangent plane (the
/// range of `J`), so `‖P r‖/‖r‖` is the sine of the residual's angle from the plane's normal, whatever the scale of
/// either coordinate. Where `JᵀJ` is singular, `‖Jᵀr‖²/‖J‖²_F` in its place, a lower bound, and such a point is
/// tagged ill-conditioned whenever it converges.
#[inline]
fn projected<R: Real + Float>(p: &Point<R>) -> R {
    let [a11, a12, a22] = p.a;
    let [g1, g2] = p.g;
    let det = a11.fma(a22, -(a12 * a12));
    let quadratic = a22 * g1 * g1 - (a12 + a12) * g1 * g2 + a11 * g2 * g2;
    let value = quadratic / det;
    if det > R::default() && finite(value) && value >= R::default() {
        value
    } else {
        let jj = a11 + a22;
        if jj > R::default() {
            gradient_squared(p) / jj
        } else {
            R::default()
        }
    }
}

/// `‖P r‖/‖r‖` at `p` ([`projected`]), 0 where `r = 0`.
#[inline]
fn optimality<R: Real + Float>(p: &Point<R>) -> R {
    if p.cost > R::default() {
        (projected(p) / p.cost).square_root()
    } else {
        R::default()
    }
}

/// `‖Jᵀr‖²` at `p`.
#[inline]
fn gradient_squared<R: Real>(p: &Point<R>) -> R {
    p.g[0].fma(p.g[0], p.g[1] * p.g[1])
}

/// The polishing step, where no damping lowers the cost: the undamped step `Hδ = −Jᵀr`, `H` the curvature in use
/// ([`curvature`]), accepted where the map is finite and `‖Jᵀr‖` falls; where `H` is not `JᵀJ` and its step does not
/// lower `‖Jᵀr‖`, Gauss–Newton's, `JᵀJδ = −Jᵀr`, on the same test (the secant `S` is built from gradients, and where
/// those are noisy, slopes by differences, it can be off by their noise). Near a minimiser the cost is flat to its
/// rounding, `ε‖r‖²`, a distance of order `√ε` in `x`, so a rule on the cost alone stalls there; the gradient is
/// resolved to `ε`, and the step goes on to its zero.
#[inline]
fn polish<R: Real + Float, const N: usize, M: Map<R, N>>(
    map: &M,
    y: &[R; N],
    w: &[R; N],
    p: &Point<R>,
    h: [R; 3],
    evaluations: &mut u32,
    #[cfg(feature = "controls")] scaled: bool,
) -> Option<Point<R>> {
    #[cfg(feature = "controls")]
    let falls = |q: &Point<R>| match scaled {
        true => projected(q) < projected(p),
        false => gradient_squared(q) < gradient_squared(p),
    };
    #[cfg(not(feature = "controls"))]
    let falls = |q: &Point<R>| gradient_squared(q) < gradient_squared(p);
    let mut attempt = |m: [R; 3]| {
        let delta = step(m, R::default(), p.g)?;
        *evaluations += 1;
        evaluate(map, y, w, [p.x[0] + delta[0], p.x[1] + delta[1]]).filter(|q| falls(q))
    };
    attempt(h).or_else(|| if h == p.a { None } else { attempt(p.a) })
}

/// The step `δ` of `(H + μI)δ = −g`, by Cramer's rule on the 2×2, where `H + μI` is positive definite; `None` where not.
#[inline]
fn step<R: Real>(h: [R; 3], mu: R, g: [R; 2]) -> Option<[R; 2]> {
    let b11 = h[0] + mu;
    let b22 = h[2] + mu;
    let det = b11.fma(b22, -(h[1] * h[1]));
    if !(b11 > R::default() && det > R::default()) {
        return None;
    }
    let du = (h[1].fma(g[1], -(b22 * g[0]))) / det;
    let dv = (h[1].fma(g[0], -(b11 * g[1]))) / det;
    (finite(du) && finite(dv)).then_some([du, dv])
}

/// The curvature the step uses: `JᵀJ + S` where that is positive definite, else `JᵀJ` (the Gauss–Newton matrix).
#[inline]
fn curvature<R: Real>(a: [R; 3], s: [R; 3]) -> [R; 3] {
    let h = [a[0] + s[0], a[1] + s[1], a[2] + s[2]];
    let det = h[0].fma(h[2], -(h[1] * h[1]));
    if h[0] > R::default() && det > R::default() {
        h
    } else {
        a
    }
}

/// The secant update of `S`, the residual's own curvature `Σ rᵢ∇²Fᵢ` that Gauss–Newton drops, after the step from `p`
/// to `q`: the symmetric (Powell) update that makes `(J₊ᵀJ₊ + S₊)·d = g₊ − g`, `d = x₊ − x`, changing `S` least in the
/// Frobenius norm. Its denominator is `dᵀd`, positive for any step taken, so it needs no skip rule; `S` is kept where
/// the update is not finite.
#[inline]
fn secant<R: Real>(s: [R; 3], p: &Point<R>, q: &Point<R>) -> [R; 3] {
    let d = [q.x[0] - p.x[0], q.x[1] - p.x[1]];
    let dd = d[0].fma(d[0], d[1] * d[1]);
    // u = (g₊ − g) − J₊ᵀJ₊·d − S·d: what the secant equation still lacks.
    let u = [
        q.g[0] - p.g[0] - q.a[0] * d[0] - q.a[1] * d[1] - s[0] * d[0] - s[1] * d[1],
        q.g[1] - p.g[1] - q.a[1] * d[0] - q.a[2] * d[1] - s[1] * d[0] - s[2] * d[1],
    ];
    let ud = u[0].fma(d[0], u[1] * d[1]);
    let c = ud / (dd * dd);
    let next = [
        s[0] + (u[0] * d[0] + u[0] * d[0]) / dd - c * d[0] * d[0],
        s[1] + (u[0] * d[1] + d[0] * u[1]) / dd - c * d[0] * d[1],
        s[2] + (u[1] * d[1] + u[1] * d[1]) / dd - c * d[1] * d[1],
    ];
    if finite(next[0]) && finite(next[1]) && finite(next[2]) {
        next
    } else {
        s
    }
}

/// The distance between two chart points.
#[inline]
pub fn distance<R: Real + Float>(a: [R; 2], b: [R; 2]) -> R {
    let du = a[0] - b[0];
    let dv = a[1] - b[1];
    du.fma(du, dv * dv).square_root()
}

/// The least-squares solve of `wᵢ(Fᵢ(x) − yᵢ)` over `map` from `seed`: the minimiser the seed's descent reaches, with its
/// report, its branch `Continued { seed }` (the module's note).
pub fn solve<R, const N: usize, M>(
    map: &M,
    target: &Target<R, N>,
    seed: [R; 2],
    spec: &Spec<R>,
) -> Report<R>
where
    R: Real + Float,
    M: Map<R, N>,
{
    let branch = Branch::Continued { seed };
    let zero = R::default();
    let (y, w) = (&target.value, &target.weight);
    let d = &spec.damping;
    let terminates = [d.initial, d.increase, d.decrease, d.ceiling]
        .iter()
        .all(|v| finite(*v))
        && d.initial > zero
        && d.increase > R::one()
        && d.decrease > zero
        && d.ceiling > d.initial;
    if !terminates {
        return Report {
            x: seed,
            status: Status::Failed(Failure::Settings),
            branch,
            iterations: 0,
            evaluations: 0,
            // Nothing was measured.
            residual: R::nan(),
            optimality: R::nan(),
            conditioning: R::nan(),
            travel: zero,
        };
    }
    let Some(mut p) = evaluate(map, y, w, seed) else {
        return Report {
            x: seed,
            status: Status::Failed(Failure::NonFinite),
            branch,
            iterations: 0,
            evaluations: 1,
            // Nothing finite was measured.
            residual: R::nan(),
            optimality: R::nan(),
            conditioning: R::nan(),
            travel: zero,
        };
    };
    let mut evaluations = 1;
    let mut iterations = 0;
    let scale = max(max(p.a[0], p.a[2]), R::one());
    // μ never falls below `floor` on a rejection, so it cannot sit at 0 (an accepted step's `decrease` may underflow
    // it there): from `floor` it grows geometrically past `ceiling`, so each iteration's trials are bounded, about
    // log(ceiling/initial)/log(increase) of them.
    let floor = d.initial * scale;
    let mut mu = floor;
    let ceiling = d.ceiling * scale;
    // S, the secant estimate of the residual's curvature, 0 at the seed (the first step is Gauss–Newton's).
    let mut secant_s = [zero; 3];
    let status = loop {
        if let Some(c) = converged(&p, spec) {
            break if conditioning(p.a) < spec.conditioning_floor {
                Status::Failed(Failure::IllConditioned)
            } else {
                Status::Converged(c)
            };
        }
        if iterations >= spec.max_iterations {
            break Status::Failed(Failure::IterationCap);
        }
        let h = curvature(p.a, secant_s);
        // Damped: (H + μI)δ = −Jᵀr, μ grown until the cost falls.
        let grow = |mu: R| max(mu * d.increase, floor);
        let accepted = loop {
            // Past the ceiling, or no longer finite: `ceiling = d.ceiling·scale` can overflow to ∞ for a finite
            // `d.ceiling` (a steep map, or `d.ceiling` near the largest value), and μ with it, where `∞ > ∞` is false.
            #[cfg(feature = "controls")]
            let guarded = !spec.unguarded;
            #[cfg(not(feature = "controls"))]
            let guarded = true;
            if mu > ceiling || (guarded && !finite(mu)) {
                // After a polishing step μ starts again from its floor, so where the cost can be resolved again
                // the next step is a damped one, accepted on the cost.
                let polished = polish(
                    map,
                    y,
                    w,
                    &p,
                    h,
                    &mut evaluations,
                    #[cfg(feature = "controls")]
                    spec.polish_projected,
                );
                if polished.is_some() {
                    mu = floor;
                }
                break polished;
            }
            let Some(delta) = step(h, mu, p.g) else {
                mu = grow(mu);
                continue;
            };
            evaluations += 1;
            match evaluate(map, y, w, [p.x[0] + delta[0], p.x[1] + delta[1]]) {
                Some(q) if q.cost < p.cost => {
                    mu = mu * d.decrease;
                    break Some(q);
                }
                _ => mu = grow(mu),
            }
        };
        match accepted {
            Some(q) => {
                secant_s = secant(secant_s, &p, &q);
                p = q;
                iterations += 1;
            }
            None => break Status::Failed(Failure::Stalled),
        }
    };
    Report {
        x: p.x,
        status,
        branch,
        iterations,
        evaluations,
        residual: p.cost.square_root(),
        optimality: optimality(&p),
        conditioning: conditioning(p.a),
        travel: distance(p.x, seed),
    }
}

/// The restart rule: [`solve`] from each of `seeds` in order, keeping the converged report of least residual, the lowest
/// index on a tie; where none converges, seed 0's report. Its branch is `Restart` with the index kept.
pub fn restart<R, const N: usize, const K: usize, M>(
    map: &M,
    target: &Target<R, N>,
    seeds: &[[R; 2]; K],
    spec: &Spec<R>,
) -> Report<R>
where
    R: Real + Float,
    M: Map<R, N>,
{
    const { assert!(K > 0, "restart takes at least one seed") };
    let mut best: Option<(u32, Report<R>)> = None;
    let mut first: Option<Report<R>> = None;
    for (k, seed) in seeds.iter().enumerate() {
        let report = solve(map, target, *seed, spec);
        if first.is_none() {
            first = Some(report);
        }
        if report.converged()
            && best
                .as_ref()
                .is_none_or(|(_, b)| report.residual < b.residual)
        {
            best = Some((k as u32, report));
        }
    }
    let (index, mut report) = match (best, first) {
        (Some(kept), _) => kept,
        (None, Some(report)) => (0, report),
        (None, None) => unreachable!("restart takes at least one seed"),
    };
    report.branch = Branch::Restart {
        seed: seeds[index as usize],
        index,
        count: K as u32,
    };
    report
}

/// The continued solve of one frame, `s_from → s_to`, from `x_from`, the point at `s_from`, under the continuity test
/// (the module's note). Converged and continuous: the direct solve's report. The direct solve failing: its report, at
/// `s_to`. A probe of the test failing to converge, or the continuity
/// test failed: `Failed(Ended)`, the report of `x_from` at `s_from` (its numbers there, `travel` 0, `branch`
/// `Continued { seed: x_from }`), the last point where the followed minimiser exists.
pub fn follow<R, const N: usize, Fam>(
    family: &Fam,
    target: &Target<R, N>,
    s_from: R,
    x_from: [R; 2],
    s_to: R,
    spec: &Spec<R>,
    continuity: &Continuity<R>,
) -> Report<R>
where
    R: Real + Float,
    Fam: Family<R, N>,
{
    let at = |s: R, seed: [R; 2]| solve(&Frame { family, s }, target, seed, spec);
    let direct = at(s_to, x_from);
    if !direct.converged() {
        return direct;
    }
    // The nested test (the module's note): halve the interval, solve its two halves in a chain from its start; the
    // halves' larger step must fall to `ratio` of the interval's, or the interval's step be of rounding size. Where it
    // does not, the test descends into the half with the larger step, up to `depth` times; only a step that has not
    // shrunk at any of them is an ending.
    let two = R::one() + R::one();
    let (mut a, mut xa, mut b, mut xb, mut d) = (s_from, x_from, s_to, direct.x, direct.travel);
    let mut continuous = false;
    for _ in 0..continuity.depth {
        if d <= continuity.floor {
            continuous = true;
            break;
        }
        let mid = a + (b - a) / two;
        // A probe inside the frame that does not converge is an ending: the followed minimiser cannot be continued
        // there (at a fold the probe lands on the degenerate minimiser, where the solve slows without bound).
        let m = at(mid, xa);
        if !m.converged() {
            break;
        }
        let e = at(b, m.x);
        if !e.converged() {
            break;
        }
        if distance(e.x, xb) > continuity.agree {
            // The chain through the midpoint lands elsewhere than the interval's own solve: no single minimiser joins
            // the interval's ends.
            break;
        }
        let (d1, d2) = (m.travel, e.travel);
        if max(d1, d2) <= continuity.ratio * d {
            continuous = true;
            break;
        }
        if d1 < d2 {
            (a, xa, d) = (mid, m.x, d2);
        } else {
            (b, xb, d) = (mid, m.x, d1);
        }
    }
    if continuous {
        return direct;
    }
    // The frame's start, re-read at `s_from`: a solve seeded at its own minimiser, so its numbers are that point's.
    let here = at(s_from, x_from);
    Report {
        x: x_from,
        status: Status::Failed(Failure::Ended),
        branch: Branch::Continued { seed: x_from },
        travel: R::default(),
        ..here
    }
}

/// Central differences of `value` at `x`, step `h` per coordinate: the check on a map's slopes, and the stand-in where a
/// map has no exact slopes yet.
pub fn central_difference<R, const N: usize>(
    value: impl Fn([R; 2]) -> Option<[R; N]>,
    x: [R; 2],
    h: R,
) -> Option<[[R; N]; 2]>
where
    R: Real + Float,
{
    let two_h = h + h;
    let mut out = [[R::default(); N]; 2];
    for (axis, column) in out.iter_mut().enumerate() {
        let mut plus = x;
        let mut minus = x;
        plus[axis] = x[axis] + h;
        minus[axis] = x[axis] - h;
        let (fp, fm) = (value(plus)?, value(minus)?);
        for i in 0..N {
            column[i] = (fp[i] - fm[i]) / two_h;
        }
    }
    Some(out)
}
