//! The Burrau family's discrete side (chart_reference §4.1, §4.2; REQ-CHART-021, REQ-SYS-016): Euclid's
//! parametrisation of the primitive Pythagorean triples, the Burrau convention's masses, positions and Jacobi vectors
//! for a right triangle of sides `(a, b, c)`, and the translation of the family's 1-based body numbers into the
//! project's 0-based indices (R-22).
//!
//! **Euclid's parametrisation** ([`euclid`]; §4.1): for coprime `m > n > 0` with `m − n` odd,
//!
//! ```text
//! a = m² − n²          b = 2mn          c = m² + n²
//! ```
//!
//! and every primitive triple arises exactly once, up to the leg swap. A non-primitive triple `(ka, kb, kc)` is the
//! same system under the similarity symmetry, so [`euclid`] gives a triple only at a primitive `(m, n)`
//! ([`primitive`]).
//!
//! **The convention in use is the normalised one** ([`CONVENTION`], [`Convention::Normalised`]; §4.2): the right
//! angle at the origin, the sides normalised by the hypotenuse, and each mass equal to its opposite side over the
//! perimeter, so `Σmᵢ = 1`:
//!
//! ```text
//! r₀ = (0, 0)        m₀ = c/(a+b+c)
//! r₁ = (a/c, 0)      m₁ = b/(a+b+c)
//! r₂ = (0, b/c)      m₂ = a/(a+b+c)
//! ρ = (a/c, 0)       λ = (−ab/(c(b+c)), b/c)        the Jacobi vectors of the inner pair (0, 1)
//! ```
//!
//! At `(m, n) = (2, 1)` that is the classical Burrau configuration, the triple `(3, 4, 5)` with masses `(5, 4, 3)/12`.
//! The historical convention, masses `(3, 4, 5)` on a triangle of sides `(3, 4, 5)` ([`Convention::Historical`]), is
//! the same system up to the scale gauge and the unit of mass, but not the same numbers. [`System::of_sides`] holds
//! the formulae once: the triple ([`Triple::burrau`]) and the continuous `ν` axis (`crate::chart::burrau::embed`)
//! both evaluate them.
//!
//! **Body indices are 0-based** (R-22; [`from_literature`], [`pair_id`]). The family is written 1-based in its
//! literature form (bodies 1–3, inner pair 1–2; chart_reference's indexing note), and that numbering is translated
//! here, at the boundary: body `k` of that form is body `k − 1`, so its inner pair `1–2` is the pair `(0, 1)`, pair id
//! 2, pair id `k` naming the side opposite body `k` (payload §2's pair-id map). Nothing in this module shows a body
//! number to a person; what a person reads goes through the one display function (R-449, REQ-GUI-188), whose numbers
//! are not the usual literature's either (RQ-335).

use spirv_std::num_traits::{Float, NumCast};

/// Which Burrau convention a system is stated in (chart_reference §4.2: "State which is in use").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convention {
    /// Right angle at the origin, sides normalised by the hypotenuse, each mass its opposite side over the perimeter:
    /// `Σmᵢ = 1`, the classical point's masses `(5, 4, 3)/12`.
    Normalised,
    /// Masses equal to the opposite sides, `(3, 4, 5)` at the classical point, on the unnormalised triangle.
    Historical,
}

/// The convention every Burrau system in this crate is stated in: the normalised one.
pub const CONVENTION: Convention = Convention::Normalised;

/// What [`CONVENTION`] says, in words, for a chart's label: no digit in it, so no body number (R-449).
pub const CONVENTION_STATEMENT: &str =
    "Burrau convention, normalised: right angle at the origin, sides divided by \
     the hypotenuse, each mass its opposite side over the perimeter, so the masses sum to one";

/// The greatest common divisor of `x` and `y` (Euclid's algorithm); `gcd(x, 0) = x`.
pub fn gcd(x: u32, y: u32) -> u32 {
    let (mut x, mut y) = (x, y);
    while y != 0 {
        let r = x % y;
        x = y;
        y = r;
    }
    x
}

/// Whether `(m, n)` gives a primitive triple (§4.1): `m > n > 0`, `gcd(m, n) = 1`, `m − n` odd, which is `m` and `n`
/// of opposite parity, their lowest bits differing.
pub fn primitive(m: u32, n: u32) -> bool {
    m > n && n > 0 && gcd(m, n) == 1 && (m ^ n) & 1 == 1
}

/// A Pythagorean triple `a² + b² = c²`, `c` the hypotenuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Triple {
    /// The leg `a = m² − n²`.
    pub a: u64,
    /// The leg `b = 2mn`.
    pub b: u64,
    /// The hypotenuse `c = m² + n²`.
    pub c: u64,
}

/// Euclid's triple at `(m, n)` (§4.1): `Some` exactly where `(m, n)` is [`primitive`] and `c` fits a `u64`.
pub fn euclid(m: u32, n: u32) -> Option<Triple> {
    if !primitive(m, n) {
        return None;
    }
    let (m, n) = (m as u64, n as u64);
    let (mm, nn) = (m * m, n * n);
    Some(Triple {
        a: mm - nn,
        b: (2 * m).checked_mul(n)?,
        c: mm.checked_add(nn)?,
    })
}

impl Triple {
    /// The triple's system in the normalised Burrau convention, at the float type `R`: each side taken into `R` (exact
    /// below `R`'s integer range), then [`System::of_sides`].
    pub fn burrau<R: Float>(&self) -> System<R> {
        let side = |x: u64| <R as NumCast>::from(x).unwrap_or_else(R::nan);
        System::of_sides(side(self.a), side(self.b), side(self.c))
    }
}

/// A Burrau system in the normalised convention (the module's table): its masses, positions and Jacobi vectors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct System<R> {
    /// `(m₀, m₁, m₂) = (c, b, a)/(a+b+c)`.
    pub m: [R; 3],
    /// `r₀ = (0, 0)`, `r₁ = (a/c, 0)`, `r₂ = (0, b/c)`.
    pub r: [[R; 2]; 3],
    /// The inner pair's Jacobi vector `ρ = r₁ − r₀ = (a/c, 0)`.
    pub rho: [R; 2],
    /// The outer Jacobi vector `λ = r₂ − r₀₁ = (−ab/(c(b+c)), b/c)`.
    pub lambda: [R; 2],
}

impl<R: Float> System<R> {
    /// The system of the right triangle with legs `a`, `b` and hypotenuse `c` (§4.2), each quantity one rounding of
    /// its formula's rational: `a/c`, `b/c`, `(a·b)/(c·(b+c))` and each side over `a + b + c`.
    #[inline]
    pub fn of_sides(a: R, b: R, c: R) -> Self {
        let zero = R::zero();
        let s = a + b + c;
        let a_c = a / c;
        let b_c = b / c;
        System {
            m: [c / s, b / s, a / s],
            r: [[zero, zero], [a_c, zero], [zero, b_c]],
            rho: [a_c, zero],
            lambda: [-(a * b) / (c * (b + c)), b_c],
        }
    }
}

/// The 0-based index of body `k` of the family's 1-based form (the module's note): `k − 1` for `k ∈ {1, 2, 3}`,
/// `None` otherwise.
pub fn from_literature(k: u32) -> Option<u32> {
    if (1..=3).contains(&k) {
        Some(k - 1)
    } else {
        None
    }
}

/// The pair id of the 0-based bodies `i` and `j` (R-22; payload §2): the side opposite the third body, `3 − i − j`;
/// `None` unless `i` and `j` are two distinct bodies of `0, 1, 2`.
pub fn pair_id(i: u32, j: u32) -> Option<u32> {
    if i < 3 && j < 3 && i != j {
        Some(3 - i - j)
    } else {
        None
    }
}

/// The pair id of the bodies `i` and `j` of the family's 1-based form: [`pair_id`] of their 0-based indices; its
/// inner pair `1–2` is pair id 2.
pub fn literature_pair_id(i: u32, j: u32) -> Option<u32> {
    pair_id(from_literature(i)?, from_literature(j)?)
}
