//! Hold `L_z` and both held (chart_composition § 5.3; dd_decoder §3.7): the optional step after the decode and before
//! the derived invariants, `η_E`'s twin. With `⟨a, b⟩_m = Σ mᵢ aᵢ·bᵢ`, `J(x, y) = (−y, x)` and `w = J r`, the angular
//! momentum is `L_z(v) = ⟨w, v⟩_m`, `‖w‖²_m = I`, and every velocity splits orthogonally: `v = (L₀/I)·w + δv`.
//!
//! - **`L_z` alone**: `v ← v + ((L* − L₀)/I)·w`, the nearest point of `{L_z = L*}`. It raises no tag; a rest start
//!   becomes the rigid rotation `(L*/I)·w`.
//! - **Both held**: `v ← (L*/I)·w + η·δ̂`, `η = √((K* − L*²/2I)/K̂)`, `K* = E* − U`, the nearest point of the sphere
//!   `{L_z = L*, K = K*}`. `η_E` alone would break the held `L_z`, so this step takes its place.
//!
//! **The off switch**: the step is chosen on the switches, never on `L* = L₀`, and `η_E` alone is not routed through
//! here: [`apply_shared`] returns [`energy_norm::apply_shared`]'s result, bit for bit, wherever `L_z` is not held.
//!
//! **The scaled form.** `K_δ` is formed from `δv`, never as `K₀ − L₀²/2I`: near a rigid rotation that difference
//! cancels, goes negative and makes `η` non-real. `δ̂ = δv/s`, `s` the largest component, then `w` is projected out of
//! `δ̂` a second time, since `L_z(δv)` is rounding-sized, not 0, and `η` multiplies it. Don't drop the second
//! projection: at f32 and `‖δv‖/‖v‖ = 10⁻⁷` one pass leaves `|ΔL_z| ≈ 1`.
//!
//! **The outcomes** (both held, in this order; [`Outcome`]): 1 the collision pole, [`Outcome::Unscalable`]; 2
//! `E* < U`, [`Outcome::Infeasible`]; 3 `2K* < L*²`, [`Outcome::BelowFloor`]; 4 `rigid(v, w)` on the floor, the
//! rotation applied, no tag, [`Outcome::OnFloor`]; 5 `rigid(v, w)` above the floor, [`Outcome::Unscalable`]; 6
//! otherwise [`Outcome::Held`], the floor when not rigid giving `η = 0`. A tagged decode keeps its momenta as decoded,
//! neither hold applied.
//!
//! **The tags are exact.** `s = 0` and `K̂ = 0` come through divisions at each float's rounding, so they differ
//! between f32 and f64 on exactly rigid input. Outcomes 4 and 5 read [`rigid`] instead, a comparison decided exactly
//! from the bits, and outcome 3 reads [`floor_order`], the sign of `2K* − L*²` at `I = 1`, as `infeasible` does. Both
//! hold on every backend that keeps additions and multiplications correctly rounded and unreassociated, as
//! `decode::invariant`'s test does.
//!
//! The plan is unclear here; chose `τ = 2⁻²²` (`R::two_to(-22)` in `within`), the value the design document proposes, since the
//! constants register holds no rigid-draw `τ` (its `tau` is the escape threshold) and the calibration is another task's.
//! The plan is unclear here; chose to read `I = 1` in the tags (the floor test) as the design document says, and the
//! decode's own `I` in the arithmetic, so the held `L_z` lands on `L*` whatever `I`'s rounding, with `K* − L*²/2I`
//! floored at 0 so a pixel on the floor cannot make `η` non-real.
//! The plan is unclear here; chose to leave a charts' own held `L_z` untouched ([`LzHold::Construction`]): a view's
//! `L*` is not carried into such a chart's parameter here, so the rotation never runs after a construction.

use spirv_std::num_traits::Float;

use crate::chart::LzHold;
use crate::decode::descriptor::describe_decoded;
use crate::decode::energy_norm::{self, largest, Outcome};
use crate::decode::invariant::{feasibility_input, ExactSign};
use crate::decode::{Decoded, Degenerate, Ic};
use crate::payload::{ICDescriptorOf, PayloadReal};

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}

/// `a[i]`, `i` a loop's index, already below `N`. Don't index such an `i` directly in the GPU's code: its bounds check is
/// a panic path, and rust-gpu inlines every function that may panic, so [`rigid`] would be copied into each decode entry.
#[inline]
fn at<T: Copy, const N: usize>(a: &[T; N], i: usize) -> T {
    #[cfg(target_arch = "spirv")]
    {
        use spirv_std::arch::IndexUnchecked;
        // SAFETY: every caller's `i` is a loop index bounded by `N`.
        unsafe { *a.index_unchecked(i) }
    }
    #[cfg(not(target_arch = "spirv"))]
    {
        debug_assert!(i < N);
        a[i]
    }
}

/// `a[i] = x`, as [`at`] reads.
#[inline]
fn put<T, const N: usize>(a: &mut [T; N], i: usize, x: T) {
    #[cfg(target_arch = "spirv")]
    {
        use spirv_std::arch::IndexUnchecked;
        // SAFETY: every caller's `i` is a loop index bounded by `N`.
        unsafe { *a.index_unchecked_mut(i) = x }
    }
    #[cfg(not(target_arch = "spirv"))]
    {
        debug_assert!(i < N);
        a[i] = x;
    }
}

/// `a + b = s + e` exactly, `s` the rounded sum.
#[inline]
fn two_sum<R: ExactSign>(a: R, b: R) -> (R, R) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}

/// The sign (−1, 0, 1) of the exact sum of `add`: each is added to a nonoverlapping expansion by a chain of exact
/// two-sums, and the sign is that of the expansion's largest nonzero component.
#[inline]
fn sum_sign<R: ExactSign>(add: &[R; 27]) -> i32 {
    let zero = R::zero();
    let mut e = [zero; 27];
    let mut k = 0;
    while k < 27 {
        let mut q = at(add, k);
        let mut i = 0;
        while i < k {
            let (s, h) = two_sum(q, at(&e, i));
            put(&mut e, i, h);
            q = s;
            i += 1;
        }
        put(&mut e, k, q);
        k += 1;
    }
    let mut sign = 0;
    let mut i = 0;
    while i < 27 {
        let x = at(&e, i);
        sign = select(x > zero, 1, select(x < zero, -1, sign));
        i += 1;
    }
    sign
}

/// The nine products of `x`'s limbs by `y`'s, each exact, every one scaled by `scale` (a power of two or a sign).
#[inline]
fn limb_products<R: ExactSign>(x: R, y: R, scale: R) -> [R; 9] {
    let [x0, x1, x2] = x.limbs();
    let [y0, y1, y2] = y.limbs();
    [
        scale * (x0 * y0),
        scale * (x0 * y1),
        scale * (x0 * y2),
        scale * (x1 * y0),
        scale * (x1 * y1),
        scale * (x1 * y2),
        scale * (x2 * y0),
        scale * (x2 * y1),
        scale * (x2 * y2),
    ]
}

/// Whether `|v_b·w_p − v_p·w_b| ≤ τ·V·|w_p|`, decided exactly: `T − d` and `T + d` are both non-negative, `d` the
/// wedge and `T = τ·V·|w_p|`, each the sign of an exact sum of limb products (`limbs` is a bit mask, so no step is
/// a rounding).
#[inline]
fn within<R: ExactSign>(v_b: R, w_p: R, v_p: R, w_b: R, big_v: R, w_p_abs: R) -> bool {
    let one = R::one();
    let tau = R::two_to(-22);
    let t = limb_products(big_v, w_p_abs, tau);
    let a = limb_products(v_b, w_p, one);
    let b = limb_products(v_p, w_b, one);
    let mut minus = [R::zero(); 27];
    let mut plus = [R::zero(); 27];
    let mut i = 0;
    while i < 9 {
        let (t, a, b) = (at(&t, i), at(&a, i), at(&b, i));
        put(&mut minus, i, t);
        put(&mut minus, i + 9, -a);
        put(&mut minus, i + 18, b);
        put(&mut plus, i, t);
        put(&mut plus, i + 9, a);
        put(&mut plus, i + 18, -b);
        i += 1;
    }
    (sum_sign(&minus) >= 0) & (sum_sign(&plus) >= 0)
}

/// The six components of `a`, `(x₀, y₀, x₁, y₁, x₂, y₂)`.
#[inline]
fn flat<R: Copy>(a: &[[R; 2]; 3]) -> [R; 6] {
    [a[0][0], a[0][1], a[1][0], a[1][1], a[2][0], a[2][1]]
}

/// `rigid(v, w)`: `v` is parallel to `w` to within `τ`, `|v_b·w_p − v_p·w_b| ≤ τ·V·|w_p|` for every component `b`, with
/// `p` the first index of the largest `|wᵢ|` and `V` the largest `|vᵢ|`, over the six components. Exact from the bits,
/// with no division and no mass: one input gives one answer at f32 and f64. Not rigid gives `|δv|∞ ≥ τV/2`, so a
/// direction exists. `v = 0` and `w = 0` are rigid.
#[inline]
pub fn rigid<R: ExactSign>(v: &[[R; 2]; 3], w: &[[R; 2]; 3]) -> bool {
    let zero = R::zero();
    let (v, w) = (flat(v), flat(w));
    let mut big_v = zero;
    let mut big_w = zero;
    let mut i = 0;
    while i < 6 {
        big_v = Float::max(big_v, Float::abs(at(&v, i)));
        big_w = Float::max(big_w, Float::abs(at(&w, i)));
        i += 1;
    }
    // The first index of the largest, by walking down so the lowest wins.
    let mut p = 0;
    let mut i = 6;
    while i > 0 {
        i -= 1;
        p = select(Float::abs(at(&w, i)) == big_w, i, p);
    }
    let (w_p, v_p) = (at(&w, p), at(&v, p));
    let mut all = true;
    let mut b = 0;
    while b < 6 {
        all &= within(at(&v, b), w_p, v_p, at(&w, b), big_v, big_w);
        b += 1;
    }
    all
}

/// [`rigid`] with its arrays by value, out of line on the GPU, so the module holds one copy of its loops, not one per
/// decode entry: every pipeline parses the whole module. By value, since rust-gpu inlines a call whose pointer
/// argument is not a local variable. Only the exact tests go out of line: they read no rounding, so the boundary moves
/// no bit, where the hold's float steps out of line round differently on Metal (a call boundary changes what its
/// compiler fuses).
#[cfg_attr(not(target_arch = "spirv"), inline)]
#[cfg_attr(target_arch = "spirv", inline(never))]
fn rigid_by_value<R: ExactSign>(v: [[R; 2]; 3], w: [[R; 2]; 3]) -> bool {
    rigid(&v, &w)
}

/// [`floor_order`] out of line on the GPU, as [`rigid_by_value`] is.
#[cfg_attr(not(target_arch = "spirv"), inline)]
#[cfg_attr(target_arch = "spirv", inline(never))]
fn floor_order_out_of_line<R: ExactSign>(k: R, l: R) -> i32 {
    floor_order(k, l)
}

/// The sign of `2K* − L*²` at `I = 1` (−1, 0, 1), exact: [`crate::decode::invariant::infeasible`]'s test, whose `< 0`
/// is `−1` here, with the floor `2K* = L*²` told from above it.
#[inline]
pub fn floor_order<R: ExactSign>(k: R, l: R) -> i32 {
    let f = feasibility_input(R::one(), k, l);
    let zero = R::zero();
    let by_columns = select(f.columns < zero, -1, select(f.columns > zero, 1, 0));
    let close = select(f.e > 1, 1, select(f.e < -1, -1, by_columns));
    select(
        f.flat,
        select(f.moving, -1, 0),
        select(f.negative, -1, select(f.moving, close, 1)),
    )
}

/// The held velocities and the outcome.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held<R> {
    /// The velocities after the step: held where [`Outcome::Held`] or [`Outcome::OnFloor`], as given otherwise.
    pub v: [[R; 2]; 3],
    /// What the step did.
    pub outcome: Outcome,
}

/// `Σ mᵢ aᵢ·bᵢ`.
#[inline]
fn inner<R: ExactSign>(m: [R; 3], a: &[[R; 2]; 3], b: &[[R; 2]; 3]) -> R {
    let one = |i: usize| m[i] * (a[i][0] * b[i][0] + a[i][1] * b[i][1]);
    one(0) + one(1) + one(2)
}

/// `a + c·b`.
#[inline]
fn axpy<R: ExactSign>(a: &[[R; 2]; 3], c: R, b: &[[R; 2]; 3]) -> [[R; 2]; 3] {
    let one = |i: usize| [a[i][0] + c * b[i][0], a[i][1] + c * b[i][1]];
    [one(0), one(1), one(2)]
}

/// Holds `L_z = lz` on the velocities `v` at the positions `r` with the masses `m`, and `E = e_star` too where
/// `e_on`, `v0` and `r_min` the descriptor's `V₀` and `r_min_pair_0` (the module's note). Everything is computed and the
/// outcome selects: no branch on a value.
#[inline]
#[allow(clippy::too_many_arguments)] // Reason: the kernel passes values one by one, never a struct across the SPIR-V seam.
pub fn hold<R: ExactSign>(
    m: [R; 3],
    r: &[[R; 2]; 3],
    v: &[[R; 2]; 3],
    v0: R,
    r_min: R,
    lz: R,
    e_on: bool,
    e_star: R,
) -> Held<R> {
    let zero = R::zero();
    let one = R::one();
    let half = one / (one + one);
    let w = [
        [-r[0][1], r[0][0]],
        [-r[1][1], r[1][0]],
        [-r[2][1], r[2][0]],
    ];
    let i_raw = inner(m, &w, &w);
    let i = select(i_raw == zero, one, i_raw);
    let l0 = inner(m, &w, v);

    let alone = axpy(v, (lz - l0) / i, &w);

    let rotation = axpy(&[[zero; 2]; 3], lz / i, &w);
    let dv = axpy(v, -(l0 / i), &w);
    let s = largest(&dv);
    let s = select(s == zero, one, s);
    let unit = [
        [dv[0][0] / s, dv[0][1] / s],
        [dv[1][0] / s, dv[1][1] / s],
        [dv[2][0] / s, dv[2][1] / s],
    ];
    let unit = axpy(&unit, -(inner(m, &w, &unit) / i), &w);
    let k_hat = half * inner(m, &unit, &unit);
    let direction = k_hat > zero;

    let collision = r_min == zero;
    let u = select(collision, zero, v0);
    let k_star = e_star - u;
    let spare = Float::max(k_star - lz * lz / (i + i), zero);
    let eta = select(
        direction,
        Float::sqrt(spare / select(direction, k_hat, one)),
        zero,
    );
    let both = axpy(&rotation, eta, &unit);

    let order = floor_order_out_of_line(k_star, lz);
    let is_rigid = rigid_by_value(*v, w);
    let step = select(
        is_rigid,
        select(order == 0, Outcome::OnFloor, Outcome::Unscalable),
        Outcome::Held,
    );
    let step = select(order < 0, Outcome::BelowFloor, step);
    let step = select(e_star < u, Outcome::Infeasible, step);
    let step = select(collision, Outcome::Unscalable, step);

    let v_joint = select(
        step == Outcome::Held,
        both,
        select(step == Outcome::OnFloor, rotation, *v),
    );
    Held {
        v: select(e_on, v_joint, alone),
        outcome: select(e_on, step, Outcome::Held),
    }
}

/// [`hold`] on a decode's `ic`, with its descriptor's `v0` and `r_min` and its tag `degenerate`: the momenta as
/// `m·v`, the decode's tag or else the outcome's, and the outcome. A tagged decode keeps its momenta bit for bit.
#[inline]
#[allow(clippy::too_many_arguments)] // Reason: the kernel passes values one by one, never a struct across the SPIR-V seam.
fn held<R: ExactSign + PayloadReal>(
    ic: Ic<R>,
    v0: R,
    r_min: R,
    degenerate: Option<Degenerate>,
    e_star: Option<R>,
    forbids_energy_normalisation: bool,
    lz: R,
) -> ([[R; 2]; 3], Option<Degenerate>, Outcome) {
    let (e_on, target) = match e_star {
        Some(e) => (!forbids_energy_normalisation, e),
        None => (false, R::zero()),
    };
    let (m, p) = (ic.m, ic.p);
    let v = [
        [p[0][0] / m[0], p[0][1] / m[0]],
        [p[1][0] / m[1], p[1][1] / m[1]],
        [p[2][0] / m[2], p[2][1] / m[2]],
    ];
    let h = hold(m, &ic.r, &v, v0, r_min, lz, e_on, target);
    let moved = (h.outcome == Outcome::Held) | (h.outcome == Outcome::OnFloor);
    let hv = h.v;
    (
        [
            [
                select(moved, m[0] * hv[0][0], p[0][0]),
                select(moved, m[0] * hv[0][1], p[0][1]),
            ],
            [
                select(moved, m[1] * hv[1][0], p[1][0]),
                select(moved, m[1] * hv[1][1], p[1][1]),
            ],
            [
                select(moved, m[2] * hv[2][0], p[2][0]),
                select(moved, m[2] * hv[2][1], p[2][1]),
            ],
        ],
        degenerate.or(h.outcome.tag()),
        h.outcome,
    )
}

/// The decode stage's step under both switches (the module's note): `η_E` alone, [`energy_norm::apply_shared`],
/// where `lz_star` is `None` or the chart's `L_z` is not free (`lz_hold`); else [`hold`], alone or with `E` where
/// `e_star` is `Some` and the chart allows it. [`hold`] runs only where the switch is on; with `L_z` off the result is
/// `apply_shared`'s, bit for bit.
#[inline]
pub fn apply_shared<R: ExactSign + PayloadReal>(
    decoded: Decoded<R>,
    e_star: Option<R>,
    forbids_energy_normalisation: bool,
    lz_star: Option<R>,
    lz_hold: LzHold,
) -> (Decoded<R>, Outcome) {
    let (on, lz) = match lz_star {
        Some(l) => (lz_hold == LzHold::Free, l),
        None => (false, R::zero()),
    };
    let base = energy_norm::apply_shared(decoded, e_star, forbids_energy_normalisation);
    // A branch, not a select of both as `energy_norm::apply_shared` makes: `hold`'s exact tests cost many decodes, and
    // a select runs them on every sample with `L_z` off. `on` is a uniform's, so the branch never diverges. Only plain
    // values cross it, never a decode: the descriptor's tail is a zero-length array, which naga refuses as an empty
    // struct, so the descriptor is formed after from the chosen IC as `energy_norm::apply_shared` forms it.
    let (base_p, base_tag, base_outcome) = (base.0.ic.p, base.0.degenerate, base.1);
    let d: &ICDescriptorOf<R> = &decoded.descriptor;
    let (ic0, v0, r_min, tag0) = (decoded.ic, d.V_0, d.r_min_pair_0, decoded.degenerate);
    let (p, degenerate, outcome) = if on {
        held(
            ic0,
            v0,
            r_min,
            tag0,
            e_star,
            forbids_energy_normalisation,
            lz,
        )
    } else {
        (base_p, base_tag, base_outcome)
    };
    let ic = Ic { p, ..decoded.ic };
    (
        Decoded {
            ic,
            descriptor: describe_decoded(&ic, &decoded.config),
            degenerate,
            ..decoded
        },
        outcome,
    )
}

/// What the holds do to a swept momentum plane (chart_composition § 5.4): the plane `v₀ + a·e₁ + b·e₂` in velocity
/// space, the holds' switches, and whether the held decode maps it onto a curve of systems, [`SystemImage::RayDegenerate`],
/// or keeps rank 2, [`SystemImage::Bijective`]. Three cases lose rank: with `L_z` held, alone or with `E`, a plane that
/// contains `Jr`; with `E` alone, a plane through rest, the radial projection's centre; with both held, a plane through a
/// point with `δv = 0`, rest or a rigid rotation, the scale's centre. Don't extend the third to `E` alone: the
/// rotation, not rest, is not that projection's centre, and a plane through it keeps rank 2.
///
/// The plan is unclear here; chose `√ε` as the relative residual that counts as "contains", since an exact plane
/// leaves about `ε` and the measured singular-value ratios are at most `10⁻⁹` where rank is lost and at least `10⁻²` where it is not.
#[cfg(not(target_arch = "spirv"))]
pub fn plane_image(
    m: [f64; 3],
    r: &[[f64; 2]; 3],
    base: &[[f64; 2]; 3],
    e: [&[[f64; 2]; 3]; 2],
    e_on: bool,
    lz_on: bool,
) -> crate::chart::SystemImage {
    use crate::chart::SystemImage;
    // The relative residual that counts as zero: an exact plane leaves about eps, so its square root clears it.
    let tol = f64::EPSILON.sqrt();
    let w = [
        [-r[0][1], r[0][0]],
        [-r[1][1], r[1][0]],
        [-r[2][1], r[2][0]],
    ];
    let ip = |a: &[[f64; 2]; 3], b: &[[f64; 2]; 3]| {
        (0..3)
            .map(|i| m[i] * (a[i][0] * b[i][0] + a[i][1] * b[i][1]))
            .sum::<f64>()
    };
    let i = ip(&w, &w);
    let sub = |a: &[[f64; 2]; 3], c: f64, b: &[[f64; 2]; 3]| {
        [0, 1, 2].map(|k| [a[k][0] - c * b[k][0], a[k][1] - c * b[k][1]])
    };
    // `x` with `w` projected out, the part orthogonal to the rotation.
    let perp = |x: &[[f64; 2]; 3]| sub(x, ip(&w, x) / i, &w);
    // Whether `x` lies in the span of `a` and `b`: Gram–Schmidt on the pair, a vector that adds no direction dropped.
    let within = |x: &[[f64; 2]; 3], a: &[[f64; 2]; 3], b: &[[f64; 2]; 3]| {
        let mut basis = [[[f64::default(); 2]; 3]; 2];
        let mut count = 0;
        for v in [a, b] {
            let mut rest = *v;
            for q in basis.iter().take(count) {
                rest = sub(&rest, ip(&rest, q), q);
            }
            let n = ip(&rest, &rest).sqrt();
            if n > tol * ip(v, v).sqrt() {
                basis[count] = [0, 1, 2].map(|k| [rest[k][0] / n, rest[k][1] / n]);
                count += 1;
            }
        }
        let mut left = *x;
        for q in basis.iter().take(count) {
            left = sub(&left, ip(&left, q), q);
        }
        ip(&left, &left).sqrt() <= tol * ip(x, x).sqrt()
    };
    let contains_jr = within(&w, e[0], e[1]);
    let through_rest = ip(base, base) == f64::default() || within(base, e[0], e[1]);
    let through_rigid = {
        let (pb, p0, p1) = (perp(base), perp(e[0]), perp(e[1]));
        ip(&pb, &pb) <= tol * ip(base, base) || within(&pb, &p0, &p1)
    };
    let loses =
        (lz_on & contains_jr) | (e_on & !lz_on & through_rest) | (e_on & lz_on & through_rigid);
    if loses {
        SystemImage::RayDegenerate
    } else {
        SystemImage::Bijective
    }
}

/// A view's two targets: `E*` for `η_E` and `L*` for the hold on `L_z`, each off at `None` (`Some(0)` is a real target).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Targets<R> {
    /// `E*`.
    pub e_star: Option<R>,
    /// `L*`.
    pub lz_star: Option<R>,
}

impl<R: Copy> Targets<R> {
    /// Both off: every decode is the chart's own.
    pub const OFF: Targets<R> = Targets {
        e_star: None,
        lz_star: None,
    };

    /// Whether neither hold is on.
    #[inline]
    pub fn is_off(&self) -> bool {
        self.e_star.is_none() & self.lz_star.is_none()
    }
}

/// `ic` through the holds `t` on a chart with the flags `forbids_energy_normalisation` and `lz_hold`: `η_E` alone, hold
/// `L_z` alone or both, as the decode stage applies them ([`apply_shared`]), the descriptor read from `ic` as a decode
/// writes it. The state and what the step did; the state is `ic`'s own, bit for bit, where no hold applies.
#[inline]
pub fn hold_ic<R: ExactSign + PayloadReal>(
    ic: &crate::decode::Ic<R>,
    t: &Targets<R>,
    forbids_energy_normalisation: bool,
    lz_hold: LzHold,
) -> (crate::decode::Ic<R>, Outcome) {
    let d = crate::decode::descriptor::describe(ic);
    let e_on = t.e_star.is_some() & !forbids_energy_normalisation;
    let target = t.e_star.unwrap_or_else(R::zero);
    match t.lz_star {
        Some(lz) if lz_hold == LzHold::Free => {
            let velocity = |i: usize| [ic.p[i][0] / ic.m[i], ic.p[i][1] / ic.m[i]];
            let v = [velocity(0), velocity(1), velocity(2)];
            let h = hold(ic.m, &ic.r, &v, d.V_0, d.r_min_pair_0, lz, e_on, target);
            let moved = (h.outcome == Outcome::Held) | (h.outcome == Outcome::OnFloor);
            let momentum = |i: usize| {
                [
                    select(moved, ic.m[i] * h.v[i][0], ic.p[i][0]),
                    select(moved, ic.m[i] * h.v[i][1], ic.p[i][1]),
                ]
            };
            (
                crate::decode::Ic {
                    p: [momentum(0), momentum(1), momentum(2)],
                    ..*ic
                },
                h.outcome,
            )
        }
        _ if e_on => {
            let n = energy_norm::normalise(ic, d.V_0, d.r_min_pair_0, target);
            (crate::decode::Ic { p: n.p, ..*ic }, n.outcome)
        }
        _ => (*ic, Outcome::Off),
    }
}
