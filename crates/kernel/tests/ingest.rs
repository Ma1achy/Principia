//! Ground-truth ingestion (inverse-encode contract Parts 4 and 7; REQ-VAL-014, REQ-ENC-025; TASK-M2-20): foreign
//! states, the Lagrange equilateral, the figure-eight and the Burrau rest start, each in its published frame, are
//! ingested, and the result carries `z`, the physical residual `‖D(z) − C(x)‖_phys` and `κ(x)`.
//!
//! **The starts** (`principia_listen.md` § "12. Sweep and known orbits", total mass 1 and `G = 1`): the figure-eight,
//! masses ⅓, `r₁ = (−0.97000436, 0.24308753)`, `r₂ = −r₁`, `r₃ = 0`, `v₁ = v₂ = −v·s/2`, `v₃ = v·s`, `v = (−0.93240737,
//! −0.86473146)`, `s = 1/√3`; Lagrange rotating, equal masses at `(cos 2πi/3, sin 2πi/3)` moving at `ω(−y, x)`,
//! `ω = √(1/(3√3))`; Burrau 3:4:5, masses `(3, 4, 5)/12` at `(1, 3)`, `(−2, −1)`, `(1, −1)`, at rest. Each is also
//! taken through a gauge element (a turn, an offset, a boost and a rescale), so `C` meets a foreign frame.
//!
//! **What is checked** (applied per R-369, the test's decisions): the result is `E(x)`, the free decode of its `z`, the
//! residual Part 4's norm from `C(x)` to `D(z)`, within `ε_phys` (REQ-ENC-024's register value) with no control clamped
//! (the figure-eight's published start is collinear with body 3 at the inner pair's centre of mass, the `α = 0` pole:
//! it raises `lookup_clamped` and its `κ` is large, as Part 4 says it must), and `κ(x)` is `σ_max(J_E(x)·W⁻¹)`, the
//! encode's weighted Jacobian at the input itself, checked against an independent oracle: `σ²·(1 + δ)` less the Gram
//! matrix `(J_E·W⁻¹)(J_E·W⁻¹)ᵀ` is positive definite and `σ²·(1 − δ)` less it is not (Cholesky), `δ = 10⁻⁶`; and Part
//! 4's bound `‖E(x̃) − z‖₂ ≤ κ(x)·‖W(x̃ − x)‖₂ ≤ 3·κ(x)·‖x̃ − x‖_phys` on states `x̃` moved off `x` along every
//! component, off the canonical section as well as along it (the physics review of PR #294, P1: the section's tangent
//! alone, `1/σ_min(W·J_D)`, does not bound a move of `m₀` at masses `(0.05, 0.15, 0.8)`).

use kernel::decode::{self, Ic, Latent};
use kernel::encode::encode;
use kernel::encode::ingest::{
    components, condition, ingest, kappa, sigma_max, sigma_min, state as from_components,
    weighted_encode_jacobian, weighted_jacobian, weights, Ingested, Phys,
};
use kernel::generated::links::{Defaults, Select, SigmoidAlpha, SigmoidBeta, SoftmaxTanh, TanhQ};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};

/// The defaults with every free momentum's link swapped to `tanh_q`: another registry link for the same controls.
type TanhMomenta = Select<SoftmaxTanh, SigmoidAlpha, SigmoidBeta, TanhQ, TanhQ, TanhQ, TanhQ>;

/// A state from masses, positions and velocities.
fn state(m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> Ic<f64> {
    Ic {
        m,
        r,
        p: std::array::from_fn(|i| v[i].map(|c| m[i] * c)),
    }
}

/// The figure-eight, Lagrange and Burrau starts, named.
fn starts() -> Vec<(&'static str, Ic<f64>)> {
    let third = 1.0 / 3.0;
    let (x, y) = (-0.970_004_36, 0.243_087_53);
    let s = 1.0 / 3.0f64.sqrt();
    let v = [-0.932_407_37 * s, -0.864_731_46 * s];
    let eight = state(
        [third; 3],
        [[x, y], [-x, -y], [0.0, 0.0]],
        [[-v[0] / 2.0, -v[1] / 2.0], [-v[0] / 2.0, -v[1] / 2.0], v],
    );
    let omega = (1.0 / (3.0 * 3.0f64.sqrt())).sqrt();
    let corner = |i: usize| {
        let a = 2.0 * std::f64::consts::PI * i as f64 / 3.0;
        [a.cos(), a.sin()]
    };
    let r = [corner(0), corner(1), corner(2)];
    let lagrange = state([third; 3], r, r.map(|c| [-omega * c[1], omega * c[0]]));
    let burrau = state(
        [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0],
        [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]],
        [[0.0; 2]; 3],
    );
    vec![
        ("figure-eight", eight),
        ("Lagrange", lagrange),
        ("Burrau", burrau),
    ]
}

/// `x` turned by `θ`, offset by `d`, boosted by `b` and rescaled by `k` (positions × k, momenta × k^{−1/2}): a gauge
/// element, so `C(g·x) = C(x)` to rounding.
fn moved(x: &Ic<f64>, theta: f64, d: [f64; 2], b: [f64; 2], k: f64) -> Ic<f64> {
    let (c, s) = (theta.cos(), theta.sin());
    let turn = |v: [f64; 2]| [c * v[0] - s * v[1], s * v[0] + c * v[1]];
    Ic {
        m: x.m,
        r: x.r.map(|r| {
            let t = turn(r);
            [k * t[0] + d[0], k * t[1] + d[1]]
        }),
        p: std::array::from_fn(|i| {
            let t = turn(x.p[i]);
            [
                (t[0] + x.m[i] * b[0]) / k.sqrt(),
                (t[1] + x.m[i] * b[1]) / k.sqrt(),
            ]
        }),
    }
}

/// Every start in its published frame and under two gauge elements.
fn foreign() -> Vec<(String, Ic<f64>)> {
    let mut out = Vec::new();
    for (name, x) in starts() {
        out.push((name.to_owned(), x));
        out.push((
            format!("{name}, moved"),
            moved(&x, 0.7, [3.0, -2.0], [0.25, -0.5], 7.5),
        ));
        out.push((
            format!("{name}, moved small"),
            moved(&x, -2.3, [-0.1, 0.4], [-1.5, 0.2], 0.04),
        ));
    }
    out
}

fn ingested(x: &Ic<f64>) -> Ingested<f64> {
    ingest::<Defaults, f64, _>(x, phys)
}

/// The result carries `E(x)`, the free decode of its `z`, the residual from `C(x)` to `D(z)` in Part 4's norm and
/// `κ(x)` at the input, each as its definition gives it.
fn check_outputs(name: &str, x: &Ic<f64>, got: &Ingested<f64>) {
    let e = encode::<Defaults, f64>(x);
    assert_eq!(got.encoded, e, "{name}: the encoded part is not E(x)");
    let d = decode::decode::<Defaults, f64>(&e.z).ic;
    assert_eq!(got.decoded, d, "{name}: the decoded state is not D(z)");
    assert_eq!(
        got.residual.to_bits(),
        phys(&e.canonical.ic, &d).to_bits(),
        "{name}: the residual is not ‖D(z) − C(x)‖_phys"
    );
    let k = kappa::<Defaults, f64>(x);
    assert_eq!(got.kappa.to_bits(), k.to_bits(), "{name}: κ is not κ(x)");
    assert!(
        got.kappa.is_finite() && got.kappa >= 1.0,
        "{name}: κ = {} is not a finite condition number",
        got.kappa
    );
}

/// The residual is within `ε_phys`, no control clamped: the start is faithfully ingested. The figure-eight's published
/// start is the exception, and says so: body 3 sits at the inner pair's centre of mass (`r₃ = 0 = (r₁ + r₂)/2`), the
/// `α = 0` pole, where `α`'s link clamps (Part 4: "at the clamps, the inverse is by construction lossy and *says so*")
/// and `β` is not defined, so `κ` is far from 1 (Part 4: `κ = +∞` at the pole, the central difference giving a large
/// finite value).
fn check_residual(name: &str, got: &Ingested<f64>, at_pole: bool) {
    let bits = got.encoded.report.clamped_controls;
    if at_pole {
        assert!(
            bits & 1 == 1,
            "{name}: the α = 0 pole start does not raise lookup_clamped on z_α ({bits:#b})"
        );
        assert!(
            got.kappa >= 1e5,
            "{name}: κ = {:e} at the α = 0 pole is not large",
            got.kappa
        );
        return;
    }
    assert_eq!(
        bits, 0,
        "{name}: an interior start raises lookup_clamped ({bits:#b})"
    );
    check_within(name, got.residual);
}

/// `residual ≤ ε_phys`.
fn check_within(name: &str, residual: f64) {
    let tol = eps_phys::<f64>();
    assert!(
        residual <= tol,
        "{name}: ‖D(z) − C(x)‖_phys = {residual:e} is beyond ε_phys = {tol:e}"
    );
}

#[test]
fn ingest_outputs_carry_z_residual_and_kappa() {
    for (name, x) in foreign() {
        let got = ingested(&x);
        check_outputs(&name, &x, &got);
        check_residual(&name, &got, name.starts_with("figure-eight"));
    }
}

negative_control!(
    ingest_outputs_carry_z_residual_and_kappa,
    "a result whose residual is measured from x, not C(x), is not the residual Part 7 asks for",
    expected = "the residual is not ‖D(z) − C(x)‖_phys",
    {
        let (name, x) = &foreign()[1];
        let mut got = ingested(x);
        got.residual = phys(x, &got.decoded);
        check_outputs(name, x, &got);
    }
);

mod pole_not_flagged {
    use super::*;
    negative_control!(
        ingest_outputs_carry_z_residual_and_kappa,
        "the Lagrange start, interior, is not at the α = 0 pole and raises no clamp",
        expected = "does not raise lookup_clamped on z_α",
        {
            let (name, x) = &foreign()[3];
            check_residual(name, &ingested(x), true);
        }
    );
}

mod pole_read_as_interior {
    use super::*;
    negative_control!(
        ingest_outputs_carry_z_residual_and_kappa,
        "the figure-eight's start, at the α = 0 pole, raises lookup_clamped and is not interior",
        expected = "an interior start raises lookup_clamped",
        {
            let (name, x) = &foreign()[0];
            check_residual(name, &ingested(x), false);
        }
    );
}

mod mismatched_decode {
    use super::*;

    negative_control!(
        ingest_outputs_carry_z_residual_and_kappa,
        "z decoded under another link than encode's gives a residual far beyond ε_phys (dd_encode §4, seam 2/13)",
        expected = "is beyond ε_phys",
        {
            let (name, x) = &foreign()[3];
            let enc = encode::<Defaults, f64>(x);
            let back = decode::decode::<TanhMomenta, f64>(&enc.z).ic;
            check_within(name, phys(&enc.canonical.ic, &back));
        }
    );
}

// ── κ ─────────────────────────────────────────────────────────────────────────────────────────────────────────────

/// The 8 × 8 Gram matrix `AᵀA` of the columns `cols`.
fn gram(cols: &[Phys<f64>; 8]) -> [[f64; 8]; 8] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..15).map(|k| cols[i][k] * cols[j][k]).sum()))
}

/// Whether the symmetric `a` is positive definite, by Cholesky.
fn positive_definite(a: &[[f64; 8]; 8]) -> bool {
    let mut l = [[0.0f64; 8]; 8];
    for i in 0..8 {
        for j in 0..=i {
            let s: f64 = a[i][j] - (0..j).map(|k| l[i][k] * l[j][k]).sum::<f64>();
            if i == j {
                if s <= 0.0 {
                    return false;
                }
                l[i][i] = s.sqrt();
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    true
}

/// `σ` is the largest singular value of `cols` to the relative `δ = 10⁻⁶`: `σ²(1 + δ)I − AᵀA` is positive definite
/// and `σ²(1 − δ)I − AᵀA` is not.
fn check_sigma(name: &str, cols: &[Phys<f64>; 8], sigma: f64) {
    let g = gram(cols);
    let shifted = |f: f64| {
        let mut a = g.map(|row| row.map(|v| -v));
        for (i, row) in a.iter_mut().enumerate() {
            row[i] += sigma * sigma * f;
        }
        a
    };
    assert!(
        positive_definite(&shifted(1.0 + 1e-6)),
        "{name}: σ = {sigma:e} is below the largest singular value"
    );
    assert!(
        !positive_definite(&shifted(1.0 - 1e-6)),
        "{name}: σ = {sigma:e} is above the largest singular value"
    );
}

#[test]
fn ingest_outputs_kappa_is_the_encode_jacobians_norm() {
    // The Gram oracle squares κ, so it resolves σ_max only where κ² is well inside 1/ε: not at the figure-eight's
    // α = 0 pole, where κ ≥ 10⁵ (check_residual).
    for (name, x) in foreign()
        .into_iter()
        .filter(|(n, _)| !n.starts_with("figure-eight"))
    {
        let cols = weighted_encode_jacobian::<Defaults, f64>(&x);
        let sigma = sigma_max(cols);
        check_sigma(&name, &cols, sigma);
        assert_eq!(
            kappa::<Defaults, f64>(&x),
            sigma,
            "{name}: κ is not σ_max(J_E·W⁻¹)"
        );
    }
}

negative_control!(
    ingest_outputs_kappa_is_the_encode_jacobians_norm,
    "twice the largest singular value is not the largest singular value",
    expected = "is above the largest singular value",
    {
        let (name, x) = &foreign()[3];
        let cols = weighted_encode_jacobian::<Defaults, f64>(x);
        check_sigma(name, &cols, 2.0 * sigma_max(cols));
    }
);

mod half_sigma {
    use super::*;
    negative_control!(
        ingest_outputs_kappa_is_the_encode_jacobians_norm,
        "half the largest singular value is not the largest singular value",
        expected = "is below the largest singular value",
        {
            let (name, x) = &foreign()[3];
            let cols = weighted_encode_jacobian::<Defaults, f64>(x);
            check_sigma(name, &cols, 0.5 * sigma_max(cols));
        }
    );
}

/// The encode Jacobian's column `k`, read back from [`weighted_encode_jacobian`], is the central difference of `E`
/// along `x`'s component `k` divided by `W`'s entry: checked on the light start's `m₀` and on body 3's `p_y`, against
/// a difference formed here at the same step.
fn check_column(x: &Ic<f64>, k: usize, scale: f64) {
    let cols = weighted_encode_jacobian::<Defaults, f64>(x);
    let c = components(x);
    let w = weights(x);
    let h = f64::EPSILON.cbrt() * c[k].abs().max(1.0);
    let (mut up, mut down) = (c, c);
    up[k] += h;
    down[k] -= h;
    let (zp, zm) = (
        encode::<Defaults, f64>(&from_components(&up)).z.z,
        encode::<Defaults, f64>(&from_components(&down)).z.z,
    );
    for i in 0..8 {
        let want = scale * (zp[i] - zm[i]) / ((up[k] - down[k]) * w[k]);
        assert_eq!(
            cols[i][k], want,
            "entry ({i}, {k}) of J_E·W⁻¹ is {:e}, the difference {want:e}",
            cols[i][k]
        );
    }
}

/// `W·J_D(z)`'s column `k`, read back from [`weighted_jacobian`], is the central difference of `D` along `zₖ` at
/// Part 4's step `hₖ = ε_R^{1/3}·max(1, |zₖ|)`, weighted by `W` at `D(z)`: checked against a difference formed here, on
/// the Burrau start's `z` with `z_q0` set to `−3`, so `|zₖ| > 1` and the step's scale matters.
fn check_decode_column(z: [f64; 8], k: usize, step: f64) {
    let cols = weighted_jacobian::<Defaults, f64>(&Latent::new(z));
    let w = weights(&decode::decode::<Defaults, f64>(&Latent::new(z)).ic);
    let h = step * z[k].abs().max(1.0);
    let (mut up, mut down) = (z, z);
    up[k] += h;
    down[k] -= h;
    let plus = components(&decode::decode::<Defaults, f64>(&Latent::new(up)).ic);
    let minus = components(&decode::decode::<Defaults, f64>(&Latent::new(down)).ic);
    for i in 0..15 {
        let want = w[i] * (plus[i] - minus[i]) / (up[k] - down[k]);
        assert_eq!(
            cols[k][i], want,
            "entry ({i}, {k}) of W·J_D is {:e}, the difference at Part 4's step {want:e}",
            cols[k][i]
        );
    }
}

fn far_z() -> [f64; 8] {
    let mut z = ingested(&starts()[2].1).encoded.z.z;
    z[2] = -3.0;
    z
}

#[test]
fn ingest_outputs_decode_jacobian_columns() {
    for k in 0..8 {
        check_decode_column(far_z(), k, f64::EPSILON.cbrt());
    }
}

negative_control!(
    ingest_outputs_decode_jacobian_columns,
    "a difference at twice Part 4's step is not the column W·J_D carries",
    expected = "the difference at Part 4's step",
    check_decode_column(far_z(), 2, 2.0 * f64::EPSILON.cbrt())
);

#[test]
fn ingest_outputs_encode_jacobian_columns() {
    let (raw, _) = light();
    check_column(&raw, 0, 1.0);
    check_column(&raw, 14, 1.0);
    check_column(&starts()[2].1, 4, 1.0);
}

negative_control!(
    ingest_outputs_encode_jacobian_columns,
    "a column not divided by W's entry is not J_E·W⁻¹'s",
    expected = "of J_E·W⁻¹ is",
    check_column(&starts()[2].1, 4, 1.0 / weights(&starts()[2].1)[4])
);

/// `sigma_min` and `sigma_max` on matrices whose singular values are known: a diagonal one, the same with its columns
/// mixed by a rotation (which keeps the singular values), and one with a zero column, where `condition` is `+∞`.
fn check_known(cols: [Phys<f64>; 8], least: f64, most: f64) {
    let got = sigma_min(cols);
    assert!(
        (got - least).abs() <= 1e-14 * least.max(1.0),
        "sigma_min is {got:e}, the matrix's least singular value is {least:e}"
    );
    let got = sigma_max(cols);
    assert!(
        (got - most).abs() <= 1e-14 * most.max(1.0),
        "sigma_max is {got:e}, the matrix's largest singular value is {most:e}"
    );
}

fn diagonal(values: [f64; 8]) -> [Phys<f64>; 8] {
    let mut cols = [[0.0; 15]; 8];
    for (k, v) in values.iter().enumerate() {
        cols[k][2 * k] = *v;
    }
    cols
}

/// `cols` mixed by the rotation through `θ` in the column pair `(p, q)`: `A·R`, the same singular values.
fn mix(mut cols: [Phys<f64>; 8], p: usize, q: usize, theta: f64) -> [Phys<f64>; 8] {
    let (c, s) = (theta.cos(), theta.sin());
    let (left, right) = cols.split_at_mut(q);
    for (u, v) in left[p].iter_mut().zip(right[0].iter_mut()) {
        (*u, *v) = (c * *u + s * *v, -s * *u + c * *v);
    }
    cols
}

#[test]
fn ingest_outputs_sigma_min_on_known_matrices() {
    let values = [3.0, 0.5, 7.0, 0.125, 2.0, 1.0, 4.0, 0.75];
    check_known(diagonal(values), 0.125, 7.0);
    let mut mixed = diagonal(values);
    for (p, q, t) in [
        (0, 3, 0.3),
        (1, 2, 1.1),
        (3, 7, -0.8),
        (4, 5, 2.0),
        (2, 6, 0.4),
        (0, 1, -1.3),
    ] {
        mixed = mix(mixed, p, q, t);
    }
    check_known(mixed, 0.125, 7.0);
    let mut equal = diagonal([2.0; 8]);
    equal = mix(equal, 0, 1, 0.9);
    check_known(equal, 2.0, 2.0);
    let mut singular = diagonal(values);
    singular[5] = [0.0; 15];
    check_known(singular, 0.0, 7.0);
    let mut top = diagonal(values);
    top[0] = [0.0; 15];
    top[0][1] = 9.5;
    check_known(top, 0.125, 9.5);
    assert_eq!(
        condition(singular),
        f64::INFINITY,
        "a rank-deficient matrix's condition is not +∞"
    );
    assert_eq!(
        condition(diagonal(values)),
        8.0,
        "the diagonal matrix's condition is not 1/0.125"
    );
}

negative_control!(
    ingest_outputs_sigma_min_on_known_matrices,
    "a matrix whose least singular value is 0.25 is not read as 0.125",
    expected = "the matrix's least singular value is",
    check_known(
        diagonal([3.0, 0.5, 7.0, 0.25, 2.0, 1.0, 4.0, 0.75]),
        0.125,
        7.0
    )
);

mod largest_misread {
    use super::*;
    negative_control!(
        ingest_outputs_sigma_min_on_known_matrices,
        "a matrix whose largest singular value is 8 is not read as 7",
        expected = "the matrix's largest singular value is",
        check_known(
            diagonal([3.0, 0.5, 8.0, 0.125, 2.0, 1.0, 4.0, 0.75]),
            0.125,
            7.0
        )
    );
}

/// A 15 × 8 matrix of pseudo-random entries in `[−1, 1)` (a 64-bit LCG from a fixed seed), column `j` scaled by
/// `gradeʲ`: a dense matrix that is not orthogonal (its least column norm, 1.82, is well above its `σ_min`, 1.09).
fn dense(grade: f64) -> [Phys<f64>; 8] {
    let mut x: u64 = 12345;
    let mut cols = [[0.0; 15]; 8];
    for (j, col) in cols.iter_mut().enumerate() {
        for c in col.iter_mut() {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *c = ((x >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0) * grade.powi(j as i32);
        }
    }
    cols
}

/// `σ_min` is a property of the matrix, not of its scale or column order: `σ_min(c·A) = c·σ_min(A)` at `c = 10^±20`,
/// where each pair's orthogonality test must scale with the columns, and `σ_min(A·P) = σ_min(A)` for the column
/// reversal `P` on a strongly graded `A` (column `j` scaled by `10^(−3j)`), where the rotation angle must be Jacobi's.
fn check_invariance(scale: f64, grade: f64) {
    let a = dense(1.0);
    let base = sigma_min(a);
    let scaled = sigma_min(a.map(|col| col.map(|c| c * scale)));
    assert!(
        (scaled / (scale * base) - 1.0).abs() <= 1e-12,
        "σ_min of the matrix scaled by {scale:e} is {scaled:e}, not {scale:e} × {base:e}"
    );
    let g = dense(grade);
    let mut reversed = g;
    reversed.reverse();
    let (fwd, back) = (sigma_min(g), sigma_min(reversed));
    assert!(
        (back / fwd - 1.0).abs() <= 1e-12,
        "σ_min of the graded matrix is {fwd:e} and of its columns reversed {back:e}"
    );
}

#[test]
fn ingest_outputs_sigma_min_is_scale_and_order_free() {
    check_invariance(1e-20, 1e-3);
    check_invariance(1e20, 1e-3);
}

negative_control!(
    ingest_outputs_sigma_min_is_scale_and_order_free,
    "σ_min of a matrix scaled by 10⁻²⁰ is not 10⁻¹⁹ times the matrix's",
    expected = "σ_min of the matrix scaled by",
    {
        let a = dense(1.0);
        let scaled = sigma_min(a.map(|col| col.map(|c| c * 1e-20)));
        assert!(
            (scaled / (1e-19 * sigma_min(a)) - 1.0).abs() <= 1e-12,
            "σ_min of the matrix scaled by 1e-20 is {scaled:e}"
        );
    }
);

/// Part 4's difference step `hₖ = ε_R^{1/3}·max(1, |zₖ|)` grows with `|zₖ|`, so a far coordinate still moves the decode:
/// at `z_q0 = 10⁶` a step of `ε_R^{1/3}` alone would vanish in `z_q0`'s rounding and the quotient would be `0/0`.
fn check_far_step(z: [f64; 8], q0: f64) {
    let mut w = z;
    w[2] = q0;
    let cols = weighted_jacobian::<Defaults, f64>(&Latent::new(w));
    for (k, col) in cols.iter().enumerate() {
        assert!(
            col.iter().all(|c| c.is_finite()),
            "column {k} of W·J_D at z_q0 = {q0:e} is not finite: {col:?}"
        );
    }
}

#[test]
fn ingest_outputs_jacobian_step_follows_z() {
    check_far_step(ingested(&starts()[2].1).encoded.z.z, 1e6);
}

negative_control!(
    ingest_outputs_jacobian_step_follows_z,
    "a latent coordinate that is not a number gives no finite Jacobian",
    expected = "is not finite",
    check_far_step(ingested(&starts()[2].1).encoded.z.z, f64::NAN)
);

/// Part 4's step `hₖ = ε_R^{1/3}·max(1, |xₖ|)` grows with `|xₖ|`, so a far component still moves the encode: on a state
/// scaled by `10⁶` a step of `ε_R^{1/3}` alone would vanish in the positions' rounding and the quotient would be
/// `0/0`.
fn check_far_encode_step(x: &Ic<f64>) {
    let cols = weighted_encode_jacobian::<Defaults, f64>(x);
    for (i, col) in cols.iter().enumerate() {
        assert!(
            col.iter().all(|c| c.is_finite()),
            "row {i} of J_E·W⁻¹ is not finite: {col:?}"
        );
    }
}

#[test]
fn ingest_outputs_encode_step_follows_x() {
    check_far_encode_step(&moved(&starts()[2].1, 0.3, [2e6, -1e6], [0.0, 0.0], 1e6));
}

negative_control!(
    ingest_outputs_encode_step_follows_x,
    "a position that is not a number gives no finite Jacobian",
    expected = "is not finite",
    {
        let mut x = starts()[2].1;
        x.r[1][0] = f64::NAN;
        check_far_encode_step(&x);
    }
);

/// The light start of the physics review of PR #294 (P1): masses `(0.05, 0.15, 0.8)`, slow momenta, a foreign frame;
/// and its canonical representative `D(E(x))`, on the section.
fn light() -> (Ic<f64>, Ic<f64>) {
    let raw = Ic {
        m: [0.05, 0.15, 0.8],
        r: [[0.3, 0.1], [-0.9, 0.4], [0.05, -0.08]],
        p: [[0.001, 0.002], [-0.003, 0.001], [0.002, -0.003]],
    };
    let on = decode::decode::<Defaults, f64>(&encode::<Defaults, f64>(&raw).z).ic;
    (raw, on)
}

/// Part 4's bound at `x`: for `x̃ = x + δ·W⁻¹u`, `δ = 10⁻⁶`, along each of the 15 components (each mass alone, so
/// `Σmᵢ` moves, each position and momentum component alone, so the centre of mass, the total momentum, `I` and the
/// orientation move: off the section) and four mixed directions, `‖E(x̃) − z‖₂ ≤ κ·‖W(x̃ − x)‖₂` and `‖E(x̃) − z‖_∞ ≤
/// 3·κ·‖x̃ − x‖_phys`, `κ` as given, to first order (a slack of 10⁻³).
fn check_bound(name: &str, x: &Ic<f64>, k: f64) {
    let z = encode::<Defaults, f64>(x).z.z;
    let c = components(x);
    let w = weights(x);
    let mut dirs: Vec<[f64; 15]> = (0..15)
        .map(|j| std::array::from_fn(|i| if i == j { 1.0 } else { 0.0 }))
        .collect();
    dirs.extend([
        std::array::from_fn(|i| ((i * 7 + 3) % 11) as f64 / 5.0 - 1.0),
        std::array::from_fn(|i| ((i * 5 + 1) % 13) as f64 / 6.0 - 1.0),
        std::array::from_fn(|i| if i % 2 == 0 { 1.0 } else { -1.0 }),
        std::array::from_fn(|i| if i < 3 { 1.0 } else { 0.1 }),
    ]);
    for u in dirs {
        let xt = from_components(&std::array::from_fn(|i| c[i] + 1e-6 * u[i] / w[i]));
        let ct = components(&xt);
        let zt = encode::<Defaults, f64>(&xt).z.z;
        let dz2 = (0..8).map(|i| (zt[i] - z[i]).powi(2)).sum::<f64>().sqrt();
        let dzi = (0..8).map(|i| (zt[i] - z[i]).abs()).fold(0.0, f64::max);
        let wdx = (0..15)
            .map(|i| (w[i] * (ct[i] - c[i])).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            dz2 <= k * wdx * (1.0 + 1e-3),
            "{name}: ‖E(x̃) − z‖₂ = {dz2:e} exceeds κ·‖W(x̃ − x)‖₂ = {:e} along {u:?}",
            k * wdx
        );
        let bound = 3.0 * k * phys(x, &xt);
        assert!(
            dzi <= bound * (1.0 + 1e-3),
            "{name}: ‖E(x̃) − z‖_∞ = {dzi:e} exceeds 3·κ·‖x̃ − x‖_phys = {bound:e} along {u:?}"
        );
    }
}

#[test]
fn ingest_outputs_kappa_bounds_the_z_residual() {
    let (raw, on) = light();
    // Not the figure-eight's published start: it sits at the α = 0 pole, where E is not differentiable (β is not
    // defined, Part 4: κ = +∞), so a move of 10⁻⁶ swings z by O(1) and the central difference's large finite κ bounds
    // nothing; its κ ≥ 10⁵ is checked in check_residual.
    let mut states: Vec<(String, Ic<f64>)> = foreign()
        .into_iter()
        .filter(|(n, _)| !n.starts_with("figure-eight"))
        .collect();
    states.push(("light, foreign".to_owned(), raw));
    states.push(("light, on the section".to_owned(), on));
    for (name, x) in states {
        let got = ingested(&x);
        check_bound(&name, &x, got.kappa);
    }
}

negative_control!(
    ingest_outputs_kappa_bounds_the_z_residual,
    "the section's tangent alone, 1/σ_min(W·J_D), does not bound a move of m₀ off the section (P1)",
    expected = "exceeds κ·‖W(x̃ − x)‖₂",
    {
        let (_, on) = light();
        let z = encode::<Defaults, f64>(&on).z;
        let tangent = condition(weighted_jacobian::<Defaults, f64>(&z));
        check_bound("light, on the section", &on, tangent);
    }
);

mod tenth {
    use super::*;
    negative_control!(
        ingest_outputs_kappa_bounds_the_z_residual,
        "a tenth of κ does not bound the z residual",
        expected = "exceeds",
        {
            let (name, x) = &starts()[2];
            check_bound(name, x, ingested(x).kappa / 10.0);
        }
    );
}

/// Near a clamp `κ` grows as the link's `1/(dv/dz)`: at `z_q0 = z₀`, the `q₀` column of `W·J_D` is the sigmoid's
/// `s(1 − s)` times the column of `∂D/∂q₀`, so `κ ≥ 1/‖W·J_D·e_q0‖`. Checked at `z_q0 = 12` on the Burrau start's `z`,
/// where `s(1 − s) ≈ 6.1e-6`, against `κ` at `z_q0 = 0` (`s(1 − s) = ¼`): `κ` must grow by at least `10⁴`; and every
/// `κ` is at least the reciprocal of each weighted column's norm.
fn check_clamp_growth(z: [f64; 8], factor: f64) {
    let at = |q0: f64| {
        let mut w = z;
        w[2] = q0;
        Latent::new(w)
    };
    let point = |q0: f64| decode::decode::<Defaults, f64>(&at(q0)).ic;
    let (k0, k12) = (
        kappa::<Defaults, f64>(&point(0.0)),
        kappa::<Defaults, f64>(&point(12.0)),
    );
    for q0 in [0.0, 12.0] {
        let cols = weighted_jacobian::<Defaults, f64>(&at(q0));
        let k = kappa::<Defaults, f64>(&point(q0));
        for col in cols {
            let n = col.iter().map(|c| c * c).sum::<f64>().sqrt();
            assert!(
                k * n >= 1.0 - 1e-6,
                "κ = {k:e} is below 1/‖column‖ = {:e}",
                1.0 / n
            );
        }
    }
    assert!(
        k12 >= factor * k0,
        "κ grows from {k0:e} to only {k12:e} toward the q₀ clamp, not by {factor:e}"
    );
}

#[test]
fn ingest_outputs_kappa_grows_toward_a_clamp() {
    let z = ingested(&starts()[2].1).encoded.z.z;
    check_clamp_growth(z, 1e4);
}

negative_control!(
    ingest_outputs_kappa_grows_toward_a_clamp,
    "κ does not grow by 10⁹ toward the clamp: the growth is the link's 1/(s(1 − s)), about 4·10⁴",
    expected = "toward the q₀ clamp",
    check_clamp_growth(ingested(&starts()[2].1).encoded.z.z, 1e9)
);

/// `κ` is the link's: at a state with a near-saturated momentum control, encoding under another registry link for
/// the momenta (`tanh_q` for `sigmoid_q`) changes `κ` (Part 7, "link-fixable"). Deep in saturation the two links'
/// inverse derivatives differ by a factor tending to 2 (measured 1.9999999 here), so the check asks for 1.5.
fn check_link_dependence(x: &Ic<f64>, ratio: f64) {
    let (ks, kt) = (kappa::<Defaults, f64>(x), kappa::<TanhMomenta, f64>(x));
    let r = ks.max(kt) / ks.min(kt);
    assert!(
        r >= ratio,
        "κ under sigmoid_q {ks:e} and under tanh_q {kt:e} differ by {r}, not by {ratio}"
    );
}

/// The Burrau start at rest with body 2 given a large momentum: a free-momentum control near its saturation.
fn kicked() -> Ic<f64> {
    let mut x = ingested(&starts()[2].1).encoded.canonical.ic;
    let z = encode::<Defaults, f64>(&x).z;
    let mut w = z.z;
    w[4] = 9.0;
    x = decode::decode::<Defaults, f64>(&Latent::new(w)).ic;
    x
}

#[test]
fn ingest_outputs_kappa_depends_on_the_link() {
    check_link_dependence(&kicked(), 1.5);
}

negative_control!(
    ingest_outputs_kappa_depends_on_the_link,
    "the two links' κ do not differ by a factor of 10⁶",
    expected = "differ by",
    check_link_dependence(&kicked(), 1e6)
);

/// `W` is Part 4's weight at `x`: 1 on each mass, `1/R_x` on each position component and `1/P_x` on each momentum
/// component, `R_x = max(1, maxⱼ‖rⱼ‖)` and `P_x` likewise; a block whose extent is below 1 is weighted 1.
fn check_weights(x: &Ic<f64>, r_x: f64, p_x: f64) {
    let w = weights(x);
    let mut want = [1.0; 15];
    want[3..9].fill(1.0 / r_x);
    want[9..15].fill(1.0 / p_x);
    assert_eq!(
        w, want,
        "the weights at {x:?} are not 1, 1/R_x = 1/{r_x}, 1/P_x = 1/{p_x}"
    );
    assert_eq!(
        components(x),
        [
            x.m[0], x.m[1], x.m[2], x.r[0][0], x.r[0][1], x.r[1][0], x.r[1][1], x.r[2][0],
            x.r[2][1], x.p[0][0], x.p[0][1], x.p[1][0], x.p[1][1], x.p[2][0], x.p[2][1]
        ],
        "the components are not the masses, positions and momenta in order"
    );
}

/// A state whose largest position has norm 2 (body 2 at `(0, 2)`) and largest momentum norm 4 (body 3's `(4, 0)`), and
/// one whose blocks lie inside the unit ball.
fn extents() -> (Ic<f64>, Ic<f64>) {
    let big = Ic {
        m: [0.2, 0.3, 0.5],
        r: [[0.5, 0.0], [0.0, 2.0], [-1.0, 1.0]],
        p: [[1.0, 1.0], [-2.0, 0.5], [4.0, 0.0]],
    };
    let small = Ic {
        m: [0.2, 0.3, 0.5],
        r: [[0.5, 0.0], [0.0, -0.75], [0.25, 0.25]],
        p: [[0.1, 0.0], [0.0, -0.2], [0.3, 0.4]],
    };
    (big, small)
}

#[test]
fn ingest_outputs_weights_are_part4s() {
    let (big, small) = extents();
    check_weights(&big, 2.0, 4.0);
    check_weights(&small, 1.0, 1.0);
}

negative_control!(
    ingest_outputs_weights_are_part4s,
    "the positions weighted by R_x itself, not 1/R_x, are not Part 4's weight",
    expected = "the weights at",
    check_weights(&extents().0, 0.5, 4.0)
);
