//! Canonicalisation `C` and the scale gauge on the CPU, at f64 and f32, the one source's two instantiations
//! (`kernel::canon`; TASK-M2-07). The GPU-f32 build's checks are `engine/tests/canon_gpu.rs`.
//! - `canonicalise_section`: on random physical states, the output has CoM at the origin, zero total momentum, `ρ̃` on
//!   `+x`, `λ̃_y ≥ −δ_λ` and `I = 1`, and `C` is idempotent (REQ-DEC-002).
//! - `canonical_noop`: dd_decoder §5 test 5, `C` on canonical-frame decodes of fuzzed and saturated `z` leaves them
//!   unchanged, and at the `λ̃_y = 0` seam gives the same result twice (REQ-DEC-018).
//! - `scale_gauge`: dd_decoder §5 test 10, the identity on canonical output; on a scaled input, `I = 1` and `E`, `L_z`
//!   transformed per the inverse-encode contract's table (REQ-DEC-023).
//! - `scale_step`: dd_encode §5 test 4 on `fixtures/states/burrau_2x.json`: `I_in = 4·I_canon`, `λ = 1/2`,
//!   `E_canon = 2·E_in`, `L_canon = L_in/I_in^{1/4}`, the timing factor `I_in^{3/4}` (REQ-ENC-006).
//! - `canon_mirror_deadband`: the mirror test at the deadband's edge, `λ̃_y = −δ_λ` and one step below it, at each
//!   precision; and `λ̃_y` is the decoder's own `λ̃_y` on canonical decodes.
//! - `canon_mirror_decision_edges`: `C`'s decision ([`mirror_decision`], R-82's test without `fma`, `√`, division or
//!   trig) at the pole, where its arithmetic is exact: no mirror at `|λ̃_y| ≤ δ_λ`, a mirror one step below `−δ_λ`, at
//!   each precision; and off it, `10⁻⁴` of `δ_λ` either side of the edges (REQ-DEC-017).
//! - `canon_mirror_decision_near`: the decision against `λ̃_y < −δ_λ` at f64 on turned, scaled, translated and mirrored
//!   states `1%` of `δ_λ` either side of the deadband's edges, and `C`'s flag on such states at `I_in ≠ 1`
//!   (REQ-DEC-017).
//! - `canon_mirror_extreme_scales`: the decision and `C`'s flag at `λ̃_y = ∓0.577` on states scaled to the edges of
//!   `I_in`'s finite, normal range at each precision (REQ-DEC-017, REQ-DEC-002).
//! - `canon_mirror_scale_invariance`: on 2000 generic states, the decision and `C`'s flag are the same at every
//!   power-of-two scale from `2^−60` to `2^60` at f32 and `2^±500` at f64 (REQ-DEC-017).
//! - `canon_product_fusion_proof`: the f32 product the mirror decision is built from rounds the same fused or not at
//!   each of its contractible sites, on 200 000 pairs (REQ-DEC-017).
//! - `canon_pow2_bits`: the bit-formed powers of two and exponents the decision's integer exponents are read with.
//! - `canon_mirror_exact_terms`: the decision on states whose sums have an exact zero term, at powers of two from
//!   `2^−100` to `2^100` (f32) and `2^±900` (f64), and on states whose sums add terms more than the exponent range
//!   apart, is the one their exact `c` gives (REQ-DEC-017).
//! - `canon_mirror_subnormal_difference`: the decision on an inner pair a subnormal apart is the exact one at f32 and
//!   f64, where subnormals are kept (REQ-DEC-017).
//! - `canon_inertia_extremes`: `I_in` rounds to 0 with the bodies apart, and to `+∞`; at underflow no `1/0` is formed
//!   (RQ-296 holds the outputs).
//! - `canon_rotation`: step 2's turn `(cos φ, sin φ)`, formed as `ρ/‖ρ‖` ([`canon::rotation`]): `(1, +0)` at `ρ = 0`
//!   for every sign of the zeros (R-437), and elsewhere within 2 eps of `ρ/‖ρ‖`, in value and in the direction it
//!   leaves `ρ` in, from the least subnormal to the greatest finite `ρ` at each precision (REQ-DEC-002).
//! - `stage_canon_native`: the compute entry point `kernel::stage::canon::canonicalise`, called natively, writes each
//!   state's `C` in its word layout, notices included, and nothing past the last state.
//!
//! **Tolerances** (applied per R-369: a test decision, as dd_decoder §5 test 1's "precision-appropriate tolerance"),
//! each a multiple of the precision's epsilon, set from the worst case these tests measure and print, with headroom.
//!
//! **Domain** (pitfalls §10): `C` alone, on states held fixed; no trajectory.

use std::path::Path;

use kernel::canon::{
    self, canonicalise, inertia, lambda_t_y, mirror_decision, mirror_taken, Canonical,
};
use kernel::decode::descriptor::{kinetic, potential};
use kernel::decode::{decode, Ic, Latent};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use kernel::real::IeeeBits;
use kernel::stage::canon::{
    canon_words, canonicalise as canonicalise_entry, notices, read_canonical, write_canonical,
};
use kernel::stage::decode::{ic_words, write_ic};
use proptest::prelude::*;
use spirv_std::glam::UVec3;
use spirv_std::num_traits::NumCast;
use validation::decode_f32_eps::SplitMix;
use validation::{negative_control, prop};

/// A float the tests run at: f32 or f64.
trait Precision: LinkReal + PayloadReal + IeeeBits + Into<f64> {}
impl Precision for f32 {}
impl Precision for f64 {}

fn eps<R: Precision>() -> f64 {
    R::epsilon().into()
}

fn cast<R: Precision>(x: f64) -> R {
    <R as NumCast>::from(x).expect("a finite value")
}

fn to64<R: Precision>(ic: &Ic<R>) -> Ic<f64> {
    Ic {
        m: ic.m.map(Into::into),
        r: ic.r.map(|v| v.map(Into::into)),
        p: ic.p.map(|v| v.map(Into::into)),
    }
}

fn from64<R: Precision>(ic: &Ic<f64>) -> Ic<R> {
    Ic {
        m: ic.m.map(cast),
        r: ic.r.map(|v| v.map(cast)),
        p: ic.p.map(|v| v.map(cast)),
    }
}

fn norm(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}

/// `E = K + V` of `ic`, at f64.
fn energy(ic: &Ic<f64>) -> f64 {
    kinetic(ic) + potential(ic)
}

/// `L_z = Σ rᵢ × pᵢ` of `ic`.
fn angular(ic: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| ic.r[i][0] * ic.p[i][1] - ic.r[i][1] * ic.p[i][0])
        .sum()
}

/// `|a − b|` relative to `|b|`, in `eps`; 0 where both are 0.
fn rel(a: f64, b: f64, eps: f64) -> f64 {
    within(a, b, b.abs(), eps)
}

/// `|a − b|` relative to `scale`, in `eps`; 0 where `a = b`, so a zero `scale` asks for equality.
fn within(a: f64, b: f64, scale: f64, eps: f64) -> f64 {
    if a == b {
        0.0
    } else {
        (a - b).abs() / (scale * eps)
    }
}

/// `K + |V|` of `ic`, the scale of its `E`, which can cancel.
fn energy_scale(ic: &Ic<f64>) -> f64 {
    kinetic(ic) + potential(ic).abs()
}

/// `Σ ‖rᵢ‖‖pᵢ‖` of `ic`, the scale of its `L_z`, which can cancel.
fn angular_scale(ic: &Ic<f64>) -> f64 {
    (0..3).map(|i| norm(ic.r[i]) * norm(ic.p[i])).sum()
}

/// The largest component difference of `a` and `b`, positions over `a`'s largest `‖rᵢ‖` and momenta over its largest
/// `‖pᵢ‖` (1 where they are all zero), in `eps`.
fn state_error(a: &Ic<f64>, b: &Ic<f64>, eps: f64) -> f64 {
    let scale = |v: &[[f64; 2]; 3]| {
        let s = v.iter().map(|x| norm(*x)).fold(0.0, f64::max);
        if s == 0.0 {
            1.0
        } else {
            s
        }
    };
    let (sr, sp) = (scale(&a.r), scale(&a.p));
    let mut worst = 0.0f64;
    for i in 0..3 {
        for k in 0..2 {
            worst = worst.max((a.r[i][k] - b.r[i][k]).abs() / (sr * eps));
            worst = worst.max((a.p[i][k] - b.p[i][k]).abs() / (sp * eps));
        }
    }
    worst
}

// ── canonicalise_section (REQ-DEC-002) ──────────────────────────────────────────────────────────────────────────

/// The section's tolerances, in the precision's epsilon: the CoM relative to `Σmᵢ‖rᵢ‖` plus the input's CoM offset
/// in output units, `λ‖R_com‖`, which step 1a subtracts with its rounding; `Σpᵢ` relative to `Σ‖pᵢ‖` plus the input's
/// total momentum in output units, `‖P_tot‖/√λ`, which step 1b subtracts; `ρ_y` relative to
/// `‖r₀‖ + ‖r₁‖`, since step 2 rotates `r₀` and `r₁`, not `ρ`; `I − 1`; and `C ∘ C` against `C`, per [`state_error`].
const COM_EPS: f64 = 4.0;
const SUM_P_EPS: f64 = 4.0;
const RHO_Y_EPS: f64 = 4.0;
const INERTIA_EPS: f64 = 8.0;
const IDEMPOTENT_EPS: f64 = 2.0;

/// The worst of each section measure over a run, in eps: CoM, `Σp`, `ρ_y`, `I − 1`, idempotence.
#[derive(Default, Debug)]
struct Worst([f64; 5]);

/// The section conditions on `out`, each within its tolerance, the worst recorded in `worst`; `offset` is the input's
/// CoM offset and total momentum in output units, `λ‖R_com‖` and `‖P_tot‖/√λ`. `Err` names the first that fails.
fn check_section<R: Precision>(
    out: &Canonical<R>,
    offset: [f64; 2],
    worst: &mut Worst,
) -> Result<(), String> {
    let e = eps::<R>();
    let ic = to64(&out.ic);
    let (m, r, p) = (ic.m, ic.r, ic.p);
    let com = [0, 1].map(|k| (0..3).map(|i| m[i] * r[i][k]).sum::<f64>());
    let size: f64 = (0..3).map(|i| m[i] * norm(r[i])).sum::<f64>() + offset[0];
    let measures = [
        norm(com) / (size * e),
        {
            let sum = [0, 1].map(|k| p[0][k] + p[1][k] + p[2][k]);
            let scale: f64 = p.iter().map(|v| norm(*v)).sum::<f64>() + offset[1];
            if scale == 0.0 {
                norm(sum)
            } else {
                norm(sum) / (scale * e)
            }
        },
        {
            let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
            if rho[0] <= 0.0 {
                return Err(format!("ρ̃ is not on +x: ρ = {rho:?}"));
            }
            rho[1].abs() / ((norm(r[0]) + norm(r[1])) * e)
        },
        (inertia(m, r) - 1.0).abs() / e,
    ];
    let bounds = [COM_EPS, SUM_P_EPS, RHO_Y_EPS, INERTIA_EPS];
    let names = ["the CoM", "Σp", "ρ_y", "I − 1"];
    for k in 0..4 {
        worst.0[k] = worst.0[k].max(measures[k]);
        if measures[k] > bounds[k] {
            return Err(format!(
                "{} is {} eps, over {}",
                names[k], measures[k], bounds[k]
            ));
        }
    }
    let lty = lambda_t_y(out.ic.m, out.ic.r);
    if lty < -<R as ChartConstants>::DELTA_LAMBDA {
        return Err(format!("λ̃_y = {lty:?} < −δ_λ on the output"));
    }
    Ok(())
}

/// `C` applied to its own output `out`, which takes neither the mirror nor the guard, changes it by what its own steps
/// remove, plus at most [`IDEMPOTENT_EPS`] over the rotation's conditioning. The error model, per [`state_error`]'s
/// units (positions over the largest `‖rᵢ‖`, momenta over the largest `‖pᵢ‖`, in eps): the second `C`'s step 1a moves
/// every `rᵢ` by `out`'s CoM residual, `‖R_com‖`; its step 1b moves `pᵢ` by `mᵢ‖P_tot‖`; its step 0 scales `r` by
/// `I^{−1/2}` and `p` by `I^{1/4}`, `|I − 1|/2` and `|I − 1|/4` at most; and its step 2 turns by the residual
/// `ρ_y/ρ_x`, at most `ρ_y/‖ρ‖` of each vector, with its own rounding. The positions' change less the CoM and scale
/// terms, and the momenta's less the `Σp` and scale terms, each over the conditioning `1 + (‖r₀‖ + ‖r₁‖)/‖ρ‖`. The first three are `out`'s own residuals, each bounded by [`check_section`]; with a large
/// input offset (the CoM far from the bodies) the CoM's can be several eps of the output's size.
fn check_idempotent<R: Precision>(out: &Canonical<R>, worst: &mut Worst) -> Result<(), String> {
    let again = canonicalise(&out.ic);
    if again.mirrored || again.clamped {
        return Err(format!(
            "C(C(x)) mirrored {} clamped {}",
            again.mirrored, again.clamped
        ));
    }
    let e = eps::<R>();
    let ic = to64(&out.ic);
    let largest = |v: &[[f64; 2]; 3]| v.iter().map(|x| norm(*x)).fold(0.0, f64::max);
    let (sr, sp) = (largest(&ic.r), largest(&ic.p));
    let com = [0, 1].map(|k| (0..3).map(|i| ic.m[i] * ic.r[i][k]).sum::<f64>());
    let total = [0, 1].map(|k| (0..3).map(|i| ic.p[i][k]).sum::<f64>());
    let m_max = ic.m.iter().copied().fold(0.0, f64::max);
    let shift_r = norm(com) / (sr * e);
    let shift_p = if sp == 0.0 {
        0.0
    } else {
        m_max * norm(total) / (sp * e)
    };
    let shrink = (inertia(ic.m, ic.r) - 1.0).abs() / e;
    let rho = norm([ic.r[1][0] - ic.r[0][0], ic.r[1][1] - ic.r[0][1]]);
    let conditioning = 1.0 + (norm(ic.r[0]) + norm(ic.r[1])) / rho;
    let again = to64(&again.ic);
    let part = |a: &[[f64; 2]; 3], b: &[[f64; 2]; 3], size: f64| {
        let size = if size == 0.0 { 1.0 } else { size };
        (0..3)
            .flat_map(|i| (0..2).map(move |k| (a[i][k] - b[i][k]).abs()))
            .fold(0.0, f64::max)
            / (size * e)
    };
    let (change_r, change_p) = (part(&ic.r, &again.r, sr), part(&ic.p, &again.p, sp));
    // Each part less its own steps' shifts: `r` by the CoM and `|I − 1|/2`, `p` by `mᵢ·Σp` and `|I − 1|/4`.
    let err_r = (change_r - shift_r - shrink / 2.0).max(0.0) / conditioning;
    let err_p = (change_p - shift_p - shrink / 4.0).max(0.0) / conditioning;
    let (change, err) = (change_r.max(change_p), err_r.max(err_p));
    worst.0[4] = worst.0[4].max(err);
    if err > IDEMPOTENT_EPS {
        return Err(format!(
            "C(C(x)) differs from C(x) by {change} eps, {err} eps over the model (positions {err_r}, momenta \
             {err_p}; CoM shift {shift_r}, Σp shift {shift_p}, |I − 1| {shrink}, conditioning {conditioning})"
        ));
    }
    Ok(())
}

/// A random physical state: masses on the simplex (each at least 0.05 before normalising), positions and momenta in
/// `[−10, 10]`, then a similarity scale `s = 10^t`, `t ∈ [−3, 3]`: `r·s`, `p·s^{−1/2}`.
fn states() -> impl Strategy<Value = Ic<f64>> {
    (
        proptest::array::uniform3(0.05f64..1.0),
        proptest::array::uniform6(-10.0f64..10.0),
        proptest::array::uniform6(-10.0f64..10.0),
        -3.0f64..3.0,
    )
        .prop_map(|(w, r, p, t)| {
            let total: f64 = w.iter().sum();
            let s = 10f64.powf(t);
            let q = s.powf(-0.5);
            Ic {
                m: w.map(|x| x / total),
                r: [
                    [r[0] * s, r[1] * s],
                    [r[2] * s, r[3] * s],
                    [r[4] * s, r[5] * s],
                ],
                p: [
                    [p[0] * q, p[1] * q],
                    [p[2] * q, p[3] * q],
                    [p[4] * q, p[5] * q],
                ],
            }
        })
}

fn section_at<R: Precision>() -> Worst {
    let worst = std::cell::RefCell::new(Worst::default());
    prop::run(&states(), |x| {
        let x = from64::<R>(&x);
        let out = canonicalise(&x);
        let x64 = to64(&x);
        let lambda: f64 = out.lambda.into();
        let com = [0, 1].map(|k| (0..3).map(|i| x64.m[i] * x64.r[i][k]).sum::<f64>());
        let total = [0, 1].map(|k| (0..3).map(|i| x64.p[i][k]).sum::<f64>());
        let offset = [norm(com) * lambda, norm(total) / lambda.sqrt()];
        let mut w = worst.borrow_mut();
        check_section(&out, offset, &mut w).map_err(TestCaseError::fail)?;
        check_idempotent(&out, &mut w).map_err(TestCaseError::fail)?;
        Ok(())
    });
    worst.into_inner()
}

#[test]
fn canonicalise_section() {
    for (name, worst) in [("f64", section_at::<f64>()), ("f32", section_at::<f32>())] {
        eprintln!(
            "canonicalise_section {name}: worst CoM, Σp, ρ_y, I − 1, idempotence (eps) = {:?}; {}",
            worst.0,
            prop::cases_status()
        );
    }
}

negative_control!(
    canonicalise_section,
    "an output turned off the x axis by 0.1 rad must fail the section's ρ̃ on +x",
    expected = "ρ_y is",
    {
        let x = Ic {
            m: [0.2, 0.3, 0.5],
            r: [[1.0, 2.0], [-0.5, 0.25], [0.3, -1.5]],
            p: [[0.1, 0.2], [-0.3, 0.4], [0.2, -0.1]],
        };
        let mut out = canonicalise(&x);
        let (s, c) = 0.1f64.sin_cos();
        for v in out.ic.r.iter_mut().chain(out.ic.p.iter_mut()) {
            *v = [c * v[0] - s * v[1], s * v[0] + c * v[1]];
        }
        check_section(&out, [0.0; 2], &mut Worst::default()).unwrap()
    }
);

// ── canonical_noop (REQ-DEC-018) ────────────────────────────────────────────────────────────────────────────────

/// The no-op's tolerance, per [`state_error`], in eps: what step 1a's CoM and step 0's rescale move by rounding.
const NOOP_EPS: f64 = 4.0;

/// The `z` the no-op runs on: 4096 fuzzed on `[−4, 4]⁸`; each component alone at `±30` and `±1000`, the others fuzzed
/// on `[−2, 2]`; and `(z_α, z_β)` together at every pair of `{±30, ±1000}`, the rest fuzzed.
fn noop_z() -> Vec<[f32; 8]> {
    let mut rng = SplitMix::new(11);
    let mut out: Vec<[f32; 8]> = (0..4096)
        .map(|_| core::array::from_fn(|_| rng.uniform(-4.0, 4.0)))
        .collect();
    let sat = [-1000.0f32, -30.0, 30.0, 1000.0];
    for k in 0..8 {
        for s in sat {
            let mut z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-2.0, 2.0));
            z[k] = s;
            out.push(z);
        }
    }
    for a in sat {
        for b in sat {
            let mut z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-2.0, 2.0));
            z[0] = a;
            z[1] = b;
            out.push(z);
        }
    }
    out
}

/// Test 5 on one decode `x` and `C(x)`: the mirror not taken, the rotation angle 0, and the state unchanged within
/// [`NOOP_EPS`]; the measured error.
fn check_noop<R: Precision>(x: &Ic<R>, out: &Canonical<R>, at: &str) -> f64 {
    assert!(!out.mirrored, "the mirror was taken at {at}");
    assert!(
        out.phi == R::zero(),
        "the rotation angle is {:?}, not 0, at {at}",
        out.phi
    );
    let err = state_error(&to64(x), &to64(&out.ic), eps::<R>());
    assert!(err <= NOOP_EPS, "C moved the decode by {err} eps at {at}");
    err
}

fn noop_at<R: Precision>() {
    let (mut worst, mut seam, mut clamped) = (0.0f64, 0, 0);
    for (j, z) in noop_z().into_iter().enumerate() {
        let x = decode::<Defaults, R>(&Latent::new(z.map(|v| cast::<R>(v as f64)))).ic;
        let out = canonicalise(&x);
        let at = format!("{} z = {z:?} (sample {j})", R::NAME);
        worst = worst.max(check_noop(&x, &out, &at));
        clamped += out.clamped as usize;
        if Into::<f64>::into(out.lambda_t_y).abs() <= Into::<f64>::into(R::DELTA_LAMBDA) {
            seam += 1;
            assert_eq!(
                canonicalise(&x),
                out,
                "the seam's tie gave two results at {at}"
            );
        }
    }
    assert!(seam > 0, "no {} decode reached the λ̃_y = 0 seam", R::NAME);
    eprintln!(
        "canonical_noop {}: worst state change {worst} eps (tolerance {NOOP_EPS}); {seam} decodes at the seam; \
         {clamped} at ρ = 0 (lookup_clamped)",
        R::NAME
    );
}

#[test]
fn canonical_noop() {
    noop_at::<f64>();
    noop_at::<f32>();
}

negative_control!(
    canonical_noop,
    "a C that mirrors the seam (a test reading λ̃_y ≤ δ_λ, the deadband's sign reversed) must fail test 5",
    expected = "the mirror was taken",
    {
        let x = decode::<Defaults, f64>(&Latent::new([0.3, -30.0, 0.1, 0.2, -0.4, 0.5, 0.2, -0.1])).ic;
        let mut out = canonicalise(&x);
        assert!(out.lambda_t_y.abs() <= f64::DELTA_LAMBDA, "not at the seam");
        out.mirrored = out.lambda_t_y <= f64::DELTA_LAMBDA;
        check_noop(&x, &out, "the seam");
    }
);

// ── scale_gauge (REQ-DEC-023) ───────────────────────────────────────────────────────────────────────────────────

/// The scale gauge's tolerances, in eps: the identity on canonical output, per [`state_error`]; `I − 1` on a scaled
/// input; `E` and `L_z` against the table, relative.
const GAUGE_ID_EPS: f64 = 4.0;
const GAUGE_I_EPS: f64 = 8.0;
const GAUGE_TABLE_EPS: f64 = 8.0;

/// Test 10's table on `x` and `g = scale_gauge(x)`: `I(g) = 1`, `E(g) = √I·E(x)`, `L_z(g) = L_z(x)/I^{1/4}`, `I = I(x)`;
/// the worst of the `E` and `L_z` rows' errors, in eps.
fn check_table<R: Precision>(x: &Ic<R>, g: &Ic<R>, at: &str) -> f64 {
    let e = eps::<R>();
    let (x, g) = (to64(x), to64(g));
    let i = inertia(x.m, x.r);
    let ig = inertia(g.m, g.r);
    assert!(
        (ig - 1.0).abs() / e <= GAUGE_I_EPS,
        "I = {ig} after the scale gauge at {at}"
    );
    let de = within(energy(&g), i.sqrt() * energy(&x), energy_scale(&g), e);
    assert!(
        de <= GAUGE_TABLE_EPS,
        "E_canon is {de} eps from √I·E_in at {at}"
    );
    let dl = within(
        angular(&g),
        angular(&x) / i.powf(0.25),
        angular_scale(&g),
        e,
    );
    assert!(
        dl <= GAUGE_TABLE_EPS,
        "L_canon is {dl} eps from L_in/I^(1/4) at {at}"
    );
    de.max(dl)
}

fn gauge_at<R: Precision>() {
    let mut rng = SplitMix::new(5);
    let (mut worst, mut table) = (0.0f64, 0.0f64);
    for j in 0..512 {
        let z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-3.0, 3.0));
        let x = decode::<Defaults, R>(&Latent::new(z.map(|v| cast::<R>(v as f64)))).ic;
        let err = state_error(&to64(&x), &to64(&canon::scale_gauge(&x)), eps::<R>());
        worst = worst.max(err);
        assert!(
            err <= GAUGE_ID_EPS,
            "the scale gauge moved canonical output by {err} eps at {} z = {z:?}",
            R::NAME
        );
        let s = 10f64.powf(rng.uniform(-2.0, 2.0) as f64);
        let mut scaled = to64(&x);
        for v in scaled.r.iter_mut() {
            *v = v.map(|c| c * s);
        }
        for v in scaled.p.iter_mut() {
            *v = v.map(|c| c / s.sqrt());
        }
        let scaled = from64::<R>(&scaled);
        table = table.max(check_table(
            &scaled,
            &canon::scale_gauge(&scaled),
            &format!("{} z = {z:?}, s = {s} (sample {j})", R::NAME),
        ));
    }
    eprintln!(
        "scale_gauge {}: worst change on canonical output {worst} eps (tolerance {GAUGE_ID_EPS}); worst E, L_z row \
         {table} eps (tolerance {GAUGE_TABLE_EPS})",
        R::NAME
    );
}

#[test]
fn scale_gauge() {
    gauge_at::<f64>();
    gauge_at::<f32>();
}

negative_control!(
    scale_gauge,
    "a scale gauge that divides the momenta by √ℓ, the s^{+1/2} error dd_encode §3.3 names, must fail the E row",
    expected = "E_canon is",
    {
        let x = decode::<Defaults, f64>(&Latent::new([0.2, 0.4, 0.5, -0.3, 0.2, 0.6, 0.1, -0.2])).ic;
        let mut scaled = x;
        for v in scaled.r.iter_mut() {
            *v = v.map(|c| c * 9.0);
        }
        let mut g = canon::scale_gauge(&scaled);
        for v in g.p.iter_mut() {
            *v = v.map(|c| c / 3.0);
        }
        check_table(&scaled, &g, "momenta scaled by 1/√ℓ");
    }
);

// ── scale_step (REQ-ENC-006) ────────────────────────────────────────────────────────────────────────────────────

/// `fixtures/states/burrau_2x.json`: its state and its stated `I_in` and scale.
fn burrau_2x() -> (Ic<f64>, f64, f64) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/states/burrau_2x.json");
    let text = std::fs::read_to_string(&path).expect("fixtures/states/burrau_2x.json");
    let j: serde_json::Value = serde_json::from_str(&text).expect("the fixture's JSON");
    let num = |v: &serde_json::Value| v.as_f64().expect("a number");
    let pair = |v: &serde_json::Value| [num(&v[0]), num(&v[1])];
    let ic = Ic {
        m: [num(&j["m"][0]), num(&j["m"][1]), num(&j["m"][2])],
        r: [pair(&j["r"][0]), pair(&j["r"][1]), pair(&j["r"][2])],
        p: [pair(&j["p"][0]), pair(&j["p"][1]), pair(&j["p"][2])],
    };
    (ic, num(&j["i_in"]), num(&j["scale"]))
}

/// The scale step's tolerance, relative, in eps.
const STEP_EPS: f64 = 8.0;

/// Encode test 4's row on the input `x` and `c = C(x)`: `I_in = 4·I_canon` with `I_canon = 1`, `λ = 1/2` carried,
/// `E_canon = 2·E_in = √I_in·E_in`, `L_canon = L_in/I_in^{1/4}`, and the timing factor `I_in^{3/4} = 2^{3/2}`.
fn check_row<R: Precision>(x: &Ic<R>, c: &Canonical<R>, at: &str) -> f64 {
    let e = eps::<R>();
    let (x, out) = (to64(x), to64(&c.ic));
    let i_in: f64 = c.i_in.into();
    let i_canon = inertia(out.m, out.r);
    let lambda: f64 = c.lambda.into();
    let checks = [
        ("I_in = 4", rel(i_in, 4.0, e)),
        ("I_in = 4·I_canon", rel(i_in, 4.0 * i_canon, e)),
        ("λ = 1/2", rel(lambda, 0.5, e)),
        ("λ = I_in^(−1/2)", rel(lambda, i_in.powf(-0.5), e)),
        (
            "E_canon = 2·E_in",
            within(energy(&out), 2.0 * energy(&x), energy_scale(&out), e),
        ),
        (
            "E_canon = √I_in·E_in",
            within(
                energy(&out),
                i_in.sqrt() * energy(&x),
                energy_scale(&out),
                e,
            ),
        ),
        (
            "L_canon = L_in/I_in^(1/4)",
            within(
                angular(&out),
                angular(&x) / i_in.powf(0.25),
                angular_scale(&out),
                e,
            ),
        ),
        (
            "the timing factor I_in^(3/4) = 2^(3/2)",
            rel(i_in.powf(0.75), 2f64.powf(1.5), e),
        ),
        (
            "the timing factor = λ^(−3/2)",
            rel(i_in.powf(0.75), lambda.powf(-1.5), e),
        ),
    ];
    let mut worst = 0.0f64;
    for (what, err) in checks {
        assert!(err <= STEP_EPS, "{what}: {err} eps off at {at}");
        worst = worst.max(err);
    }
    worst
}

/// `x` with momenta of zero sum and non-zero `L_z`, so the `E` and `L_z` rows carry kinetic energy and an angular
/// momentum.
fn moving(x: &Ic<f64>) -> Ic<f64> {
    Ic {
        p: [[0.1, -0.2], [-0.3, 0.05], [0.2, 0.15]],
        ..*x
    }
}

fn step_at<R: Precision>() {
    let (x, i_stated, scale) = burrau_2x();
    assert_eq!(
        x.m,
        [5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0],
        "the fixture's masses are not (5,4,3)/12"
    );
    assert_eq!((i_stated, scale), (4.0, 2.0), "the fixture's stated I_in");
    for (what, x) in [("at rest", x), ("moving", moving(&x))] {
        let xr = from64::<R>(&x);
        let c = canonicalise(&xr);
        let at = format!("{} {what}", R::NAME);
        assert!(!c.mirrored && !c.clamped, "a notice besides λ at {at}");
        let worst = check_row(&xr, &c, &at);
        eprintln!("scale_step {at}: λ = {:?}, I_in = {:?}; worst row error {worst} eps (tolerance {STEP_EPS})", c.lambda, c.i_in);
    }
}

#[test]
fn scale_step() {
    step_at::<f64>();
    step_at::<f32>();
}

negative_control!(
    scale_step,
    "a step 0 that scales momenta by I^(−1/4), the s^(+1/2) error, must fail the E row",
    expected = "E_canon = 2·E_in",
    {
        let (x, _, _) = burrau_2x();
        let x = moving(&x);
        let mut c = canonicalise(&x);
        for v in c.ic.p.iter_mut() {
            *v = v.map(|q| q / 2.0);
        }
        check_row(&x, &c, "momenta scaled by I^(−1/4)");
    }
);

// ── stage_canon_native ──────────────────────────────────────────────────────────────────────────────────────────

/// States reaching each pair of notices: none, `lookup_mirrored`, `lookup_clamped`, and both.
fn notice_states() -> [Ic<f32>; 4] {
    let plain = Ic {
        m: [0.25f32, 0.25, 0.5],
        r: [[-1.0, -0.5], [1.0, -0.5], [0.0, 0.5]],
        p: [[0.1, 0.2], [-0.3, 0.1], [0.1, -0.15]],
    };
    let flip = |mut x: Ic<f32>| {
        for v in x.r.iter_mut() {
            v[1] = -v[1];
        }
        x
    };
    let pair = |mut x: Ic<f32>| {
        x.r[1] = x.r[0];
        x
    };
    [plain, flip(plain), pair(plain), pair(flip(plain))]
}

/// The entry point over `states`, invoked once more than there are states, into a buffer of `u32::MAX`; each state's
/// words read back as its `C`, its notices word as [`notices`] gives, and the words past the last state untouched.
fn check_stage(states: &[Ic<f32>], entry: fn(UVec3, &[u32], &mut [u32])) {
    let mut input = vec![0; states.len() * ic_words()];
    for (i, x) in states.iter().enumerate() {
        write_ic(&mut input, i, x);
    }
    let mut out = vec![u32::MAX; (states.len() + 1) * canon_words()];
    for i in 0..=states.len() {
        entry(UVec3::new(i as u32, 0, 0), &input, &mut out);
    }
    for (i, x) in states.iter().enumerate() {
        let want = canonicalise(x);
        assert_eq!(read_canonical(&out, i), want, "state {i}'s C");
        assert_eq!(
            out[i * canon_words() + ic_words() + 4],
            notices(&want),
            "state {i}'s notices word"
        );
    }
    assert!(
        out[states.len() * canon_words()..]
            .iter()
            .all(|&w| w == u32::MAX),
        "the entry wrote past the last state"
    );
}

#[test]
fn stage_canon_native() {
    assert_eq!(canon_words(), ic_words() + 5, "the words of a Canonical");
    let states = notice_states();
    let flags: Vec<(bool, bool)> = states
        .iter()
        .map(|x| {
            let c = canonicalise(x);
            (c.clamped, c.mirrored)
        })
        .collect();
    assert_eq!(
        flags,
        [(false, false), (false, true), (true, false), (true, true)],
        "the states do not reach each pair of notices"
    );
    assert_eq!(
        states.map(|x| notices(&canonicalise(&x))),
        [0, 2, 1, 3],
        "the notices word: bit 0 clamped, bit 1 mirrored"
    );
    check_stage(&states, canonicalise_entry);
    // A Canonical written at sample 1 of three reads back there, its neighbours untouched.
    let c = canonicalise(&states[3]);
    let mut words = vec![7u32; 3 * canon_words()];
    write_canonical(&mut words, 1, &c);
    assert_eq!(read_canonical(&words, 1), c, "sample 1 read back");
    assert!(
        words[..canon_words()]
            .iter()
            .chain(&words[2 * canon_words()..])
            .all(|&w| w == 7),
        "writing sample 1 touched another sample"
    );
}

negative_control!(
    stage_canon_native,
    "an entry that writes every invocation's output, past the last state too, must fail",
    expected = "the entry wrote past the last state",
    {
        let states = notice_states();
        check_stage(&states, |id, input, out| {
            let i = id.x as usize;
            let x = kernel::stage::decode::read_ic(input, i.min(input.len() / ic_words() - 1));
            write_canonical(out, i, &canonicalise(&x));
        })
    }
);

// ── canon_mirror_deadband (REQ-DEC-017) ─────────────────────────────────────────────────────────────────────────

/// The tolerance on `λ̃_y` against the decoder's, absolute (`|λ̃| ≤ 1`), in eps.
const LAMBDA_T_EPS: f64 = 4.0;

/// The mirror test at `−δ_λ` (no mirror) and one step below (mirror), at both edges; and on decodes with unequal
/// masses, `C`'s branch input against the decoder's `λ̃_y = sin α·sin β`, `test` reading `λ̃_y` of `r` and `m`.
fn check_deadband<R: Precision>(test: fn([R; 3], [[R; 2]; 3]) -> R, down: fn(R) -> R) {
    let d = <R as ChartConstants>::DELTA_LAMBDA;
    assert!(!mirror_taken(-d), "{}: mirrored at λ̃_y = −δ_λ", R::NAME);
    assert!(
        mirror_taken(down(-d)),
        "{}: no mirror just below −δ_λ",
        R::NAME
    );
    assert!(!mirror_taken(d), "{}: mirrored at λ̃_y = +δ_λ", R::NAME);
    let mut rng = SplitMix::new(31);
    for j in 0..256 {
        let z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-2.0, 2.0));
        let x = decode::<Defaults, R>(&Latent::new(z.map(|v| cast::<R>(v as f64))));
        let got: f64 = test(x.ic.m, x.ic.r).into();
        let want: f64 = x.config.lambda_t[1].into();
        assert!(
            (got - want).abs() <= LAMBDA_T_EPS * eps::<R>(),
            "{}: λ̃_y = {got}, the decoder's {want}, at sample {j}",
            R::NAME
        );
    }
}

/// [`mirror_decision`]'s signature: masses, CoM-frame positions before step 0; whether to mirror.
type Decide<R> = fn([R; 3], [[R; 2]; 3]) -> bool;

/// An edge state, masses `(¼, ¼, ½)`, its CoM 0:
/// - at the pole, `x₀ = x₁ = (−1, a)`, `x₂ = (1, −a)`: `ρ = 0`, so step 2 is the identity, `Λ = (1, −a)`, `I = 1`
///   exactly and `λ̃_y = −a`. Every operation of the decision is exact on it but `a²` and `δ_λ²`, so its sides compare
///   `a²/2` with `δ_λ²/2`, each one rounding: the deadband's edge exactly, at each precision;
/// - on `+x`, `x₀ = (−1, a)`, `x₁ = (1, a)`, `x₂ = (0, −a)`: `ρ = (2, 0)`, `Λ = (0, −a)`, `I = ½` exactly and
///   `λ̃_y = −√2·a`, which no float `a` puts on `−δ_λ`; the cases there are a set fraction either side.
fn edge<R: Precision>(pole: bool, a: R) -> ([R; 3], [[R; 2]; 3]) {
    let (q, h, one, z) = (cast::<R>(0.25), cast::<R>(0.5), R::one(), R::zero());
    let x = if pole {
        [[-one, a], [-one, a], [one, -a]]
    } else {
        [[-one, a], [one, a], [z, -a]]
    };
    ([q, q, h], x)
}

/// The decision at the deadband's edges, `λ̃_y = −a` at the pole: no mirror at `a = ±δ_λ`, one step inside, `±0`, and
/// `−2δ_λ`; a mirror one step outside `−δ_λ` and at `2δ_λ`. On `+x`, `λ̃_y = −√2·a`: no mirror at `√2·a = δ_λ(1 − 10⁻⁴)`,
/// a mirror at `δ_λ(1 + 10⁻⁴)`.
fn check_decision_edges<R: Precision + std::fmt::LowerExp>(
    decide: Decide<R>,
    up: fn(R) -> R,
    down: fn(R) -> R,
) {
    let d = <R as ChartConstants>::DELTA_LAMBDA;
    let pole = [
        (d, false),
        (up(d), true),
        (down(d), false),
        (d + d, true),
        (-d, false),
        (-up(d), false),
        (-(d + d), false),
        (R::zero(), false),
        (-R::zero(), false),
    ];
    let at = |k: f64| cast::<R>(f64::DELTA_LAMBDA * k / 2f64.sqrt());
    let axis = [
        (at(1.0 - 1e-4), false),
        (at(1.0 + 1e-4), true),
        (at(2.0), true),
        (at(-1.0 - 1e-4), false),
        (R::zero(), false),
    ];
    for (on_pole, cases) in [(true, &pole[..]), (false, &axis[..])] {
        for &(a, want) in cases {
            let (m, x) = edge(on_pole, a);
            assert_eq!(
                decide(m, x),
                want,
                "{}: the decision at a = {a:e}, at the pole {on_pole}, against λ̃_y < −δ_λ",
                R::NAME
            );
        }
    }
}

/// A state whose `λ̃_y` is `t` exactly in the reals, under masses `m`: built on the section (`ρ̃ = (√(1 − 0.36 − t²), 0)`,
/// `λ̃ = (0.6, t)`, `I = 1`), then mirrored or not, turned by `θ`, scaled by `s` and translated by `(2s, −s)`, at f64; its
/// CoM-frame positions as the test forms them, and their `I`. `λ̃_y` of the gauged state is `t`, or `−t` where mirrored.
fn gauged(m: [f64; 3], t: f64, mirror: bool, theta: f64, s: f64) -> [[f64; 2]; 3] {
    let m01 = m[0] + m[1];
    let (mu_rho, mu_lambda) = (m[0] * m[1] / m01, m[2] * m01);
    let rho = [(1.0 - 0.36 - t * t).sqrt() / mu_rho.sqrt(), 0.0];
    let lam = [0.6 / mu_lambda.sqrt(), t / mu_lambda.sqrt()];
    let place = |a: f64, b: f64| [a * rho[0] + b * lam[0], a * rho[1] + b * lam[1]];
    let r = [
        place(-m[1] / m01, -m[2]),
        place(m[0] / m01, -m[2]),
        place(0.0, m01),
    ];
    let (sn, cs) = theta.sin_cos();
    r.map(|v| {
        let y = if mirror { -v[1] } else { v[1] };
        [
            s * (cs * v[0] - sn * y) + 2.0 * s,
            s * (sn * v[0] + cs * y) - s,
        ]
    })
}

/// The decision against R-82's `λ̃_y < −δ_λ` on f64 states whose `λ̃_y` is a set fraction of `δ_λ` off the deadband's
/// edges, `±1%` and more, under masses with `m₂/M₀₁` of 1.5 and of 3/7, turned, scaled, translated and mirrored.
fn check_decision_near(decide: Decide<f64>) {
    let d = f64::DELTA_LAMBDA;
    for m in [[0.15, 0.25, 0.6], [0.35, 0.35, 0.3]] {
        for k in [-2.0, -1.01, -0.99, -0.5, 0.0, 0.5, 0.99, 1.01, 2.0] {
            for mirror in [false, true] {
                for theta in [0.4, 2.2, -1.7] {
                    for s in [0.5, 3.0] {
                        let y = gauged(m, k * d, mirror, theta, s);
                        let com = canon::centre_of_mass(m, y);
                        let x = y.map(|v| [v[0] - com[0], v[1] - com[1]]);
                        let want = if mirror { -k } else { k } < -1.0;
                        assert_eq!(
                            decide(m, x),
                            want,
                            "the decision at λ̃_y = {}δ_λ, masses {m:?}, θ {theta}, s {s}, mirrored {mirror}, \
                             against λ̃_y < −δ_λ",
                            if mirror { -k } else { k }
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn canon_mirror_deadband() {
    check_deadband::<f64>(lambda_t_y, f64::next_down);
    check_deadband::<f32>(lambda_t_y, f32::next_down);
}

negative_control!(
    canon_mirror_deadband,
    "a branch input that reads λ_y, not λ̃_y = √μ_λ·λ_y, must fail",
    expected = "the decoder's",
    check_deadband::<f64>(
        |m, r| lambda_t_y(m, r) / (m[2] * (m[0] + m[1])).sqrt(),
        f64::next_down
    )
);

/// The decision at f64 in plain arithmetic, unrescaled, for the controls: `m₂` and `M₀₁` exchanged where `swap`
/// (R-82's threshold scaled by `m₂/M₀₁`), and the tie `m₂c² = δ_λ²M₀₁‖ρ‖²d²` taken as a mirror where `tie`.
#[cfg(feature = "controls")]
fn defective(m: [f64; 3], x: [[f64; 2]; 3], swap: bool, tie: bool) -> bool {
    let rho = [x[1][0] - x[0][0], x[1][1] - x[0][1]];
    let rho = if rho == [0.0, 0.0] { [1.0, 0.0] } else { rho };
    let m01 = m[0] + m[1];
    let big = |k: usize| m01 * x[2][k] - m[0] * x[0][k] - m[1] * x[1][k];
    let c = rho[0] * big(1) - rho[1] * big(0);
    let (lhs, rhs) = if swap { (m01, m[2]) } else { (m[2], m01) };
    let (left, right) = (
        c * c * lhs,
        f64::DELTA_LAMBDA
            * f64::DELTA_LAMBDA
            * rhs
            * (rho[0] * rho[0] + rho[1] * rho[1])
            * inertia(m, x),
    );
    c < 0.0 && if tie { left >= right } else { left > right }
}

/// `C` itself at f64 on [`gauged`]'s states, with momenta, scaled so `I_in = s²` is not 1: its mirror flag against
/// `λ̃_y < −δ_λ`.
fn check_canonicalise_near() {
    let d = f64::DELTA_LAMBDA;
    for k in [-1.01, -0.99, 0.99, 1.01] {
        for mirror in [false, true] {
            for s in [0.5, 3.0] {
                let m = [0.15, 0.25, 0.6];
                let ic = Ic {
                    m,
                    r: gauged(m, k * d, mirror, 0.9, s),
                    p: [[0.2, -0.1], [-0.3, 0.4], [0.05, 0.1]],
                };
                let want = if mirror { -k } else { k } < -1.0;
                assert_eq!(
                    canonicalise(&ic).mirrored,
                    want,
                    "C's mirror at λ̃_y = {}δ_λ, s {s}, mirrored {mirror}, against λ̃_y < −δ_λ",
                    if mirror { -k } else { k }
                );
            }
        }
    }
}

#[test]
fn canon_mirror_decision_near() {
    check_decision_near(mirror_decision);
    check_canonicalise_near();
}

negative_control!(
    canon_mirror_decision_near,
    "a decision with m₂ and M₀₁ exchanged, R-82's threshold scaled by m₂/M₀₁, must fail near the edge",
    expected = "against λ̃_y < −δ_λ",
    check_decision_near(|m, x| defective(m, x, true, false))
);

#[test]
fn canon_mirror_decision_edges() {
    check_decision_edges::<f64>(mirror_decision, f64::next_up, f64::next_down);
    check_decision_edges::<f32>(mirror_decision, f32::next_up, f32::next_down);
}

negative_control!(
    canon_mirror_decision_edges,
    "a decision that mirrors at |λ̃_y| = δ_λ, the deadband's tie taken as a mirror, must fail",
    expected = "the decision at a = 1e-12, at the pole true",
    check_decision_edges::<f64>(
        |m, x| defective(m, x, false, true),
        f64::next_up,
        f64::next_down
    )
);

// ── canon_mirror_extreme_scales (REQ-DEC-017, REQ-DEC-002) ───────────────────────────────────────────────────

/// The far scales at each precision: up to the largest `s` whose `I_in = s²` is finite, down to the smallest whose
/// `I_in` is normal, where the decision's sides, of degree 4 in `s`, overflow or underflow in plain floats.
const SCALES_F32: [f64; 9] = [4e15, 1e16, 1e18, 1e-8, 1e-9, 1e-10, 1e-12, 1e-15, 1e-18];
const SCALES_F64: [f64; 9] = [
    1e83, 1e100, 1e150, 1e153, 1e-75, 1e-80, 1e-100, 1e-150, 1e-153,
];

/// [`gauged`]'s state at `λ̃_y = t`, turned by 0.9 and scaled by `s`, with momenta, at `R`.
fn far<R: Precision>(t: f64, s: f64) -> Ic<R> {
    let m = [0.15, 0.25, 0.6];
    from64(&Ic {
        m,
        r: gauged(m, t, false, 0.9, s),
        p: [[0.2, -0.1], [-0.3, 0.4], [0.05, 0.1]],
    })
}

/// The decision at `λ̃_y = ∓0.577` (mirror, no mirror) on [`far`]'s states at each scale, with `I_in` finite and normal
/// at `R`, and `C`'s own flag there where `with_c`.
fn check_extreme_scales<R: Precision>(decide: Decide<R>, scales: &[f64], with_c: bool) {
    for &s in scales {
        for t in [-0.577, 0.577] {
            let ic = far::<R>(t, s);
            let com = canon::centre_of_mass(ic.m, ic.r);
            let x = ic.r.map(|v| [v[0] - com[0], v[1] - com[1]]);
            let i = inertia(ic.m, x);
            assert!(
                !i.non_finite_bits() && i >= R::min_positive_value(),
                "{}: I_in = {i:?} is not finite and normal at s = {s:e}",
                R::NAME
            );
            let want = t < 0.0;
            assert_eq!(
                decide(ic.m, x),
                want,
                "{}: the decision at s = {s:e}, λ̃_y = {t}, against λ̃_y < −δ_λ",
                R::NAME
            );
            if with_c {
                assert_eq!(
                    canonicalise(&ic).mirrored,
                    want,
                    "{}: C's mirror at s = {s:e}, λ̃_y = {t}",
                    R::NAME
                );
            }
        }
    }
}

#[test]
fn canon_mirror_extreme_scales() {
    check_extreme_scales::<f32>(mirror_decision, &SCALES_F32, true);
    check_extreme_scales::<f64>(mirror_decision, &SCALES_F64, true);
}

/// The decision in plain floats, without its integer exponents, for the control.
#[cfg(feature = "controls")]
fn unscaled<R: Precision>(m: [R; 3], x: [[R; 2]; 3]) -> bool {
    let rho = [x[1][0] - x[0][0], x[1][1] - x[0][1]];
    let m01 = m[0] + m[1];
    let big = |k: usize| m01 * x[2][k] - m[0] * x[0][k] - m[1] * x[1][k];
    let c = rho[0] * big(1) - rho[1] * big(0);
    let d = <R as ChartConstants>::DELTA_LAMBDA;
    let p = rho[0] * rho[0] + rho[1] * rho[1];
    c < R::zero() && c * c * m[2] > d * d * m01 * p * inertia(m, x)
}

negative_control!(
    canon_mirror_extreme_scales,
    "the decision in plain floats overflows at s = 1e16 in f32 and must fail",
    expected = "f32: the decision at s = 1e16",
    check_extreme_scales::<f32>(unscaled, &SCALES_F32, false)
);

// ── canon_mirror_scale_invariance (REQ-DEC-017) ────────────────────────────────────────────────────────────────

/// The powers of two the states are scaled by, at each precision: near `1e±8`, `1e±9`, `1e±12`, `1e±18` and `1e±30`
/// at f32, and to `2^±900` at f64. Past `2^±60` (f32) and `2^±500` (f64) `I_in` itself overflows or underflows, which
/// the decision, formed from the differences alone, does not read.
const EXPS_F32: [i32; 11] = [-100, -60, -40, -30, -27, 0, 27, 30, 40, 60, 100];
const EXPS_F64: [i32; 11] = [-900, -500, -330, -250, -100, 0, 100, 250, 330, 500, 900];

/// Generic states, random masses, positions and momenta: `λ̃_y` anywhere, about half of them mirrored.
fn generic<R: Precision>(n: usize) -> Vec<Ic<R>> {
    let mut rng = SplitMix::new(43);
    let mut u = |lo: f32, hi: f32| rng.uniform(lo, hi) as f64;
    (0..n)
        .map(|_| {
            let w = [u(0.2, 1.2), u(0.2, 1.2), u(0.2, 1.2)];
            let total: f64 = w.iter().sum();
            from64(&Ic {
                m: w.map(|x| x / total),
                r: [0; 3].map(|_| [u(-1.0, 1.0), u(-1.0, 1.0)]),
                p: [0; 3].map(|_| [u(-0.5, 0.5), u(-0.5, 0.5)]),
            })
        })
        .collect()
}

/// `ic`'s positions times `2^e`, exactly.
fn scaled<R: Precision>(ic: &Ic<R>, e: i32) -> Ic<R> {
    let k = cast::<R>(2f64.powi(e));
    Ic {
        m: ic.m,
        r: ic.r.map(|v| v.map(|q| q * k)),
        p: ic.p,
    }
}

/// The decision on `ic`'s CoM-frame positions, formed as `C` forms them.
fn decide_on<R: Precision>(decide: Decide<R>, ic: &Ic<R>) -> bool {
    let com = canon::centre_of_mass(ic.m, ic.r);
    decide(ic.m, ic.r.map(|v| [v[0] - com[0], v[1] - com[1]]))
}

/// A power-of-two scale changes no bit of the rescaled state, so the decision, and `C`'s flag where `with_c`, is the
/// same at every scale in `exps` as at `2⁰`, on every one of `n` generic states; the number mirrored at `2⁰`.
fn check_scale_invariance<R: Precision>(decide: Decide<R>, exps: &[i32], with_c: bool) -> usize {
    let states = generic::<R>(2000);
    let mut mirrors = 0;
    for (j, ic) in states.iter().enumerate() {
        let base = decide_on(decide, ic);
        mirrors += base as usize;
        for &e in exps {
            let x = scaled(ic, e);
            assert_eq!(
                decide_on(decide, &x),
                base,
                "{}: the decision at 2^{e} differs from 2⁰ at state {j}",
                R::NAME
            );
            if with_c {
                assert_eq!(
                    canonicalise(&x).mirrored,
                    base,
                    "{}: C's mirror at 2^{e} differs from 2⁰ at state {j}",
                    R::NAME
                );
            }
        }
    }
    mirrors
}

#[test]
fn canon_mirror_scale_invariance() {
    let m32 = check_scale_invariance::<f32>(mirror_decision, &EXPS_F32, true);
    let m64 = check_scale_invariance::<f64>(mirror_decision, &EXPS_F64, true);
    assert!(m32 > 0 && m64 > 0, "no generic state mirrored");
    eprintln!(
        "canon_mirror_scale_invariance: 2000 generic states, {m32} (f32) and {m64} (f64) mirrored, the same at every \
         scale"
    );
}

negative_control!(
    canon_mirror_scale_invariance,
    "the decision in plain floats changes with the scale and must fail",
    expected = "f32: the decision at 2^",
    {
        check_scale_invariance::<f32>(unscaled, &EXPS_F32, false);
    }
);

// ── canon_product_fusion_proof (REQ-DEC-017) ───────────────────────────────────────────────────────────────────

/// The f32 [`IeeeBits::product`]'s four partial products and its fixed sum, written out with each of its two
/// contractible sites `x·y + z` fused or not as `fuse` says.
fn product_with(a: f32, b: f32, fuse: [bool; 2]) -> f32 {
    let ah = f32::from_bits(a.to_bits() & 0xFFFF_F000);
    let bh = f32::from_bits(b.to_bits() & 0xFFFF_F000);
    let (al, bl) = (a - ah, b - bh);
    let mul_add =
        |x: f32, y: f32, z: f32, fused: bool| if fused { x.mul_add(y, z) } else { x * y + z };
    let inner = mul_add(ah, bl, al * bh, fuse[0]) + al * bl;
    mul_add(ah, bh, inner, fuse[1])
}

/// On `n` random pairs over a wide exponent range, `product(a, b)` is the same whether each of its two contractible
/// sites (`ah·bl + al·bh` and `ah·bh + rest`) is fused or not: the partial products are exact, so fusion cannot change
/// a sum's rounding. `with` is the product under test, given the fusion choice. The number of pairs.
fn check_fusion_proof(with: fn(f32, f32, [bool; 2]) -> f32) -> usize {
    let mut rng = SplitMix::new(53);
    let n = 200_000;
    for j in 0..n {
        let e = |r: &mut SplitMix| 2f32.powi((r.uniform(-40.0, 40.0)) as i32);
        let a = rng.uniform(-2.0, 2.0) * e(&mut rng);
        let b = rng.uniform(-2.0, 2.0) * e(&mut rng);
        let plain = a.product(b);
        for fuse in [[false, false], [true, false], [false, true], [true, true]] {
            assert_eq!(
                with(a, b, fuse).to_bits(),
                plain.to_bits(),
                "the product of {a:e} and {b:e} changes with fusion {fuse:?} at pair {j}"
            );
        }
    }
    n
}

#[test]
fn canon_product_fusion_proof() {
    let n = check_fusion_proof(product_with);
    eprintln!("canon_product_fusion_proof: {n} pairs, the same bits fused or not at each site");
}

negative_control!(
    canon_product_fusion_proof,
    "a split that clears 10 bits, not 12, leaves 14-bit parts whose product is rounded, and must fail",
    expected = "changes with fusion",
    {
        check_fusion_proof(|a, b, fuse| {
            let ah = f32::from_bits(a.to_bits() & 0xFFFF_FC00);
            let bh = f32::from_bits(b.to_bits() & 0xFFFF_FC00);
            let (al, bl) = (a - ah, b - bh);
            let inner = (ah * bl + al * bh) + al * bl;
            if fuse[1] {
                ah.mul_add(bh, inner)
            } else {
                a.product(b)
            }
        });
    }
);

// ── canon_pow2_bits ─────────────────────────────────────────────────────────────────────────────────────────────

/// [`IeeeBits::pow2`] is `2^e` and [`IeeeBits::exponent`] reads `e` back, over every normal exponent at `R`, and the
/// exponent of `±1.5·2^e` is `e`; zero reads the field's least value. `pow2` is the function under test.
fn check_pow2<R: Precision>(pow2: fn(i32) -> R, lo: i32, hi: i32) {
    for e in lo..=hi {
        let want = cast::<R>(2f64.powi(e));
        assert_eq!(pow2(e), want, "{}: pow2({e})", R::NAME);
        assert_eq!(want.exponent(), e, "{}: the exponent of 2^{e}", R::NAME);
        let x = want * cast::<R>(-1.5);
        assert_eq!(x.exponent(), e, "{}: the exponent of −1.5·2^{e}", R::NAME);
    }
    assert_eq!(
        R::zero().exponent(),
        lo - 1,
        "{}: the exponent of 0",
        R::NAME
    );
}

#[test]
fn canon_pow2_bits() {
    check_pow2::<f32>(<f32 as IeeeBits>::pow2, -126, 127);
    check_pow2::<f64>(<f64 as IeeeBits>::pow2, -1022, 1023);
}

negative_control!(
    canon_pow2_bits,
    "a pow2 whose bias has the wrong sign must fail",
    expected = "f32: pow2(-126)",
    check_pow2::<f32>(|e| f32::from_bits(((e - 127) as u32) << 23), -126, 127)
);

// ── canon_mirror_exact_terms (REQ-DEC-017) ─────────────────────────────────────────────────────────────────────

/// `v` turned by `k` quarter turns, exactly: `(x, y) ↦ (−y, x)` `k` times.
fn quarter<R: Precision>(v: [R; 2], k: usize) -> [R; 2] {
    (0..k).fold(v, |w, _| [-w[1], w[0]])
}

/// States whose sums have an exact zero term, masses `(¼, ¼, ½)`, and their exact `c = ρ × Λ`, whose sign alone sets
/// the decision (each `λ̃_y` is at least about `0.1`, far outside the deadband):
/// - `x₀ = 0`, `x₁ = (1, 1)`, `x₂ = (0, y)`: `d02_x = 0`, so `Λ_x = m₀·0 + m₁·(−1)`, and `c = y/2`;
/// - `x₀ = 0`, `x₁ = (1, 0)`, `x₂ = (0.375, y)`: `ρ_y = 0`, so `c = ρ_xΛ_y − 0·Λ_x` and `‖ρ‖² = 1 + 0`, and `c = y/2`;
///
/// each turned by `k` quarter turns, which keeps the zeros (in the other component) and `c`.
fn zero_terms<R: Precision>() -> Vec<([[R; 2]; 3], bool)> {
    let mut out = Vec::new();
    for y in [0.25, -0.25, 0.75, -0.75, 3.0, -3.0] {
        for x1 in [[1.0, 1.0], [1.0, 0.0]] {
            let x2 = if x1[1] == 1.0 { 0.0 } else { 0.375 };
            for k in 0..4 {
                let r = [[0.0, 0.0], x1, [x2, y]].map(|v| quarter(v.map(cast::<R>), k));
                out.push((r, y < 0.0));
            }
        }
    }
    out
}

/// States whose sums add terms more than the exponent range apart, masses `(¼, ¼, ½)`: `x₀ = 0`,
/// `x₁ = (1, 2^−g)`, `x₂ = (0, −t)`, with `g = 100` at f32 and `1000` at f64, so `‖ρ‖² = 1 + 2^−2g` sums terms `2g`
/// binades apart, past the exponent range. `c = −M₀₁t` to within `2^−g`; at `t = 2^−50`
/// `m₂c² ≈ 2^−103` is below `δ_λ²·M₀₁·‖ρ‖²·d² ≈ 2^−83` (no mirror), at `t = 2^−20` it is above (a mirror), and at
/// `t = −2^−20` `c > 0` (no mirror).
fn spread_terms<R: Precision>(g: i32) -> Vec<([[R; 2]; 3], bool)> {
    [(-50, 1.0, false), (-20, 1.0, true), (-20, -1.0, false)]
        .into_iter()
        .map(|(e, sign, mirror)| {
            let t = sign * 2f64.powi(e);
            let r = [[0.0, 0.0], [1.0, 2f64.powi(-g)], [0.0, -t]].map(|v| v.map(cast::<R>));
            (r, mirror)
        })
        .collect()
}

/// The decision on every exact-term state at every power of two in `exps`, and on the spread states, is the exact
/// one; the number of states checked.
fn check_exact_terms<R: Precision>(decide: Decide<R>, exps: &[i32], g: i32) -> usize {
    let m = [0.25, 0.25, 0.5].map(cast::<R>);
    let mut n = 0;
    for (j, (r, want)) in zero_terms::<R>().into_iter().enumerate() {
        for &e in exps {
            let k = cast::<R>(2f64.powi(e));
            let x = r.map(|v| v.map(|q| q * k));
            assert_eq!(
                decide(m, x),
                want,
                "{}: the decision at 2^{e} on zero-term state {j} ({x:?})",
                R::NAME
            );
            n += 1;
        }
    }
    for (j, (r, want)) in spread_terms::<R>(g).into_iter().enumerate() {
        assert_eq!(
            decide(m, r),
            want,
            "{}: the decision on spread state {j} ({r:?})",
            R::NAME
        );
        n += 1;
    }
    n
}

#[test]
fn canon_mirror_exact_terms() {
    let n32 = check_exact_terms::<f32>(mirror_decision, &EXPS_F32, 100);
    let n64 = check_exact_terms::<f64>(mirror_decision, &EXPS_F64, 1000);
    eprintln!(
        "canon_mirror_exact_terms: {n32} (f32) and {n64} (f64) states with an exact zero term or terms past the \
         exponent range apart, each decided as its exact c says"
    );
}

negative_control!(
    canon_mirror_exact_terms,
    "the decision in plain floats underflows on the zero-term states at 2^−100 in f32 and must fail",
    expected = "f32: the decision at 2^-100",
    {
        check_exact_terms::<f32>(unscaled, &EXPS_F32, 100);
    }
);

// ── canon_mirror_subnormal_difference (REQ-DEC-017) ────────────────────────────────────────────────────────────

/// States whose inner pair is a subnormal apart, `v` the subnormal with bits `0x201`, masses `(¼, ¼, ½)`, and the exact
/// decision: `x₁ = (v, 0)`, `x₂ = (1536, −1024)`, `c = −512v`, a mirror; `x₁ = (0, v)`, `x₂ = (−1536, −1024)`,
/// `c = 768v`, none. `x₀ = 0`. Each `λ̃_y` is far outside the deadband.
fn subnormal_pairs<R: Precision>(v: R) -> [([[R; 2]; 3], bool); 2] {
    let z = R::zero();
    let c = cast::<R>;
    [
        ([[z, z], [v, z], [c(1536.0), c(-1024.0)]], true),
        ([[z, z], [z, v], [c(-1536.0), c(-1024.0)]], false),
    ]
}

/// The decision on the subnormal pairs at `R`, through `decide`, is the exact one, on a backend that keeps subnormals.
fn check_subnormal<R: Precision>(decide: Decide<R>, v: R) {
    assert!(
        v != R::zero() && v.exponent() == R::zero().exponent(),
        "{}: v is subnormal",
        R::NAME
    );
    let m = [0.25, 0.25, 0.5].map(cast::<R>);
    for (j, (r, want)) in subnormal_pairs(v).into_iter().enumerate() {
        assert_eq!(
            decide(m, r),
            want,
            "{}: the decision on subnormal pair {j}",
            R::NAME
        );
    }
}

#[test]
fn canon_mirror_subnormal_difference() {
    check_subnormal::<f32>(mirror_decision, f32::from_bits(0x201));
    check_subnormal::<f64>(mirror_decision, f64::from_bits(0x201));
    eprintln!(
        "canon_mirror_subnormal_difference: an inner pair 0x201 (subnormal) apart, at f32 and f64: the mirror on c < 0 \
         and none on c > 0"
    );
}

/// The decision after flushing every subnormal coordinate to zero, as a GPU that flushes reads the state.
#[cfg(feature = "controls")]
fn flushed_input(m: [f32; 3], r: [[f32; 2]; 3]) -> bool {
    let flush = |q: f32| if q.abs() < f32::MIN_POSITIVE { 0.0 } else { q };
    mirror_decision(m, r.map(|v| v.map(flush)))
}

negative_control!(
    canon_mirror_subnormal_difference,
    "the decision on the input with its subnormals flushed reads the pole and must fail",
    expected = "f32: the decision on subnormal pair 1",
    check_subnormal::<f32>(flushed_input, f32::from_bits(0x201))
);

// ── canon_inertia_extremes (RQ-296) ─────────────────────────────────────────────────────────────────────────────

/// Bodies apart at the scale `k`, masses `(¼, ¼, ½)`, with momenta, one of them zero in `x`.
fn spread<R: Precision>(k: f64) -> Ic<R> {
    from64(&Ic {
        m: [0.25, 0.25, 0.5],
        r: [[-k, 0.0], [k, 0.0], [0.0, k]],
        p: [[0.1, 0.0], [-0.3, 0.2], [0.1, -0.1]],
    })
}

/// No `1/0` was formed: `λ` and every component of the state are finite. Says nothing about which values they are
/// (RQ-296 holds them).
fn check_no_division_by_zero<R: Precision>(c: &Canonical<R>, at: &str) {
    let ic = to64(&c.ic);
    let all = ic.r.iter().chain(&ic.p).flatten().copied();
    let lambda: f64 = c.lambda.into();
    assert!(
        all.chain([lambda]).all(f64::is_finite),
        "a non-finite value at {at}: {c:?}"
    );
}

/// `I_in` rounds to 0 with the bodies apart, and to `+∞`: each case is reached (RQ-296's underflow and overflow), and at
/// underflow no `1/0` is formed. The outputs themselves are RQ-296's and are not asserted.
fn extremes_at<R: Precision>(under: f64, over: f64) {
    let x = spread::<R>(under);
    let c = canonicalise(&x);
    let at = format!("{} at scale {under:e}", R::NAME);
    assert!(x.r[0] != x.r[1], "the bodies coincide at {at}");
    assert!(c.i_in == R::zero(), "I_in = {:?}, not 0, at {at}", c.i_in);
    check_no_division_by_zero(&c, &at);
    let c = canonicalise(&spread::<R>(over));
    assert!(
        c.i_in.non_finite_bits(),
        "I_in = {:?} is finite at {} scale {over:e}",
        c.i_in,
        R::NAME
    );
}

#[test]
fn canon_inertia_extremes() {
    extremes_at::<f64>(1e-170, 1e160);
    extremes_at::<f32>(1e-25, 1e20);
}

negative_control!(
    canon_inertia_extremes,
    "a step 0 that divides by √I_in unguarded forms r/0 at underflow and must fail",
    expected = "a non-finite value",
    {
        let x = spread::<f64>(1e-170);
        let mut c = canonicalise(&x);
        for v in c.ic.r.iter_mut() {
            *v = v.map(|q| q / c.i_in.sqrt());
        }
        check_no_division_by_zero(&c, "an unguarded divisor");
    }
);

// ── canon_rotation (REQ-DEC-002) ────────────────────────────────────────────────────────────────────────────────

/// A step-2 turn: `ρ ↦ (cos φ, sin φ)`.
type Turn<R> = fn([R; 2]) -> (R, R);

/// `ρ` at `2^e` in the direction `θ`, each component rounded to `R` (a subnormal one included).
fn turned<R: Precision>(theta: f64, e: i32) -> [R; 2] {
    // In two factors, each normal, so `2^e` below f64's normal range is not formed by `powi`.
    let (a, b) = (2f64.powi(e / 2), 2f64.powi(e - e / 2));
    [
        cast::<R>(theta.cos() * a * b),
        cast::<R>(theta.sin() * a * b),
    ]
}

/// The turn on `ρ` against `ρ/‖ρ‖`, from `ρ`'s own `R` values at f64 (at f64 itself, to its own rounding): `c` and `s` each within `tol` eps of it, and
/// the turn's error in direction, `|c·ρ_y − s·ρ_x|/‖ρ‖`, the `y` that `R(−φ)` leaves on `ρ`, within `tol` eps. The
/// worst of the two, in eps.
fn check_turn<R: Precision>(turn: Turn<R>, rho: [R; 2], tol: f64) -> f64 {
    let (x, y): (f64, f64) = (rho[0].into(), rho[1].into());
    // Taken by a power of two, exactly, away from f64's overflow and underflow, so `ρ/‖ρ‖` is f64's to an ulp or so.
    let big = x.abs().max(y.abs());
    let k = if big > 1e300 {
        2f64.powi(-600)
    } else if big < 1e-300 {
        2f64.powi(600)
    } else {
        1.0
    };
    let (x, y) = (x * k, y * k);
    let n = f64::hypot(x, y);
    let (c, s) = turn(rho);
    let (c, s): (f64, f64) = (c.into(), s.into());
    let dc = (c - x / n).abs().max((s - y / n).abs()) / eps::<R>();
    let dy = c.mul_add(y / n, -(s * (x / n))).abs() / eps::<R>();
    assert!(
        dy <= tol,
        "{}: at ρ = {rho:?}, the turn turns ρ {dy:.2} eps off +x, over {tol}",
        R::NAME
    );
    assert!(
        dc <= tol,
        "{}: at ρ = {rho:?}, (c, s) = ({c:e}, {s:e}) is {dc:.2} eps from ρ/‖ρ‖, over {tol}",
        R::NAME
    );
    dc.max(dy)
}

/// Step 2's turn ([`canon::rotation`]): at `ρ = 0`, `(1, +0)` exactly for every sign of the zeros (R-437); elsewhere
/// `ρ/‖ρ‖` to `tol` eps, in value and in direction, on 64 directions at every power of two from the least subnormal to
/// the greatest exponent, at the format's greatest value and with the components at opposite ends of the range. The
/// worst, in eps.
fn check_rotation<R: Precision>(turn: Turn<R>, lo: i32, hi: i32, tol: f64) -> f64 {
    for rho in [[0.0, 0.0], [-0.0, 0.0], [0.0, -0.0], [-0.0, -0.0]].map(|v| v.map(cast::<R>)) {
        let (c, s) = turn(rho);
        let (c, s): (f64, f64) = (c.into(), s.into());
        assert!(
            c.to_bits() == 1f64.to_bits() && s.to_bits() == 0f64.to_bits(),
            "{}: at ρ = {rho:?}, (c, s) = ({c:?}, {s:?}), not (1, +0)",
            R::NAME
        );
    }
    let mut rng = SplitMix::new(437);
    let mut worst: f64 = 0.0;
    for e in lo..=hi {
        for j in 0..64 {
            let theta: f64 = <f64 as From<f32>>::from(rng.uniform(-3.2, 3.2)) + 1e-3 * j as f64;
            let rho = turned::<R>(theta, e);
            if rho[0] != R::zero() || rho[1] != R::zero() {
                worst = worst.max(check_turn(turn, rho, tol));
            }
        }
    }
    let tiny = cast::<R>(2f64.powi(lo + 100)) * cast::<R>(2f64.powi(-100));
    assert!(tiny > R::zero(), "{}: 2^{lo} is 0", R::NAME);
    // `2^hi·(2 − ε)`, the greatest finite value.
    let max = cast::<R>(2f64.powi(hi)) * (cast::<R>(2.0) - cast::<R>(eps::<R>()));
    for rho in [
        [max, max],
        [-max, max],
        [max, tiny],
        [tiny, -max],
        [tiny, tiny],
    ] {
        worst = worst.max(check_turn(turn, rho, tol));
    }
    worst
}

#[test]
fn canon_rotation() {
    let w32 = check_rotation::<f32>(canon::rotation, -149, 127, 2.0);
    let w64 = check_rotation::<f64>(canon::rotation, -1074, 1023, 2.0);
    eprintln!("canon_rotation: worst {w32:.2} eps at f32, {w64:.2} at f64 (bound 2)");
}

negative_control!(
    canon_rotation,
    "a turn through sin and cos of an angle 10⁻⁶ rad off, the order of CI's lavapipe trig, must fail the direction",
    expected = "the turn turns ρ",
    {
        check_rotation::<f32>(
            |rho| {
                // Off by 10⁻⁶ away from the pole, where R-437's φ = 0 is kept.
                let phi = canon::rotation_angle(rho);
                let phi = if phi == 0.0 { phi } else { phi + 1e-6 };
                (phi.cos(), phi.sin())
            },
            -20,
            20,
            2.0,
        );
    }
);
