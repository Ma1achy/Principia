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
//! - [`CrossingDetector`] carries the last accepted endpoint's distances between substeps, so each endpoint is shared
//!   by the two substeps that meet there and an endpoint on a cut is counted once.
//!
//! The distances are the input: which cuts carry `a` and `b`, and so how a state's distances are formed, is RQ-630
//! (symbolic_dynamics_contract §1). Nothing here depends on the cuts' shape.

use super::binding::{on_substep, CrossingHook, SubstepCrossings, WordBuffer};
use crate::payload::inverse;
use crate::real::IeeeBits;

/// Each cut's signed distance at one endpoint of an accepted substep: cut `a`, ID 0, and cut `b`, ID 1
/// (symbolic_dynamics_contract §1). Signed so that the positive sense, negative → non-negative, is the generator.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CutDistances<T> {
    /// The signed distance to cut `a`.
    pub a: T,
    /// The signed distance to cut `b`.
    pub b: T,
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

#[inline]
fn cut_crossing<T: IeeeBits>(d0: T, d1: T, generator: u32) -> CutCrossing {
    let from_negative = negative(d0);
    CutCrossing {
        crossed: from_negative != negative(d1),
        symbol: if from_negative {
            generator
        } else {
            inverse(generator)
        },
    }
}

/// The crossings of one accepted substep, from the cuts' distances at its start to those at its end, in the order they
/// are appended: by ascending `τ`, `a` first on a tie (integrator contract Part 5). `a`'s generator is symbol 0 and
/// `b`'s symbol 2 (payload §3's codes, `a = 0, A = 1, b = 2, B = 3`). `τ` is never formed: `b` precedes `a` iff
/// `|d₀,b|·|d₁,a| < |d₀,a|·|d₁,b|`, [`product_less`].
#[inline]
pub fn substep_crossings<T: IeeeBits>(
    start: CutDistances<T>,
    end: CutDistances<T>,
) -> SubstepCrossings {
    let a = cut_crossing(start.a, end.a, 0);
    let b = cut_crossing(start.b, end.b, 2);
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
