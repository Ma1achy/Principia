//! Ground-truth ingestion (inverse-encode contract Parts 4 and 7; REQ-VAL-014, REQ-ENC-025; TASK-M2-20): foreign
//! states, the Lagrange equilateral, the figure-eight and the Burrau rest start, each in its published frame, are
//! ingested, and the result carries `z`, the physical residual `‖D(z) − C(x)‖_phys` and `κ(z)`.
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
//! it raises `lookup_clamped` and its `κ` is large, as Part 4 says it must), and `κ(z)` is
//! `1/σ_min(W·J_D)`, checked against an independent oracle: the Gram matrix `(W·J_D)ᵀ(W·J_D)` less `σ²·(1 − δ)` is
//! positive definite and less `σ²·(1 + δ)` is not (Cholesky), `δ = 10⁻⁶`; and Part 4's bound `‖E(x̃) − z‖_∞ ≤
//! 3·κ(z)·‖x̃ − x‖_phys` on states `x̃ = D(z + δz)` near `x`.

use kernel::decode::{self, Ic, Latent};
use kernel::encode::encode;
use kernel::encode::ingest::{
    condition, ingest, kappa, sigma_min, weighted_jacobian, Ingested, Phys,
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
/// `κ(z)`, each as its definition gives it.
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
    let k = kappa::<Defaults, f64>(&e.z);
    assert_eq!(got.kappa.to_bits(), k.to_bits(), "{name}: κ is not κ(z)");
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

/// `σ` is the smallest singular value of `cols` to the relative `δ = 10⁻⁶`: `AᵀA − σ²(1 − δ)I` is positive definite
/// and `AᵀA − σ²(1 + δ)I` is not.
fn check_sigma(name: &str, cols: &[Phys<f64>; 8], sigma: f64) {
    let g = gram(cols);
    let shifted = |f: f64| {
        let mut a = g;
        for (i, row) in a.iter_mut().enumerate() {
            row[i] -= sigma * sigma * f;
        }
        a
    };
    assert!(
        positive_definite(&shifted(1.0 - 1e-6)),
        "{name}: σ = {sigma:e} is above the smallest singular value"
    );
    assert!(
        !positive_definite(&shifted(1.0 + 1e-6)),
        "{name}: σ = {sigma:e} is below the smallest singular value"
    );
}

#[test]
fn ingest_outputs_kappa_is_one_over_the_least_singular_value() {
    // The Gram oracle squares the condition, so it resolves σ_min only where κ² is well inside 1/ε: not at the
    // figure-eight's α = 0 pole, where κ ≥ 10⁵ (check_residual).
    for (name, x) in foreign()
        .into_iter()
        .filter(|(n, _)| !n.starts_with("figure-eight"))
    {
        let z = ingested(&x).encoded.z;
        let cols = weighted_jacobian::<Defaults, f64>(&z);
        let sigma = sigma_min(cols);
        check_sigma(&name, &cols, sigma);
        assert_eq!(
            kappa::<Defaults, f64>(&z),
            1.0 / sigma,
            "{name}: κ is not 1/σ_min"
        );
    }
}

negative_control!(
    ingest_outputs_kappa_is_one_over_the_least_singular_value,
    "twice the least singular value is not the least singular value",
    expected = "is above the smallest singular value",
    {
        let (name, x) = &foreign()[3];
        let cols = weighted_jacobian::<Defaults, f64>(&ingested(x).encoded.z);
        check_sigma(name, &cols, 2.0 * sigma_min(cols));
    }
);

mod half_sigma {
    use super::*;
    negative_control!(
        ingest_outputs_kappa_is_one_over_the_least_singular_value,
        "half the least singular value is not the least singular value",
        expected = "is below the smallest singular value",
        {
            let (name, x) = &foreign()[3];
            let cols = weighted_jacobian::<Defaults, f64>(&ingested(x).encoded.z);
            check_sigma(name, &cols, 0.5 * sigma_min(cols));
        }
    );
}

/// `sigma_min` on matrices whose singular values are known: a diagonal one, the same with its columns mixed by a
/// rotation (which keeps the singular values), and one with a zero column, where `condition` is `+∞`.
fn check_known(cols: [Phys<f64>; 8], want: f64) {
    let got = sigma_min(cols);
    assert!(
        (got - want).abs() <= 1e-14 * want.max(1.0),
        "sigma_min is {got:e}, the matrix's least singular value is {want:e}"
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
    check_known(diagonal(values), 0.125);
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
    check_known(mixed, 0.125);
    let mut equal = diagonal([2.0; 8]);
    equal = mix(equal, 0, 1, 0.9);
    check_known(equal, 2.0);
    let mut singular = diagonal(values);
    singular[5] = [0.0; 15];
    check_known(singular, 0.0);
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
    check_known(diagonal([3.0, 0.5, 7.0, 0.25, 2.0, 1.0, 4.0, 0.75]), 0.125)
);

/// Part 4's bound: for `x̃ = D(z + δz)` near `x = D(z)`, `‖E(x̃) − z‖_∞ ≤ 3·κ·‖x̃ − x‖_phys`, `κ` as given, along each
/// control and along four mixed directions.
fn check_bound(name: &str, z: &Latent<f64>, k: f64) {
    let x = decode::decode::<Defaults, f64>(z).ic;
    let mut dirs: Vec<[f64; 8]> = (0..8)
        .map(|j| std::array::from_fn(|i| if i == j { 1.0 } else { 0.0 }))
        .collect();
    dirs.extend([
        [0.3, -0.7, 0.1, 0.5, -0.2, 0.9, -0.4, 0.6],
        [-0.5, 0.2, 0.8, -0.1, 0.6, 0.3, 0.7, -0.9],
        [1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0],
        [0.05, 0.9, -0.3, -0.6, 0.2, 0.1, -0.8, 0.4],
    ]);
    for u in dirs {
        let zt = Latent::new(std::array::from_fn(|i| z.z[i] + 1e-6 * u[i]));
        let xt = decode::decode::<Defaults, f64>(&zt).ic;
        let back = encode::<Defaults, f64>(&xt).z;
        let dz = (0..8)
            .map(|i| (back.z[i] - z.z[i]).abs())
            .fold(0.0, f64::max);
        let bound = 3.0 * k * phys(&x, &xt);
        assert!(
            dz <= bound * (1.0 + 1e-3),
            "{name}: ‖E(x̃) − z‖_∞ = {dz:e} exceeds 3·κ·‖x̃ − x‖_phys = {bound:e} along {u:?}"
        );
    }
}

#[test]
fn ingest_outputs_kappa_bounds_the_z_residual() {
    for (name, x) in starts() {
        let got = ingested(&x);
        check_bound(name, &got.encoded.z, got.kappa);
    }
}

negative_control!(
    ingest_outputs_kappa_bounds_the_z_residual,
    "a tenth of κ does not bound the z residual",
    expected = "exceeds 3·κ·‖x̃ − x‖_phys",
    {
        let (name, x) = &starts()[2];
        let got = ingested(x);
        check_bound(name, &got.encoded.z, got.kappa / 10.0);
    }
);

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
    let (k0, k12) = (
        kappa::<Defaults, f64>(&at(0.0)),
        kappa::<Defaults, f64>(&at(12.0)),
    );
    for zz in [at(0.0), at(12.0)] {
        let cols = weighted_jacobian::<Defaults, f64>(&zz);
        let k = kappa::<Defaults, f64>(&zz);
        for col in cols {
            let n = col.iter().map(|c| c * c).sum::<f64>().sqrt();
            assert!(
                k * n >= 1.0 - 1e-12,
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

/// `κ` is the link's: at a state with a near-saturated momentum control, re-encoding under another registry link for
/// the momenta (`tanh_q` for `sigmoid_q`) changes `κ` (Part 7, "link-fixable").
fn check_link_dependence(x: &Ic<f64>, ratio: f64) {
    let zs = encode::<Defaults, f64>(x).z;
    let zt = encode::<TanhMomenta, f64>(x).z;
    let (ks, kt) = (kappa::<Defaults, f64>(&zs), kappa::<TanhMomenta, f64>(&zt));
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
    check_link_dependence(&kicked(), 2.0);
}

negative_control!(
    ingest_outputs_kappa_depends_on_the_link,
    "the two links' κ do not differ by a factor of 10⁶",
    expected = "differ by",
    check_link_dependence(&kicked(), 1e6)
);
