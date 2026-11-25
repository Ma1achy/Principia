//! The escape detector (integrator_contract Part 7; dd_integrator §3.6; pitfalls §2): closure and `E_rel`, the window,
//! the escaper and ionisation.
//!
//! **The rule, two conditions** (Part 7): `ESCAPE ⟺ |Δn̂| over the window < tau AND E_rel > 0`. The shape point has
//! settled and the farthest body is unbound. There is no receding test, no `r_esc` and no persistence counter: the detector reads
//! the state and the lagged shape point, nothing else ([`Detector::sample`]'s inputs).
//!
//! **`E_rel`** ([`relative`], [`e_rel`]): for a candidate body `b`, `E_rel = ½|Δv|² − (M_pair + m_b)/d`, with `Δv` and
//! `d` the velocity and distance of `b` relative to the centre of mass of the other two and `M_pair` their mass
//! (`G = 1`). It takes the **total** mass; the pair's mass alone biases toward escape. Every term is a difference of
//! positions or of velocities, so it is the same before and after the centre-of-mass projection.
//!
//! **The window** ([`window`], [`is_mark`]): `|Δn̂|` is the chord between the shape point now and the one held from one
//! window, 0.4 time units, earlier. The occupant's sampling points are the macro-step boundaries for an unregularised
//! occupant and the sync boundaries for a regularised one, each `index · quantum` in physical time. The lagged point is
//! one register, [`Detector::lagged`]: set to `n̂(0)` at `t = 0`, and at the first sampling point at or after each
//! `k · 0.4` the test reads `|n̂ − n̂_lag|` and the register takes `n̂`. So the window spans from one mark's sampling
//! point to the next's, within one quantum of 0.4. "At or after" is decided on the two float products, `k · 0.4` and
//! `index · quantum`: where the quantum is 0.4 over a power of two the products are equal and the window is 0.4
//! exactly, and where they are equal as decimals but not as floats (a quantum of 0.03 at `t = 1.2`) the mark can fall
//! one sampling point late. Whether a
//! sampling point is such a mark is a function of its index and the quantum alone ([`is_mark`]), so a resumed march
//! needs only the register (`SimState`'s `_nhat_lag_*`, payload §1) to go on as the uncut one. The settling test reads
//! this register, never `closure_min`, which is a minimum against the IC's shape and a different quantity.
//!
//! **The escaper** ([`escaper`]): the body with the largest `d`, the same `d` as in its `E_rel` and the lowest index
//! of a tie, and it is a candidate only where its own `E_rel > 0`. It is the escape's `detail`. The plan is unclear
//! here: "the body with `E_rel > 0` and the largest `d`" also reads as the farthest of the bodies that have
//! `E_rel > 0`. Chose the farthest body, tested on its own `E_rel`, because under the other reading the energy
//! condition does not hold back a bound hierarchy: a member of the inner binary moves at the binary's orbital speed
//! relative to the centre of mass of the other two, which lies about half way to the far body, so its `E_rel` turns
//! positive with the binary's phase once the third body is a few binary separations away, bound or not (measured on
//! `fixtures/escape/settled_bound_hierarchy.toml` at `t = 0`: `+0.37` for one binary member, `−0.40` for the other,
//! `−0.15` for the third body). A body that escapes from a pair of comparable masses becomes the farthest, so there
//! this reading can only fire later. Neither reading is exact, because the rule treats "the other two" as one mass
//! whether or not they are the bound pair: where the inner pair is light and slow beside a heavy third body, a member
//! of the pair can be the farthest body with `E_rel > 0` while every pair is bound, and the rule fires on a bound
//! system (measured, and recorded in `plan/KNOWN.md`: masses `(0.001, 0.001, 1)`, the pair's separation 1, the heavy
//! body at 400 on a circular orbit, total `E < 0`; the window chord is `4.5·10⁻⁵` and `E_rel = +1.2·10⁻⁴` for the far
//! member, so it fires at the first mark).
//!
//! **Triple ejection** ([`ionised`]) is a classification of a firing the rule has already made, never a second way to
//! fire: where all three pairwise two-body energies are positive, all three separations are growing and the total
//! energy is positive, the escape's `detail` is 3. The total energy is read from the stored `E_0`, its sign decided
//! once per trajectory at `t = 0`: the live `E` differs from it only by drift, and a label must not flip on
//! integration error.
//!
//! **The decisions' forms.** Each is one small function ([`settled`], [`Relative::unbound`], [`pair_unbound`]), with no
//! `√` and no division at the comparison: the chord is compared squared, and an energy's sign is decided multiplied
//! through, `|Δv|⁴·d² > 4·(M_pair + m_b)²` for `E_rel > 0` (Part 4), both sides being positive. The quantities under
//! them still hold divisions (the shape point's by `I`, a velocity's by its mass), and a chord that small can square to
//! a subnormal, which changes no decision against `tau²` but is not the written form rule 6 asks for. The plan is
//! unclear here (Part 4 leaves the forms to later work); chose these because
//! they are the forms Part 4 already writes, and kept each decision in its own function for that task to replace. A
//! value that is not finite fires nothing: the chord is tested on its bits, and a NaN energy fails its comparison.
//!
//! **After it fires** the label is live by default: the next mark re-reads it, and the label written when the loop ends
//! is the last mark's ([`crate::detect::label`]). With the cadence's stop option on
//! (`crate::driver::cadence::WrapperCadence`), the first firing sets the run's `done` flag and stays.

use spirv_std::num_traits::Float;

use crate::detect::label::Label;
use crate::driver::advance::{Count, Width};
use crate::driver::welford::chord2;
use crate::generated::constants::{ESCAPE_WINDOW, TAU};
use crate::payload::{
    DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION, DETAIL_ESCAPE_BODY_0, DETAIL_ESCAPE_BODY_1,
    DETAIL_ESCAPE_BODY_2, STATE_ESCAPE,
};
use crate::real::IeeeBits;
use crate::shape::shape;
use crate::Real;

/// The provisional settling threshold `tau`, `10⁻³`, a chord on the unit shape sphere, so a pure number: the
/// register's `tau`, which is prin-rs's `CLOSURE_TAU` (`src/outcome.rs:263` at `8600d45`), inside the recorded gap
/// between escapers and bound trajectories, `7.04·10⁻⁵ … 2.70·10⁻²`. Provisional until the gap is re-measured with
/// this `E_rel` and the confirmed value replaces it.
pub const TAU_PROVISIONAL: f64 = TAU;

/// The float type the detector runs at: the shape map's, with a sampling index converted and compared ([`Count`]), a
/// register value read at its width ([`Width`]) and a value's bits read ([`IeeeBits`]). f32 and f64.
pub trait EscapeReal: Real + Float + Count + Width + IeeeBits {}

impl<R: Real + Float + Count + Width + IeeeBits> EscapeReal for R {}

/// `tau` at the width `R` ([`TAU_PROVISIONAL`]).
#[inline]
pub fn tau<R: Width>() -> R {
    R::at_width(TAU_PROVISIONAL as f32, TAU_PROVISIONAL)
}

/// The window `|Δn̂|` is taken over, 0.4 time units, at the width `R` (the register's `escape_window`; provisional).
#[inline]
pub fn window<R: Width>() -> R {
    R::at_width(ESCAPE_WINDOW as f32, ESCAPE_WINDOW)
}

/// The window marks at or before the sampling point `index` of a grid of `quantum`: the largest `k` with
/// `k · window ≤ index · quantum`, each side one product, decided by comparing those products (the quotient only
/// proposes `k`, as `crate::regularisation::az::boundary_at_or_before` does).
#[inline]
pub fn marks_at_or_before<R: EscapeReal>(index: u32, quantum: R) -> u32 {
    let w = window::<R>();
    let t = R::count(index) * quantum;
    let mut k = (t / w).whole();
    if k > 0 && R::count(k) * w > t {
        k -= 1;
    }
    if k < u32::MAX && R::count(k + 1) * w <= t {
        k += 1;
    }
    k
}

/// Whether the sampling point `index` of a grid of `quantum` is the first at or after a window mark `k · 0.4`
/// (the module's note): it has passed a mark its predecessor had not. A function of the index and the quantum alone.
/// The point at `t = 0` is none: the register is set there, not read.
#[inline]
pub fn is_mark<R: EscapeReal>(index: u32, quantum: R) -> bool {
    index > 0 && marks_at_or_before(index, quantum) > marks_at_or_before(index - 1, quantum)
}

/// Whether the shape point has settled over the window: the squared chord between `n` and the lagged `n_lag` is
/// finite, tested on its bits, and below `tau²`.
#[inline]
pub fn settled<R: EscapeReal>(n: [R; 3], n_lag: [R; 3]) -> bool {
    let c2 = chord2(n, n_lag);
    let t = tau::<R>();
    !c2.non_finite_bits() && c2 < t * t
}

/// One body's two-body relation to the other two (the module's note): its squared speed and squared distance relative
/// to their centre of mass, and the total mass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Relative<R> {
    /// `|Δv|²`.
    pub v2: R,
    /// `d²`.
    pub d2: R,
    /// `M_pair + m_b`.
    pub mass: R,
}

impl<R: EscapeReal> Relative<R> {
    /// `E_rel = ½|Δv|² − (M_pair + m_b)/d`, the value: for a report, never the decision.
    #[inline]
    pub fn e_rel(&self) -> R {
        let two = R::one() + R::one();
        self.v2 / two - self.mass / Float::sqrt(self.d2)
    }

    /// Whether `E_rel > 0`, decided as `|Δv|⁴·d² > 4·(M_pair + m_b)²` (the module's note).
    #[inline]
    pub fn unbound(&self) -> bool {
        let four = R::count(4);
        self.v2 * self.v2 * self.d2 > four * self.mass * self.mass
    }
}

/// The relation of the body of mass `mb` at `rb` with momentum `pb` to the two others, `(mi, ri, pi)` and
/// `(mj, rj, pj)`.
#[inline]
fn relative_of<R: EscapeReal>(
    (mb, rb, pb): (R, [R; 2], [R; 2]),
    (mi, ri, pi): (R, [R; 2], [R; 2]),
    (mj, rj, pj): (R, [R; 2], [R; 2]),
) -> Relative<R> {
    let pair = mi + mj;
    let dr = [
        rb[0] - (mi * ri[0] + mj * rj[0]) / pair,
        rb[1] - (mi * ri[1] + mj * rj[1]) / pair,
    ];
    let dv = [
        pb[0] / mb - (pi[0] + pj[0]) / pair,
        pb[1] / mb - (pi[1] + pj[1]) / pair,
    ];
    Relative {
        v2: dv[0] * dv[0] + dv[1] * dv[1],
        d2: dr[0] * dr[0] + dr[1] * dr[1],
        mass: pair + mb,
    }
}

/// Each body's relation to the other two, by body index, at masses `m`, positions `r` and momenta `p`. Constant
/// indices only (gpu_determinism_note rule 5).
#[inline]
pub fn relative<R: EscapeReal>(m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> [Relative<R>; 3] {
    let b0 = (m[0], r[0], p[0]);
    let b1 = (m[1], r[1], p[1]);
    let b2 = (m[2], r[2], p[2]);
    [
        relative_of(b0, b1, b2),
        relative_of(b1, b2, b0),
        relative_of(b2, b0, b1),
    ]
}

/// Each body's `E_rel` about the centre of mass of the other two, with the total mass (the module's note).
#[inline]
pub fn e_rel<R: EscapeReal>(m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> [R; 3] {
    let [a, b, c] = relative(m, r, p);
    [a.e_rel(), b.e_rel(), c.e_rel()]
}

/// The candidate escaper: the farthest body, and whether it is unbound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Escaper {
    /// Whether the farthest body has `E_rel > 0`.
    pub found: bool,
    /// The body with the largest `d`, the lowest index of a tie.
    pub body: u32,
}

/// The escaper among the bodies' relations `rel` (the module's note): the body whose `d²` is largest, a later body
/// replacing an earlier only where strictly farther, found where its own `E_rel > 0`. Constant indices only.
#[inline]
pub fn escaper_of<R: EscapeReal>(rel: &[Relative<R>; 3]) -> Escaper {
    let [a, b, c] = *rel;
    let (mut body, mut far) = (DETAIL_ESCAPE_BODY_0, a);
    if b.d2 > far.d2 {
        body = DETAIL_ESCAPE_BODY_1;
        far = b;
    }
    if c.d2 > far.d2 {
        body = DETAIL_ESCAPE_BODY_2;
        far = c;
    }
    Escaper {
        found: far.unbound(),
        body,
    }
}

/// The escaper of the state `(m, r, p)`: [`escaper_of`] its bodies' [`relative`] relations.
#[inline]
pub fn escaper<R: EscapeReal>(m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> Escaper {
    escaper_of(&relative(m, r, p))
}

/// Whether the pair `(mi, ri, pi)`, `(mj, rj, pj)` is unbound and separating: its two-body energy
/// `½μ|Δv|² − mᵢmⱼ/r` is positive, decided as `(μ|Δv|²)²·r² > 4·(mᵢmⱼ)²` (the module's note), and `Δr·Δv > 0`.
#[inline]
pub fn pair_unbound<R: EscapeReal>(
    (mi, ri, pi): (R, [R; 2], [R; 2]),
    (mj, rj, pj): (R, [R; 2], [R; 2]),
) -> bool {
    let dr = [rj[0] - ri[0], rj[1] - ri[1]];
    let dv = [pj[0] / mj - pi[0] / mi, pj[1] / mj - pi[1] / mi];
    let mm = mi * mj;
    let k = mm / (mi + mj) * (dv[0] * dv[0] + dv[1] * dv[1]);
    let r2 = dr[0] * dr[0] + dr[1] * dr[1];
    let four = R::count(4);
    k * k * r2 > four * mm * mm && dr[0] * dv[0] + dr[1] * dv[1] > R::zero()
}

/// The ionisation gate's predicate on the state `(m, r, p)` of a trajectory whose stored initial energy is `e_0` (the
/// module's note): all three pairs unbound and separating ([`pair_unbound`]), and `E_0 > 0`. It reads no `E_rel`, no
/// shape point and no live total energy; the detector asks it only of an escape that has fired.
#[inline]
pub fn ionised<R: EscapeReal>(m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3], e_0: R) -> bool {
    let b0 = (m[0], r[0], p[0]);
    let b1 = (m[1], r[1], p[1]);
    let b2 = (m[2], r[2], p[2]);
    e_0 > R::zero() && pair_unbound(b1, b2) && pair_unbound(b2, b0) && pair_unbound(b0, b1)
}

/// An escape test's verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verdict {
    /// Whether escape has fired.
    pub fired: bool,
    /// The escaper's body index; where not fired, 0, which nothing reads.
    pub escaper: u32,
    /// The escape's `detail`: the escaper's body index, or 3, a triple ejection; where not fired, 0, which nothing
    /// reads.
    pub detail: u32,
}

/// The escape test on the state `(m, r, p)` against the lagged shape point `n_lag`, for a trajectory whose stored
/// initial energy is `e_0` (the module's note): fired where the shape has settled and the farthest body is unbound,
/// its escaper that body, its `detail` 3 where the ionisation gate holds and the escaper otherwise.
#[inline]
pub fn test<R: EscapeReal>(
    m: &[R; 3],
    r: &[[R; 2]; 3],
    p: &[[R; 2]; 3],
    n_lag: [R; 3],
    e_0: R,
) -> Verdict {
    let candidate = escaper(m, r, p);
    let fired = settled(shape(*r, *m), n_lag) && candidate.found;
    let detail = if fired && ionised(m, r, p, e_0) {
        DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION
    } else {
        candidate.body
    };
    Verdict {
        fired,
        escaper: candidate.body,
        detail,
    }
}

/// The run's escape detector (the module's note): the lagged shape point, the stored `E_0` and the verdict, which
/// stays once fired.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detector<R> {
    n_lag: [R; 3],
    e_0: R,
    verdict: Verdict,
}

impl<R: EscapeReal> Detector<R> {
    /// The detector of a run from masses `m` and positions `r` whose initial energy is `e_0`: the register set to
    /// `n̂(0)`, nothing fired. There is no escape at `t = 0`: the rule needs a window of history.
    #[inline]
    pub fn new(m: &[R; 3], r: &[[R; 2]; 3], e_0: R) -> Self {
        Self::resumed(shape(*r, *m), e_0)
    }

    /// The detector of a march taken up again, still running: the register as stored, `n_lag`, and the stored `E_0`.
    #[inline]
    pub fn resumed(n_lag: [R; 3], e_0: R) -> Self {
        Detector {
            n_lag,
            e_0,
            verdict: Verdict {
                fired: false,
                escaper: DETAIL_ESCAPE_BODY_0,
                detail: DETAIL_ESCAPE_BODY_0,
            },
        }
    }

    /// The occupant's sampling point `index` on its grid of `quantum`, physical time `index · quantum`, at the state
    /// `(m, r, p)`: where it is a window mark ([`is_mark`]), the escape test against the register ([`test`]), the label
    /// until the next mark, fired or not (the label is live: `crate::detect::escape_rule`'s note), and then the register
    /// takes the state's shape point. Any other point is not read. A march that stops at the first label stops calling
    /// this once it has fired (`crate::driver::cadence::WrapperCadence`).
    #[inline]
    pub fn sample(&mut self, m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3], index: u32, quantum: R) {
        if is_mark(index, quantum) {
            self.verdict = test(m, r, p, self.n_lag, self.e_0);
            self.n_lag = shape(*r, *m);
        }
    }

    /// Takes up a label a re-booted march stored, `verdict`, until the next mark re-reads it.
    #[inline]
    pub fn hold(&mut self, verdict: Verdict) {
        self.verdict = verdict;
    }

    /// Whether the label is an escape at this playhead; under the stop option it sets the run's `done` flag.
    #[inline]
    pub fn fired(&self) -> bool {
        self.verdict.fired
    }

    /// The verdict.
    #[inline]
    pub fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// The lagged shape point, the register `SimState` stores (`_nhat_lag_0`, `_1`, `_2`).
    #[inline]
    pub fn lagged(&self) -> [R; 3] {
        self.n_lag
    }

    /// `packed_a` with the escape written through the generated descriptor setters where it fired, `state` `escape`
    /// and its `detail`; every other bit kept, and `packed_a` as it was where it did not.
    #[inline]
    pub fn label(&self, packed_a: u32) -> u32 {
        if self.verdict.fired {
            Label {
                state: STATE_ESCAPE,
                detail: self.verdict.detail,
            }
            .write(packed_a)
        } else {
            packed_a
        }
    }
}
