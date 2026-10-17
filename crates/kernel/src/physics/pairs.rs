//! `PairGeom`, the pairwise geometry of three bodies, computed once per force evaluation and read by the force, the
//! potential term of `E` and `r_min` (dd_integrator §3.1: "‖·‖³ shared with r_min"; REQ-INT-036, REQ-PERF-001).
//!
//! The pairs are indexed by R-22's pair id: pair `k` is the side opposite body `k`, `(i, j) = (1, 2), (2, 0), (0, 1)`
//! for `k = 0, 1, 2` (`i = k + 1`, `j = k + 2`, mod 3), the order `burrau::pair_id` and `shape::Landmarks::bc` use and
//! dd_simstate_payload §2's `detail` and `dmin_pair` read. Pair `k`'s separation is `dₖ = rⱼ − rᵢ`.

use crate::Real;

/// The three pairs' separations `dₖ = rⱼ − rᵢ`, by R-22 pair id `k`, `(i, j) = (1, 2), (2, 0), (0, 1)`, with each one's `d²`, `‖dₖ‖`
/// and `‖dₖ‖³`. Built only by [`PairGeom::new`], so each is the one computation its readers share.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PairGeom<R> {
    sep: [[R; 2]; 3],
    d2: [R; 3],
    norm: [R; 3],
    cube: [R; 3],
}

impl<R: Real> PairGeom<R> {
    /// The pairwise geometry of the positions `r`: per pair, `dₖ`, `d² = dₖ·dₖ`, `‖dₖ‖ = √d²` and
    /// `‖dₖ‖³ = d²·‖dₖ‖`.
    #[inline]
    pub fn new(r: &[[R; 2]; 3]) -> Self {
        let sep = [
            [r[2][0] - r[1][0], r[2][1] - r[1][1]],
            [r[0][0] - r[2][0], r[0][1] - r[2][1]],
            [r[1][0] - r[0][0], r[1][1] - r[0][1]],
        ];
        let d2 = [
            sep[0][0] * sep[0][0] + sep[0][1] * sep[0][1],
            sep[1][0] * sep[1][0] + sep[1][1] * sep[1][1],
            sep[2][0] * sep[2][0] + sep[2][1] * sep[2][1],
        ];
        let norm = [
            d2[0].square_root(),
            d2[1].square_root(),
            d2[2].square_root(),
        ];
        let cube = [d2[0] * norm[0], d2[1] * norm[1], d2[2] * norm[2]];
        PairGeom {
            sep,
            d2,
            norm,
            cube,
        }
    }

    /// The separations `dₖ = rⱼ − rᵢ`.
    #[inline]
    pub fn sep(&self) -> [[R; 2]; 3] {
        self.sep
    }

    /// The squared distances `d²ₖ`.
    #[inline]
    pub fn d2(&self) -> [R; 3] {
        self.d2
    }

    /// The distances `‖dₖ‖`.
    #[inline]
    pub fn norm(&self) -> [R; 3] {
        self.norm
    }

    /// The cubed distances `‖dₖ‖³`.
    #[inline]
    pub fn cube(&self) -> [R; 3] {
        self.cube
    }

    /// The closest pair's R-22 id: the least `d²`, the lowest pair id of a tie. A comparison of `d²`, not of `‖·‖` (Part 2c rule
    /// 2), folded from pair 0 rather than from an infinity (rule 4).
    #[inline]
    pub fn closest(&self) -> usize {
        let mut k = 0;
        if self.d2[1] < self.d2[k] {
            k = 1;
        }
        if self.d2[2] < self.d2[k] {
            k = 2;
        }
        k
    }

    /// `r_min`, the least pairwise distance: the closest pair's `‖dₖ‖`, read from this geometry, not recomputed.
    #[inline]
    pub fn r_min(&self) -> R {
        self.norm[self.closest()]
    }
}
