//! Branch-cut crossing detection, run at every accepted substep (integrator contract Part 5, the branch-cut bullet and
//! its τ tie rule, REQ-INT-079; dd_simstate_payload §3): each cut's signed distance at the substep's two endpoints, the
//! half-open count, the order by crossing fraction `τ` with the cut ID as the tie-break, and the crossings, in that
//! order, appended to the word through [`super::on_substep`].
//!
//! - A cut's endpoint is on its *negative* side iff it is below zero and not read as zero; `±0` and subnormals read as
//!   zero, on the cut ([`negative`]). A cut is crossed iff its two endpoints' sides differ: negative → non-negative is
//!   the positive sense, the cut's generator (`a`, `b`), and non-negative → negative its inverse (`A`, `B`).
//! - With both cuts crossed, `τ_b < τ_a` is decided as `|d₀,b|·|d₁,a| < |d₀,a|·|d₁,b|` ([`product_less`]), never by
//!   computing `τ`: no division, no addition, no subnormal intermediate, one correctly rounded multiply. On a tie, `a`
//!   first.
//! - Each cut carries a segment gate as well as its distance: a side change is a crossing of the cut only where its
//!   gate is open at the substep's non-negative endpoint (integrator contract Part 5, the segment gate).
//! - [`CrossingDetector`] carries the last accepted endpoint's distances between substeps, so each endpoint is shared
//!   by the two substeps that meet there and an endpoint on a cut is counted once.
//!
//! **The cuts** (symbolic_dynamics_contract §1; RQ-632, option (a)): the collision equator's segments with body 0
//! between (`a`, from `P₀₁` to `P₀₂`) and with body 2 between (`b`, from `P₁₂` to `P₀₂`). Both share one signed
//! function, `d = −((r₁ − r₀) × (r₂ − r₀))`, negative on `L⁺`'s hemisphere, so a generator crosses from `L⁺`'s
//! hemisphere into `L⁻`'s; the gates are §1's segment classifier, [`between`]: cut `a`'s is open where body 0 is
//! between, cut `b`'s where body 2 is, neither where body 1 is. [`state_cuts`] forms the two from the positions.
//! The gates being exclusive, one substep crosses at most one of these cuts, so the `τ`-sort never meets two of them;
//! it stays defined for cuts with distinct distances.
//!
//! **The branch inputs** (R-547; integrator contract Part 4). `d` and the three squared separations the classifier
//! compares are formed as `crate::canon` forms its branch inputs: each coordinate taken in as a mantissa in `[1, 2)`
//! and an integer exponent, the differences, the products ([`IeeeBits::product`]'s pattern) and the sums formed on
//! those, so no intermediate is subnormal and every IEEE backend rounds them alike. The comparisons are on those
//! pairs. `d` is decided on the pair's exponent before it is converted: below the normal range it is zero, and in the
//! range the conversion is exact, so no subnormal is formed and no conversion rounds up across the boundary
//! ([`crate::canon::to_normal_float`]). Its side is then read by [`negative`], which reads a subnormal handed to it as
//! zero, so a distance from elsewhere is read as a backend that flushes it reads it.

use super::binding::{on_substep, CrossingHook, SubstepCrossings, WordBuffer};
use crate::canon::{
    difference, to_normal_float, vector, wide_greater, wide_mul, wide_norm2, wide_sub,
};
use crate::generated::links::LinkReal;
use crate::payload::inverse;
use crate::real::IeeeBits;

/// Each cut's signed distance and segment gate at one endpoint of an accepted substep: cut `a`, ID 0, and cut `b`,
/// ID 1 (symbolic_dynamics_contract §1). Signed so that the positive sense, negative → non-negative, is the generator.
/// A side change of a cut's distance is a crossing of it only where its gate is open at the substep's non-negative
/// endpoint (integrator contract Part 5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CutDistances<T> {
    /// The signed distance to cut `a`.
    pub a: T,
    /// The signed distance to cut `b`.
    pub b: T,
    /// Cut `a`'s segment gate at this endpoint.
    pub gate_a: bool,
    /// Cut `b`'s segment gate at this endpoint.
    pub gate_b: bool,
}

impl<T> CutDistances<T> {
    /// Two cuts with distances of their own and no segment between them to choose: both gates open.
    #[inline]
    pub const fn ungated(a: T, b: T) -> Self {
        Self {
            a,
            b,
            gate_a: true,
            gate_b: true,
        }
    }
}

/// Whether `d` reads as zero: its exponent field is all zeros, so it is `±0` or a subnormal, read as a backend that
/// flushes subnormals reads it. A test on the bits, against zero's own exponent, so no precision's constant enters it.
#[inline]
pub fn reads_as_zero<T: IeeeBits>(d: T) -> bool {
    d.exponent() == T::default().exponent()
}

/// Whether `d` is on its cut's negative side: below zero and not read as zero ([`reads_as_zero`]). Everything else,
/// the cut itself included, is the non-negative side (the half-open rule).
#[inline]
pub fn negative<T: IeeeBits>(d: T) -> bool {
    !reads_as_zero(d) && d < T::default()
}

/// `|x|·|y|`, `x` and `y` normal, as the pair (exponent, significand in `[1, 2)`): the significands multiplied once, a
/// correctly rounded multiply in `[1, 4)`, its own exponent added to the factors'. No intermediate can be subnormal.
#[inline]
fn product_key<T: IeeeBits>(x: T, y: T) -> (i32, T) {
    let m = x.significand() * y.significand();
    (x.exponent() + y.exponent() + m.exponent(), m.significand())
}

/// Whether `|x₁|·|y₁| < |x₂|·|y₂|`, comparison-only and the same on every IEEE backend (integrator contract Part 5):
/// a factor that reads as zero ([`reads_as_zero`]) makes its product zero; otherwise each product is the pair
/// (exponent, significand in `[1, 2)`), its significands multiplied once, and the pairs are compared exponent first.
/// Equal pairs are a tie: the tolerance is the products' own rounding. The operands are finite.
#[inline]
pub fn product_less<T: IeeeBits>(x1: T, y1: T, x2: T, y2: T) -> bool {
    let zero1 = reads_as_zero(x1) || reads_as_zero(y1);
    let zero2 = reads_as_zero(x2) || reads_as_zero(y2);
    if zero1 || zero2 {
        zero1 && !zero2
    } else {
        let (e1, s1) = product_key(x1, y1);
        let (e2, s2) = product_key(x2, y2);
        e1 < e2 || (e1 == e2 && s1 < s2)
    }
}

/// One cut over one segment: whether it is crossed, and the crossing's symbol, the cut's `generator` in the positive
/// sense, its inverse in the negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CutCrossing {
    crossed: bool,
    symbol: u32,
}

/// The cut with distances `d0`, `d1` and gates `gate0`, `gate1` at the segment's start and end: crossed iff its sides
/// differ and its gate is open at the non-negative endpoint, the end in the positive sense, the start in the negative.
#[inline]
fn cut_crossing<T: IeeeBits>(
    d0: T,
    d1: T,
    gate0: bool,
    gate1: bool,
    generator: u32,
) -> CutCrossing {
    let from_negative = negative(d0);
    let gate = if from_negative { gate1 } else { gate0 };
    CutCrossing {
        crossed: (from_negative != negative(d1)) & gate,
        symbol: if from_negative {
            generator
        } else {
            inverse(generator)
        },
    }
}

/// The crossings of one accepted substep, from the cuts' distances and gates at its start to those at its end, in the
/// order they are appended: by ascending `τ`, `a` first on a tie (integrator contract Part 5). `a`'s generator is
/// symbol 0 and `b`'s symbol 2 (payload §3's codes, `a = 0, A = 1, b = 2, B = 3`). `τ` is never formed: `b` precedes
/// `a` iff `|d₀,b|·|d₁,a| < |d₀,a|·|d₁,b|`, [`product_less`].
#[inline]
pub fn substep_crossings<T: IeeeBits>(
    start: CutDistances<T>,
    end: CutDistances<T>,
) -> SubstepCrossings {
    let a = cut_crossing(start.a, end.a, start.gate_a, end.gate_a, 0);
    let b = cut_crossing(start.b, end.b, start.gate_b, end.gate_b, 2);
    let b_first = a.crossed && b.crossed && product_less(start.b, end.a, start.a, end.b);
    let (first, second) = if b_first { (b, a) } else { (a, b) };
    let mut out = SubstepCrossings::default();
    if first.crossed {
        out.symbols[0] = first.symbol;
        out.count = 1;
    }
    if second.crossed {
        if first.crossed {
            out.symbols[1] = second.symbol;
        } else {
            out.symbols[0] = second.symbol;
        }
        out.count += 1;
    }
    out
}

/// Which body lies between the other two at positions `r`, §1's segment classifier (REQ-PAY-113): the body opposite the
/// longest side, the `k` whose pair's squared separation `r²ₖ` is the greatest (pair `k` the side opposite body `k`,
/// R-22), the lowest `k` on a tie. On the collision equator it is the body between the other two. The `r²ₖ` are formed
/// and compared as the module's note says, so the choice is the same on every IEEE backend given the same `r`.
#[inline]
pub fn between<R: LinkReal + IeeeBits>(r: &[[R; 2]; 3]) -> u32 {
    let [p0, p1, p2] = [vector(r[0]), vector(r[1]), vector(r[2])];
    let n = [
        wide_norm2(difference(p1, p2)),
        wide_norm2(difference(p2, p0)),
        wide_norm2(difference(p0, p1)),
    ];
    if wide_greater(n[1], n[0]) {
        if wide_greater(n[2], n[1]) {
            2
        } else {
            1
        }
    } else if wide_greater(n[2], n[0]) {
        2
    } else {
        0
    }
}

/// The cuts' shared signed function at positions `r` (symbolic_dynamics_contract §1): `d = −((r₁ − r₀) × (r₂ − r₀))`,
/// minus the triangle 0 → 1 → 2's doubled signed area, formed as the module's note says, `(r₁ − r₀)_y·(r₂ − r₀)_x −
/// (r₁ − r₀)_x·(r₂ − r₀)_y` in one rounding of each product and one of the difference, then taken to the float only
/// where that exponent is in the normal range, zero below it ([`to_normal_float`]).
/// Negative on `L⁺`'s hemisphere (0 → 1 → 2 anticlockwise), positive on `L⁻`'s, zero on the equator.
#[inline]
pub fn signed_distance<R: LinkReal + IeeeBits>(r: &[[R; 2]; 3]) -> R {
    let p0 = vector(r[0]);
    let u = difference(p0, vector(r[1]));
    let w = difference(p0, vector(r[2]));
    to_normal_float(wide_sub(wide_mul(u[1], w[0]), wide_mul(u[0], w[1])))
}

/// The cuts at positions `r` (symbolic_dynamics_contract §1): both distances [`signed_distance`], cut `a`'s gate open
/// where body 0 is [`between`], cut `b`'s where body 2 is.
#[inline]
pub fn state_cuts<R: LinkReal + IeeeBits>(r: &[[R; 2]; 3]) -> CutDistances<R> {
    let d = signed_distance(r);
    let k = between(r);
    CutDistances {
        a: d,
        b: d,
        gate_a: k == 0,
        gate_b: k == 2,
    }
}

/// What the march carries for detection between accepted substeps: the cuts' distances at the last accepted endpoint.
/// Each accepted substep is tested against it and then replaces it, so detection runs at every accepted substep, never
/// once per macro-step, and each endpoint is shared by the two substeps that meet there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossingDetector<T> {
    /// The cuts' distances at the last accepted endpoint.
    pub last: CutDistances<T>,
}

impl<T: IeeeBits> CrossingDetector<T> {
    /// The detector of a march that starts at distances `start`.
    #[inline]
    pub const fn new(start: CutDistances<T>) -> Self {
        Self { last: start }
    }

    /// An accepted substep ending at distances `end`: its crossings, in order ([`substep_crossings`]).
    #[inline]
    pub fn accept(&mut self, end: CutDistances<T>) -> SubstepCrossings {
        let crossings = substep_crossings(self.last, end);
        self.last = end;
        crossings
    }

    /// An accepted substep ending at distances `end`, its crossings appended to the sample's word in order through
    /// [`on_substep`]: with none, the word buffer is neither read nor written. Returns `packed_a` as the last crossing
    /// left it.
    #[inline]
    pub fn accept_into<B: WordBuffer + ?Sized>(
        &mut self,
        end: CutDistances<T>,
        hook: &mut CrossingHook,
        words: &mut B,
        packed_a: u32,
    ) -> u32 {
        let crossings = self.accept(end);
        on_substep(hook, words, packed_a, crossings)
    }
}
