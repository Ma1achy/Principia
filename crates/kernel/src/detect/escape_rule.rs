//! The escape rule, selectable as the integrators are: a type the wrapper's cadence is generic over
//! (`crate::driver::cadence::WrapperCadence<R, E>`), so a march names its rule as it names its occupant, and a build
//! compiles only the rules it instantiates. The default is [`DefaultRule`], [`Fewbody`].
//!
//! **The rules.**
//! - [`Closure`]: the closure-and-`E_rel` rule (`super::escape`), unchanged.
//! - [`EnergyFloor`]: prin-rs's naive energy baseline (`examples/chazy_probe.rs::energy_naive` at prin-rs `57a7f5e`):
//!   the tightest pair is the binary and the third body's two-body energy against it decides. With the pair bound, the
//!   third body unbound is an escape of that body; with the pair unbound too, a triple ejection.
//! - [`DistanceFloor`]: prin-rs's naive distance baseline (`dist_naive`): the body farthest from the other two's
//!   centre of mass, beyond [`ESCAPE_DISTANCE_FLOOR`]` · R_0`, has escaped; a triple ejection where the other two are
//!   also that far apart.
//!
//! - [`Chazy`]: prin-rs's Chazy final-motion classifier, live (`crate::chazy::classify` at the register's defaults, on
//!   the state and the cadence's `I_max` latch), committed where it reads an escape: H, HP or P is a triple ejection,
//!   HE or PE the escape of the body most separated from its pair (the body its hierarchy step read). B-stable and the
//!   four working states commit nothing, and the run goes on.
//!
//! - [`Fewbody`]: Fewbody's termination test (Fregeau et al. 2004, §3.3.5), ported from
//!   `validation::chazy::fewbody_resolved` (the reference, f64, kept in validation as the cross-check): the bound pair of
//!   least Kepler `a`; with none, a triple ejection where every pair recedes and `E_0 > 0`; with one, the escape of its
//!   third body where that body is unbound from the pair, recedes and the pair's tidal ratio is below the register's
//!   `δ`. B-stable and unresolved commit nothing.
//!
//! **The label is live** (the owner, 23 Nov: "I wouldn't by default stop integration at the first label; I'd make that
//! an option"). Every rule re-reads its test at every window mark, on the state and its own latches (the closure rule's
//! lagged shape point, Chazy's `I_max`), so its verdict is the label at the playhead and may change, a body's escape
//! included, as the march goes on. Whether the first label stops the march and stays is the cadence's option
//! (`crate::driver::cadence::WrapperCadence`'s `stop_on_escape`, off by default): with it on, the cadence stops calling
//! [`EscapeRule::sample`] once a rule has fired, which is the latched march of before, bit for bit.
//!
//! **Their forms, ported.** prin-rs writes the two-body tests as `K` against `U`. The outer body's `K_out > U_out` is
//! `½μ_out|Δv|² > m_b M_pair/d` with `μ_out = m_b M_pair/(m_b + M_pair)`, which is `½|Δv|² > (m_b + M_pair)/d`: the
//! detector's own `E_rel > 0`, so the floors read [`Relative::unbound`]. A pair's `K < U` is decided multiplied
//! through, as `super::escape::pair_unbound` decides its energy half: `(μ|Δv|²)²·r² < 4(mᵢmⱼ)²` ([`pair_bound`]).
//!
//! **Sampling.** Every rule is tested at the escape window's marks ([`super::escape::is_mark`]), on the state the
//! sampling point holds, and its verdict is that mark's until the next. So a rule's label is a function of the march
//! and the rule alone, and the harness that measures them reads each at the same points.
//!
//! **What a resumed march reloads.** Only [`Closure`] reads a register (the lagged shape point, `SimState`'s
//! `_nhat_lag_*`), but every rule keeps it, the shape point at its last window mark ([`ShapeLag`]), so the stored
//! register is the same under every rule and a re-booted march reads a non-zero one as a `SimState` the wrapper wrote,
//! and reloads it (`WrapperCadence`'s `resume`); a rule that stored zeros there would make every re-boot skip the
//! monitor's and the collision detector's reload. [`Chazy`] also holds `E_0`, reloaded as the closure rule's is, and
//! reads `I_max` from the cadence, whose latch a re-boot reloads. A live label a re-booted march stored is taken up by
//! [`EscapeRule::hold`], so a march cut between two marks writes the uncut march's label. `I_max` steers OS-candidate alone, a
//! working state, so a Chazy commit does not depend on it; it is passed so the classifier runs as the read side's does. The distance floor's `R_0` is the gauge's, 1: the canonical gauge
//! has `M = I = 1` at `t = 0`, so `R_0 = √(I_0/M) = 1`, read as given and never recomputed from a state (integrator
//! contract Part 5, as `ε` is read), so a march re-booted from any stored state reaches the same verdict. [`Chazy`]
//! reads the same `R_0`.

use crate::chazy::{
    classify, kepler, most_separated, out_kup, pair_kup, tidal_ratio, ChazyCfg, Motion, Playhead,
    Previous,
};
use crate::detect::escape::{self, is_mark, relative, Detector, EscapeReal, Relative, Verdict};
use crate::generated::constants::{CHAZY_DELTA, ESCAPE_DISTANCE_FLOOR};
use crate::payload::{
    DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION, DETAIL_ESCAPE_BODY_0, DETAIL_ESCAPE_BODY_1,
    DETAIL_ESCAPE_BODY_2,
};
use crate::shape::shape;

/// An escape rule the wrapper's cadence holds (the module's note).
pub trait EscapeRule<R: EscapeReal>: Copy + core::fmt::Debug + PartialEq {
    /// The rule of a run from masses `m`, positions `r` and momenta `p` whose stored initial energy is `e_0`, nothing
    /// fired.
    fn start(m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3], e_0: R) -> Self;
    /// Takes up a march still running from what its `SimState` holds: the lagged shape register `n_lag` and `E_0`.
    fn resume(&mut self, n_lag: [R; 3], e_0: R);
    /// The occupant's sampling point `index` on its grid of `quantum`, at the state `(m, r, p)`, with the cadence's
    /// Chazy latch `i_max`, the greatest moment of inertia met so far (`crate::chazy::ImaxLatch`), which only
    /// [`Chazy`] reads.
    fn sample(
        &mut self,
        m: &[R; 3],
        r: &[[R; 2]; 3],
        p: &[[R; 2]; 3],
        index: u32,
        quantum: R,
        i_max: R,
    );
    /// Takes up the live label a re-booted march stored, an escape with `detail` (the escaper, or 3 for a triple
    /// ejection): the verdict until the next window mark re-reads it.
    fn hold(&mut self, detail: u32);
    /// Whether the label is an escape at this playhead.
    fn fired(&self) -> bool;
    /// The verdict.
    fn verdict(&self) -> Verdict;
    /// The register `SimState` stores (`_nhat_lag_*`): the lagged shape point, which every rule keeps ([`ShapeLag`];
    /// the module's note); a resumed march reads three zeros as nothing to reload.
    fn lagged(&self) -> [R; 3];
}

/// Today's rule: closure and `E_rel` (`super::escape`).
pub type Closure<R> = Detector<R>;

/// The default rule, the one a march that names none runs (`WrapperCadence`'s default, `integrate_detecting`): one name,
/// so the default changes here alone. Fewbody, picked by escape-live-label's re-score (`validation::escape_checks::live`,
/// its rule written before the run): the only rule with no wrong label, wrong body included, over #49's fixtures, Burrau,
/// the periodic orbits, the stable triples and the positive-energy slice; Burrau's first label at 77.2 of Szebehely &
/// Peters' units, body 0.
pub type DefaultRule<R> = Fewbody<R>;

impl<R: EscapeReal> EscapeRule<R> for Detector<R> {
    #[inline]
    fn start(m: &[R; 3], r: &[[R; 2]; 3], _p: &[[R; 2]; 3], e_0: R) -> Self {
        Detector::new(m, r, e_0)
    }

    #[inline]
    fn resume(&mut self, n_lag: [R; 3], e_0: R) {
        *self = Detector::resumed(n_lag, e_0);
    }

    #[inline]
    fn sample(
        &mut self,
        m: &[R; 3],
        r: &[[R; 2]; 3],
        p: &[[R; 2]; 3],
        index: u32,
        quantum: R,
        _i_max: R,
    ) {
        Detector::sample(self, m, r, p, index, quantum)
    }

    #[inline]
    fn hold(&mut self, detail: u32) {
        Detector::hold(self, stored(detail));
    }

    #[inline]
    fn fired(&self) -> bool {
        Detector::fired(self)
    }

    #[inline]
    fn verdict(&self) -> Verdict {
        Detector::verdict(self)
    }

    #[inline]
    fn lagged(&self) -> [R; 3] {
        Detector::lagged(self)
    }
}

/// The verdict before anything fires.
#[inline]
fn not_fired() -> Verdict {
    Verdict {
        fired: false,
        escaper: DETAIL_ESCAPE_BODY_0,
        detail: DETAIL_ESCAPE_BODY_0,
    }
}

/// The verdict of a label stored with `detail`: fired, its escaper the body `detail` names, body 0 for a triple ejection,
/// whose escaper nothing reads.
#[inline]
pub fn stored(detail: u32) -> Verdict {
    Verdict {
        fired: true,
        escaper: body_code(detail),
        detail: if detail == DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION {
            DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION
        } else {
            body_code(detail)
        },
    }
}

/// The lagged shape point a rule that does not read it keeps all the same (the module's note): `n̂` at the IC, then at
/// each window mark the sampling point is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeLag<R>(pub [R; 3]);

impl<R: EscapeReal> ShapeLag<R> {
    /// The register of a run from masses `m` at positions `r`: `n̂(0)`, as the closure detector's.
    #[inline]
    pub fn new(m: &[R; 3], r: &[[R; 2]; 3]) -> Self {
        ShapeLag(shape(*r, *m))
    }

    /// The register at the sampling point `index` on its grid of `quantum`: the state's shape point where the point is a
    /// window mark, as the closure detector's register moves.
    #[inline]
    pub fn mark(&mut self, m: &[R; 3], r: &[[R; 2]; 3], index: u32, quantum: R) {
        if is_mark(index, quantum) {
            self.0 = shape(*r, *m);
        }
    }
}

/// The body's `detail` code, `0..3`, by constant comparisons (gpu_determinism_note rule 5).
#[inline]
fn body_code(b: u32) -> u32 {
    if b == 1 {
        DETAIL_ESCAPE_BODY_1
    } else if b == 2 {
        DETAIL_ESCAPE_BODY_2
    } else {
        DETAIL_ESCAPE_BODY_0
    }
}

/// A fired verdict naming `escaper`, a triple ejection where `triple`.
#[inline]
fn fired(escaper: u32, triple: bool) -> Verdict {
    Verdict {
        fired: true,
        escaper: body_code(escaper),
        detail: if triple {
            DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION
        } else {
            body_code(escaper)
        },
    }
}

/// Whether the pair `(mi, ri, pi)`, `(mj, rj, pj)` is bound, `K < U`: `½μ|Δv|² < mᵢmⱼ/r`, decided as
/// `(μ|Δv|²)²·r² < 4(mᵢmⱼ)²` (the module's note).
#[inline]
pub fn pair_bound<R: EscapeReal>(
    (mi, ri, pi): (R, [R; 2], [R; 2]),
    (mj, rj, pj): (R, [R; 2], [R; 2]),
) -> bool {
    let dr = [rj[0] - ri[0], rj[1] - ri[1]];
    let dv = [pj[0] / mj - pi[0] / mi, pj[1] / mj - pi[1] / mi];
    let mm = mi * mj;
    let k = mm / (mi + mj) * (dv[0] * dv[0] + dv[1] * dv[1]);
    let r2 = dr[0] * dr[0] + dr[1] * dr[1];
    let four = R::count(4);
    k * k * r2 < four * mm * mm
}

/// The squared pair separations `|r₁ − r₂|²`, `|r₂ − r₀|²`, `|r₀ − r₁|²`, by the body each pair leaves out.
#[inline]
fn pair_d2<R: EscapeReal>(r: &[[R; 2]; 3]) -> [R; 3] {
    let d2 = |a: [R; 2], b: [R; 2]| {
        let (x, y) = (a[0] - b[0], a[1] - b[1]);
        x * x + y * y
    };
    [d2(r[1], r[2]), d2(r[2], r[0]), d2(r[0], r[1])]
}

/// prin-rs's naive energy baseline (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnergyFloor<R> {
    verdict: Verdict,
    lag: ShapeLag<R>,
}

/// The energy floor's test on the state `(m, r, p)`: the body left out of the tightest pair (the lowest index of a
/// tie), fired where it is unbound from the pair, a triple ejection where the pair is unbound too.
#[inline]
pub fn energy_floor_test<R: EscapeReal>(m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> Verdict {
    let d2 = pair_d2(r);
    let mut b = 0u32;
    if d2[1] < d2[0] {
        b = 1;
    }
    // Constant indices only (the compile-discipline lint): the tightest pair so far, chosen by value.
    let tightest = if b == 1 { d2[1] } else { d2[0] };
    if d2[2] < tightest {
        b = 2;
    }
    let rel: [Relative<R>; 3] = relative(m, r, p);
    let b0 = (m[0], r[0], p[0]);
    let b1 = (m[1], r[1], p[1]);
    let b2 = (m[2], r[2], p[2]);
    let (outer, pair) = if b == 0 {
        (rel[0], pair_bound(b1, b2))
    } else if b == 1 {
        (rel[1], pair_bound(b2, b0))
    } else {
        (rel[2], pair_bound(b0, b1))
    };
    if outer.unbound() {
        fired(b, !pair)
    } else {
        not_fired()
    }
}

impl<R: EscapeReal> EscapeRule<R> for EnergyFloor<R> {
    #[inline]
    fn start(m: &[R; 3], r: &[[R; 2]; 3], _p: &[[R; 2]; 3], _e_0: R) -> Self {
        EnergyFloor {
            verdict: not_fired(),
            lag: ShapeLag::new(m, r),
        }
    }

    #[inline]
    fn resume(&mut self, n_lag: [R; 3], _e_0: R) {
        self.lag = ShapeLag(n_lag);
    }

    #[inline]
    fn sample(
        &mut self,
        m: &[R; 3],
        r: &[[R; 2]; 3],
        p: &[[R; 2]; 3],
        index: u32,
        quantum: R,
        _i_max: R,
    ) {
        if is_mark(index, quantum) {
            self.verdict = energy_floor_test(m, r, p);
            self.lag.mark(m, r, index, quantum);
        }
    }

    #[inline]
    fn hold(&mut self, detail: u32) {
        self.verdict = stored(detail);
    }

    #[inline]
    fn fired(&self) -> bool {
        self.verdict.fired
    }

    #[inline]
    fn verdict(&self) -> Verdict {
        self.verdict
    }

    #[inline]
    fn lagged(&self) -> [R; 3] {
        self.lag.0
    }
}

/// prin-rs's naive distance baseline (the module's note): the verdict.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistanceFloor<R> {
    verdict: Verdict,
    lag: ShapeLag<R>,
}

/// The distance floor's test on the state `(m, r, p)`: the body farthest from the other two's centre of mass
/// ([`escape::escaper_of`]'s choice), fired where that distance exceeds [`ESCAPE_DISTANCE_FLOOR`]` · R_0`, `R_0 = 1` in
/// the canonical gauge (the module's note), a triple ejection where the other two are that far apart as well.
#[inline]
pub fn distance_floor_test<R: EscapeReal>(m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> Verdict {
    let rel = relative(m, r, p);
    let b = escape::escaper_of(&rel).body;
    let f = R::at_width(ESCAPE_DISTANCE_FLOOR as f32, ESCAPE_DISTANCE_FLOOR);
    let limit = f * f;
    let d2 = pair_d2(r);
    let (far, pair) = if b == 1 {
        (rel[1].d2, d2[1])
    } else if b == 2 {
        (rel[2].d2, d2[2])
    } else {
        (rel[0].d2, d2[0])
    };
    if far > limit {
        fired(b, pair > limit)
    } else {
        not_fired()
    }
}

impl<R: EscapeReal> EscapeRule<R> for DistanceFloor<R> {
    #[inline]
    fn start(m: &[R; 3], r: &[[R; 2]; 3], _p: &[[R; 2]; 3], _e_0: R) -> Self {
        DistanceFloor {
            verdict: not_fired(),
            lag: ShapeLag::new(m, r),
        }
    }

    #[inline]
    fn resume(&mut self, n_lag: [R; 3], _e_0: R) {
        self.lag = ShapeLag(n_lag);
    }

    #[inline]
    fn sample(
        &mut self,
        m: &[R; 3],
        r: &[[R; 2]; 3],
        p: &[[R; 2]; 3],
        index: u32,
        quantum: R,
        _i_max: R,
    ) {
        if is_mark(index, quantum) {
            self.verdict = distance_floor_test(m, r, p);
            self.lag.mark(m, r, index, quantum);
        }
    }

    #[inline]
    fn hold(&mut self, detail: u32) {
        self.verdict = stored(detail);
    }

    #[inline]
    fn fired(&self) -> bool {
        self.verdict.fired
    }

    #[inline]
    fn verdict(&self) -> Verdict {
        self.verdict
    }

    #[inline]
    fn lagged(&self) -> [R; 3] {
        self.lag.0
    }
}

/// prin-rs's Chazy final-motion classifier as an escape rule (the module's note): its verdict and `E_0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chazy<R> {
    verdict: Verdict,
    e_0: R,
    lag: ShapeLag<R>,
}

/// The Chazy rule's test on the state `(m, r, p)` of a run whose `E_0` is `e_0` and whose latch holds `i_max`
/// (the module's note): the classifier at the register's defaults, `R_0 = 1` by the canonical gauge, settling off (no
/// previous snapshot); fired on H, HP, P (a triple ejection) and HE, PE (the escape of the body most separated from its
/// pair), not fired on any other label.
#[inline]
pub fn chazy_test<R: EscapeReal>(
    m: &[R; 3],
    r: &[[R; 2]; 3],
    p: &[[R; 2]; 3],
    e_0: R,
    i_max: R,
) -> Verdict {
    let ph = Playhead::from_momenta(*m, *r, *p, e_0, R::one(), i_max);
    let motion = classify(&ph, &ChazyCfg::defaults(), &Previous::none());
    match motion.coarse() {
        Motion::H => fired(most_separated(&ph) as u32, true),
        Motion::He => fired(most_separated(&ph) as u32, false),
        _ => not_fired(),
    }
}

impl<R: EscapeReal> EscapeRule<R> for Chazy<R> {
    #[inline]
    fn start(m: &[R; 3], r: &[[R; 2]; 3], _p: &[[R; 2]; 3], e_0: R) -> Self {
        Chazy {
            verdict: not_fired(),
            e_0,
            lag: ShapeLag::new(m, r),
        }
    }

    #[inline]
    fn resume(&mut self, n_lag: [R; 3], e_0: R) {
        self.e_0 = e_0;
        self.lag = ShapeLag(n_lag);
    }

    #[inline]
    fn sample(
        &mut self,
        m: &[R; 3],
        r: &[[R; 2]; 3],
        p: &[[R; 2]; 3],
        index: u32,
        quantum: R,
        i_max: R,
    ) {
        if is_mark(index, quantum) {
            self.verdict = chazy_test(m, r, p, self.e_0, i_max);
            self.lag.mark(m, r, index, quantum);
        }
    }

    #[inline]
    fn hold(&mut self, detail: u32) {
        self.verdict = stored(detail);
    }

    #[inline]
    fn fired(&self) -> bool {
        self.verdict.fired
    }

    #[inline]
    fn verdict(&self) -> Verdict {
        self.verdict
    }

    #[inline]
    fn lagged(&self) -> [R; 3] {
        self.lag.0
    }
}

/// The Kepler semimajor axis of the pair of bodies `(mi, ri, vi)`, `(mj, rj, vj)` (`crate::chazy::kepler`), negative
/// where it is unbound.
#[inline]
fn pair_a<R: EscapeReal>(
    (mi, ri, vi): (R, [R; 2], [R; 2]),
    (mj, rj, vj): (R, [R; 2], [R; 2]),
) -> R {
    kepler(
        [rj[0] - ri[0], rj[1] - ri[1]],
        [vj[0] - vi[0], vj[1] - vi[1]],
        mi + mj,
    )
    .a
}

/// The body code of `k`, `0..3`, by constant comparisons.
#[inline]
fn code_of(k: usize) -> u32 {
    if k == 1 {
        DETAIL_ESCAPE_BODY_1
    } else if k == 2 {
        DETAIL_ESCAPE_BODY_2
    } else {
        DETAIL_ESCAPE_BODY_0
    }
}

/// Fewbody's test on the state `(m, r, p)` of a run whose `E_0` is `e_0` (the module's note; validation's
/// `fewbody_resolved`, transcribed): the bound pair of least Kepler `a` (the first in the order `(0, 1)`, `(0, 2)`,
/// `(1, 2)` of a tie); with none, a triple ejection of the body most separated from its pair where every pair recedes
/// and `E_0 > 0`; with one, the escape of its third body `k` where `k` is unbound from it (`K_out ≥ U_out`), recedes
/// and the pair's tidal ratio from `k` is below the register's `δ`. Anything else, B-stable or unresolved, is not
/// fired.
///
/// The plan is unclear here: the reference reads the live energy for H's `E > 0`. Chose the stored `E_0`, its sign
/// decided once, as the closure rule's ionisation gate reads it, because the live energy differs from it only by drift
/// and a label must not flip on integration error; the harness's per-mark cross-check against the reference counts
/// where the two differ.
#[inline]
pub fn fewbody_test<R: EscapeReal>(
    m: &[R; 3],
    r: &[[R; 2]; 3],
    p: &[[R; 2]; 3],
    e_0: R,
) -> Verdict {
    let ph = Playhead::from_momenta(*m, *r, *p, e_0, R::one(), R::one());
    let b0 = (m[0], r[0], ph.v[0]);
    let b1 = (m[1], r[1], ph.v[1]);
    let b2 = (m[2], r[2], ph.v[2]);
    let zero = R::zero();
    // The third body of the least-`a` bound pair, 3 where no pair is bound. Constant indices only.
    let a01 = pair_a(b0, b1);
    let a02 = pair_a(b0, b2);
    let a12 = pair_a(b1, b2);
    let mut k = 3usize;
    let mut least = a01;
    if a01 > zero {
        k = 2;
    }
    if a02 > zero && (k == 3 || a02 < least) {
        k = 1;
        least = a02;
    }
    if a12 > zero && (k == 3 || a12 < least) {
        k = 0;
    }
    if k == 3 {
        let receding = pair_kup(&ph, 0, 1).v_r > zero
            && pair_kup(&ph, 0, 2).v_r > zero
            && pair_kup(&ph, 1, 2).v_r > zero;
        return if receding && e_0 > zero {
            fired(code_of(most_separated(&ph)), true)
        } else {
            not_fired()
        };
    }
    // The outer body's two-body energies and the pair's tidal ratio, by literal indices (the compile-discipline lint).
    let (outer, tidal) = if k == 0 {
        (out_kup(&ph, 0, 1, 2), tidal_ratio(&ph, 1, 2, 0))
    } else if k == 1 {
        (out_kup(&ph, 1, 0, 2), tidal_ratio(&ph, 0, 2, 1))
    } else {
        (out_kup(&ph, 2, 0, 1), tidal_ratio(&ph, 0, 1, 2))
    };
    let delta = R::at_width(CHAZY_DELTA as f32, CHAZY_DELTA);
    if outer.k >= outer.u && outer.v_r > zero && tidal < delta {
        fired(code_of(k), false)
    } else {
        not_fired()
    }
}

/// Fewbody's termination test as an escape rule (the module's note): its verdict and `E_0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fewbody<R> {
    verdict: Verdict,
    e_0: R,
    lag: ShapeLag<R>,
}

impl<R: EscapeReal> EscapeRule<R> for Fewbody<R> {
    #[inline]
    fn start(m: &[R; 3], r: &[[R; 2]; 3], _p: &[[R; 2]; 3], e_0: R) -> Self {
        Fewbody {
            verdict: not_fired(),
            e_0,
            lag: ShapeLag::new(m, r),
        }
    }

    #[inline]
    fn resume(&mut self, n_lag: [R; 3], e_0: R) {
        self.e_0 = e_0;
        self.lag = ShapeLag(n_lag);
    }

    #[inline]
    fn sample(
        &mut self,
        m: &[R; 3],
        r: &[[R; 2]; 3],
        p: &[[R; 2]; 3],
        index: u32,
        quantum: R,
        _i_max: R,
    ) {
        if is_mark(index, quantum) {
            self.verdict = fewbody_test(m, r, p, self.e_0);
            self.lag.mark(m, r, index, quantum);
        }
    }

    #[inline]
    fn hold(&mut self, detail: u32) {
        self.verdict = stored(detail);
    }

    #[inline]
    fn fired(&self) -> bool {
        self.verdict.fired
    }

    #[inline]
    fn verdict(&self) -> Verdict {
        self.verdict
    }

    #[inline]
    fn lagged(&self) -> [R; 3] {
        self.lag.0
    }
}
