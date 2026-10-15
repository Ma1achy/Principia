//! The `ICDescriptor` the decode stage writes (`kernel::decode::descriptor`; dd_decoder §3.6; dd_generation_root §3.6),
//! at the f64 and the f32 instantiation natively (R-265):
//! - `ic_descriptor_golden`: at the golden IC (`z = 0`) each field against its hand-computed value, the exact value
//!   rounded to the precision (`fixtures/golden_ic/z0.json`; R-421): `K_0` and `virial_ratio`, 0 at rest, bit-exact;
//!   the others within REQ-PAY-095's proposed ulps (REQ-PAY-034).
//! - `ic_descriptor_names`: the generated struct's fields are dd_generation_root §3.6's twelve, in order, with no `E₀`;
//!   the derived `E₀` is `K_0 + V_0` (REQ-PAY-034, R-86).
//! - `ic_descriptor_definitions`: on fuzzed `z`, every field is dd_decoder §3.6's formula of the decoded `(m, r, p)`,
//!   evaluated again here in f64, in the canonical frame and rotated out of it; and in the canonical frame
//!   `rho_mag = cos α`, `lambda_mag = sin α` and `rho_angle = β` (REQ-PAY-088).
//! - `ic_descriptor_rho_angle_axis`: `rho_angle` on the axis `ρ × λ = 0` and next to it, at f64 and f32: `π̂` when
//!   antiparallel, whatever the sign of a zero `ρ × λ`; 0 when parallel and at the pole; `atan2`'s value taken down to
//!   `π̂`, or read as `π̂` at or below `−π̂`, elsewhere (dd_decoder §3.6; REQ-PAY-088).
//! - `ic_descriptor_floors`: REQ-TOOL-163's evidence on decoded values, `rho_ratio` and `r_min_pair_0` against their
//!   log floors (R-419).
//!
//! **REQ-PAY-095's proposal** (R-71, R-421; unconfirmed until the M2 gate). At the golden IC the fields 0 at rest are
//! `K_0` and `virial_ratio` (R-421), compared bit-exact; none of the six fields REQ-PAY-088 defines is 0 there. Each
//! other field is within 4 ulps of its exact value rounded to the precision, at f64 and at f32
//! (`decode_support::DESCRIPTOR_ULPS`). The measured distance, in ulps, the decode at `z = 0` being deterministic:
//!
//! | field | exact | f64: measured | f32: measured | proposed, f64 and f32 |
//! |---|---|---|---|---|
//! | `m0`, `m1`, `m2` | 1/3 | 0 | 0 | 4 |
//! | `q_mass` | 1 | 0 | 0 | 4 |
//! | `rho_mag` | 1/√2 | 0 | 1 | 4 |
//! | `lambda_mag` | 1/√2 | 2 | 0 | 4 |
//! | `rho_ratio` | 1 | 1 | 1 | 4 |
//! | `rho_angle` | π/2 | 0 | 1 | 4 |
//! | `V_0` (and the derived `E₀`) | −√3/9 | 0 | 1 | 4 |
//! | `r_min_pair_0` | √3 | 1 | 1 | 4 |
//! | `K_0`, `virial_ratio` | 0 | bit-exact (+0) | bit-exact (+0) | bit-exact |
//!
//! Re-measured by `cargo test -p kernel --test ic_descriptor -- ic_descriptor_golden --nocapture`.

mod decode_support;

use decode_support::*;
use kernel::decode::descriptor;
use kernel::decode::{self, Decoded};
use kernel::generated::links::*;
use kernel::payload::{ICDescriptor, ICDescriptorOf};
use std::f64::consts::{PI, TAU};
use validation::negative_control;

/// A decode entry at `R`, behind a function pointer, so a control runs the same check on a broken one.
type Run<R> = fn(&kernel::decode::Latent<R>) -> Decoded<R>;

// ── ic_descriptor_golden (REQ-PAY-034, REQ-PAY-095) ─────────────────────────────────────────────────────────────

/// Checks each descriptor field of `run`'s golden IC against the fixture, and the derived `E₀`.
fn check_golden<R: P>(run: Run<R>, worst: &Worst) {
    let g = golden();
    let d = run(&latent::<R>([0.0; 8])).descriptor;
    let mut seen = 0;
    for v in golden_values(&g) {
        let n = name(&v);
        let want = R::from_fixture(&v);
        if let Some(f) = n.strip_prefix("descriptor.") {
            assert_eq!(
                v["zero_at_rest"].as_bool(),
                Some(ZERO_AT_REST.contains(&f)),
                "the fixture marks {f}'s rest value otherwise"
            );
            check_field(f, field(&d, f), want, worst);
            seen += 1;
        } else if n == "E_0" {
            check_field("V_0", descriptor::energy(&d), want, worst);
        }
    }
    assert_eq!(seen, 12, "the fixture does not give the twelve fields");
}

#[test]
fn ic_descriptor_golden() {
    let worst = Worst::default();
    check_golden(decode::decode::<Defaults, f64>, &worst);
    check_golden(decode::decode::<Defaults, f32>, &worst);
    worst.print("ic_descriptor_golden");
}

/// The defaults with the descriptor's `rho_mag` read as the unweighted `‖ρ‖` (`√3` at the golden IC).
#[cfg(feature = "controls")]
fn unweighted_rho<R: P>(z: &kernel::decode::Latent<R>) -> Decoded<R> {
    let mut d = decode::decode::<Defaults, R>(z);
    let rho = d.config.rho;
    d.descriptor.rho_mag = (rho[0] * rho[0] + rho[1] * rho[1]).sqrt();
    d
}

negative_control!(
    ic_descriptor_golden,
    "rho_mag read as the unweighted ‖ρ‖ is not the golden value",
    expected = "the golden descriptor's rho_mag",
    check_golden(unweighted_rho::<f64>, &Worst::default())
);

/// The defaults with `K_0` one ulp above 0: a field 0 at rest compared within a tolerance would pass it.
#[cfg(feature = "controls")]
fn kinetic_ulp<R: P>(z: &kernel::decode::Latent<R>) -> Decoded<R> {
    let mut d = decode::decode::<Defaults, R>(z);
    d.descriptor.K_0 = R::min_positive_value();
    d
}

mod zero_at_rest {
    use super::*;

    negative_control!(
        ic_descriptor_golden,
        "K_0 one step above 0 fails the bit-exact rest check",
        expected = "the golden descriptor's K_0",
        check_golden(kinetic_ulp::<f32>, &Worst::default())
    );
}

// ── ic_descriptor_names (REQ-PAY-034) ───────────────────────────────────────────────────────────────────────────

/// dd_generation_root §3.6's fields, in order (R-22, R-62).
const FIELDS: [&str; 12] = [
    "m0",
    "m1",
    "m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
];

/// The data field names of a struct from its `Debug` form, `Name { a: …, b: … }`, in order; padding (`_…`) skipped.
fn debug_fields(debug: &str) -> Vec<String> {
    let body = debug
        .split_once('{')
        .and_then(|(_, b)| b.rsplit_once('}'))
        .map(|(b, _)| b)
        .expect("a struct's Debug form");
    let mut names = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in body.char_indices() {
        match c {
            '[' | '{' | '(' => depth += 1,
            ']' | '}' | ')' => depth -= 1,
            ',' if depth == 0 => {
                names.push(body[start..i].to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }
    names.push(body[start..].to_owned());
    names
        .iter()
        .filter_map(|f| f.split_once(':').map(|(n, _)| n.trim().to_owned()))
        .filter(|n| !n.starts_with('_'))
        .collect()
}

/// Checks that `debug`, a descriptor's `Debug` form, names exactly §3.6's fields in order, none of them `E₀`, and that
/// the fixture's descriptor values name the same.
fn check_names(debug: &str) {
    let fields = debug_fields(debug);
    assert_eq!(
        fields, FIELDS,
        "the generated ICDescriptor's fields are not dd_generation_root §3.6's"
    );
    assert!(
        !fields
            .iter()
            .any(|f| f.eq_ignore_ascii_case("e_0") || f.eq_ignore_ascii_case("e0")),
        "the ICDescriptor stores E₀"
    );
    let g = golden();
    let fixture: Vec<_> = golden_values(&g)
        .iter()
        .filter_map(|v| name(v).strip_prefix("descriptor.").map(str::to_owned))
        .collect();
    assert_eq!(
        fixture, FIELDS,
        "the fixture's descriptor fields are not §3.6's"
    );
}

/// `E₀` is derived at `R`, `K_0 + V_0`, on the decode of `z`.
fn check_energy<R: P>(z: [f64; 8]) {
    let d = decode::decode::<Defaults, R>(&latent::<R>(z)).descriptor;
    assert_eq!(
        descriptor::energy(&d).bits(),
        (d.K_0 + d.V_0).bits(),
        "E₀ is not K_0 + V_0 at {}",
        R::NAME
    );
}

#[test]
fn ic_descriptor_names() {
    check_names(&format!("{:?}", ICDescriptor::default()));
    check_names(&format!("{:?}", ICDescriptorOf::<f64>::default()));
    over(any_z(), |z| {
        check_energy::<f64>(z);
        check_energy::<f32>(z);
    });
}

negative_control!(
    ic_descriptor_names,
    "a descriptor with an E_0 member must fail",
    expected = "the generated ICDescriptor's fields are not dd_generation_root §3.6's",
    check_names(
        &format!("{:?}", ICDescriptor::default())
            .replace("r_min_pair_0:", "r_min_pair_0: 0.0, E_0:")
    )
);

// ── ic_descriptor_definitions (REQ-PAY-088) ─────────────────────────────────────────────────────────────────────

/// The definitions' tolerance, in `eps` of each field's scale (below): REQ-DEC-044's neighbour, the formula's few
/// roundings at `R` against the same formula in f64 on the same `(m, r, p)`. Over 200 runs at the seeds 1 … 200 the
/// worst was 2.3 `eps` for a field, the rotated IC included, and 5.2 for the canonical frame's
/// `rho_mag² + lambda_mag² = 1`, at f32.
const DEFINITION_EPS: f64 = 16.0;

/// A descriptor computed at `R`, behind a function pointer, so a control runs the same check on a broken one.
type Describe<R> = fn(&kernel::decode::Ic<R>) -> ICDescriptorOf<R>;

/// The norm of `a`, in f64.
fn norm(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}

/// The angle, in radians, [`turned`] rotates an IC by: off every axis, so `ρ` has both components.
const TURN: f64 = 2.0;

/// `ic` rotated by [`TURN`] about the origin, at `R`: out of the canonical frame (`ρ̃` on `+x`), where every term of
/// `ρ × λ` and `ρ · λ` is non-zero. §3.6's fields are rotation invariants, and §3.6's formulas hold in any frame.
fn turned<R: P>(ic: &kernel::decode::Ic<R>) -> kernel::decode::Ic<R> {
    let (c, s) = (R::of(TURN.cos()), R::of(TURN.sin()));
    let turn = |v: [R; 2]| [c * v[0] - s * v[1], s * v[0] + c * v[1]];
    kernel::decode::Ic {
        m: ic.m,
        r: ic.r.map(turn),
        p: ic.p.map(turn),
    }
}

/// Checks `describe` on `ic` against dd_decoder §3.6's formulas, evaluated here in f64 on the same `(m, r, p)`: each
/// field within [`DEFINITION_EPS`] of its scale, the derived fields `rho_ratio` and `virial_ratio` exactly the stored
/// fields' quotients, `q_mass` in `[0, 1]` and `rho_angle` in `(−π̂, π̂]`, `π̂` the image of π (dd_decoder §3.6).
fn check_formulas<R: P>(
    describe: Describe<R>,
    ic: &kernel::decode::Ic<R>,
    at: &str,
    worst: &Worst,
) {
    let d = describe(ic);
    let m = ic.m.map(|x| x.f());
    let r = ic.r.map(|v| v.map(|x| x.f()));
    let p = ic.p.map(|v| v.map(|x| x.f()));
    let sub = |a: [f64; 2], b: [f64; 2]| [a[0] - b[0], a[1] - b[1]];
    let m01 = m[0] + m[1];
    let rho = sub(r[1], r[0]);
    let inner = [
        (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
        (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
    ];
    let lambda = sub(r[2], inner);
    let (s_rho, s_lambda) = ((m[0] * m[1] / m01).sqrt(), (m[2] * m01).sqrt());
    let k: f64 = (0..3)
        .map(|i| (p[i][0].powi(2) + p[i][1].powi(2)) / (2.0 * m[i]))
        .sum();
    let pairs = [(0, 1), (0, 2), (1, 2)];
    let v: f64 = -pairs
        .iter()
        .map(|&(i, j)| m[i] * m[j] / norm(sub(r[i], r[j])))
        .sum::<f64>();
    let r_min = pairs
        .iter()
        .map(|&(i, j)| norm(sub(r[i], r[j])))
        .fold(f64::INFINITY, f64::min);
    // Each field's scale: the magnitude its inputs carry, so a cancellation in `r₁ − r₀` or in the inner CoM is in it.
    let rho_scale = s_rho * (norm(r[0]) + norm(r[1]));
    let lambda_scale = s_lambda * (norm(r[2]) + (m[0] * norm(r[0]) + m[1] * norm(r[1])) / m01);
    let angle_scale = (norm(r[0]) + norm(r[1])) / norm(rho)
        + (norm(r[2]) + (m[0] * norm(r[0]) + m[1] * norm(r[1])) / m01) / norm(lambda);
    let want = [
        ("q_mass", m[0].min(m[1]) / m[0].max(m[1]), 1.0),
        ("rho_mag", s_rho * norm(rho), rho_scale),
        ("lambda_mag", s_lambda * norm(lambda), lambda_scale),
        (
            "rho_angle",
            (rho[0] * lambda[1] - rho[1] * lambda[0])
                .atan2(rho[0] * lambda[0] + rho[1] * lambda[1]),
            angle_scale,
        ),
        ("K_0", k, k),
        ("V_0", v, v.abs()),
        ("r_min_pair_0", r_min, r_min),
        ("m0", m[0], m[0]),
        ("m1", m[1], m[1]),
        ("m2", m[2], m[2]),
    ];
    for (f, w, scale) in want {
        let got = field(&d, f).f();
        let mut diff = got - w;
        if f == "rho_angle" {
            // A cyclic field: −π and π are one angle, and a rounding of `ρ × λ` near 0 can pick either.
            diff = (diff + PI).rem_euclid(TAU) - PI;
        }
        let err = if scale == 0.0 {
            diff.abs()
        } else {
            diff.abs() / scale / R::eps()
        };
        worst.see(&format!("{f} at {}", R::NAME), err);
        assert!(
            err <= DEFINITION_EPS || !scale.is_finite(),
            "{f} = {got} is not dd_decoder §3.6's {w} ({err} eps of {scale}) at {at}"
        );
    }
    assert_eq!(
        d.rho_ratio.bits(),
        (d.rho_mag / d.lambda_mag).bits(),
        "rho_ratio is not rho_mag / lambda_mag at {at}"
    );
    assert_eq!(
        d.virial_ratio.bits(),
        ((d.K_0 + d.K_0) / d.V_0.abs()).bits(),
        "virial_ratio is not 2K_0/|V_0| at {at}"
    );
    let q = d.q_mass.f();
    assert!(
        (0.0..=1.0).contains(&q),
        "q_mass = {q} is outside [0, 1] at {at}"
    );
    let angle = d.rho_angle.f();
    let pi = descriptor::pi_image::<R>().f();
    assert!(
        angle > -pi && angle <= pi,
        "rho_angle = {angle} is outside (−π̂, π̂], π̂ = {pi} at {}, at {at}",
        R::NAME
    );
}

/// Checks `describe` at `z` (the defaults' decode) against dd_decoder §3.6's formulas, on the decoded `(m, r, p)` and
/// on it [`turned`] out of the canonical frame (`check_formulas`); and, where `z` is in the interior (`|zₖ| ≤ 2`), the
/// canonical frame's `rho_mag = cos α`, `lambda_mag = sin α`, `rho_angle = β`.
fn check_definitions<R: P>(describe: Describe<R>, z: [f64; 8], worst: &Worst) {
    let dec = decode::decode::<Defaults, R>(&latent::<R>(z));
    let at = format!("z = {z:?}, {}", R::NAME);
    check_formulas(describe, &dec.ic, &at, worst);
    check_formulas(
        describe,
        &turned(&dec.ic),
        &format!("{at}, turned by {TURN} rad"),
        worst,
    );
    let d = describe(&dec.ic);
    let angle = d.rho_angle.f();
    if z.iter().all(|x| x.abs() <= 2.0) {
        let (alpha, beta) = (dec.config.alpha.f(), dec.config.beta.f());
        let canon = [
            ("rho_mag = cos α", d.rho_mag.f() - alpha.cos()),
            ("lambda_mag = sin α", d.lambda_mag.f() - alpha.sin()),
            (
                "lambda_mag·(rho_angle − β)",
                d.lambda_mag.f() * (angle - beta),
            ),
            (
                "rho_mag² + lambda_mag² = 1",
                d.rho_mag.f().powi(2) + d.lambda_mag.f().powi(2) - 1.0,
            ),
        ];
        for (what, r) in canon {
            let err = r.abs() / R::eps();
            worst.see(&format!("{what} at {}", R::NAME), err);
            assert!(
                err <= DEFINITION_EPS,
                "{what}: off by {err} eps in the canonical frame at {at}"
            );
        }
    }
}

fn check_definitions_both(d64: Describe<f64>, d32: Describe<f32>) {
    let worst = Worst::default();
    over(any_z(), |z| {
        check_definitions(d64, z, &worst);
        check_definitions(d32, z, &worst);
    });
    let interior = [
        -2.0f64..=2.0,
        -2.0..=2.0,
        -2.0..=2.0,
        -2.0..=2.0,
        -2.0..=2.0,
        -2.0..=2.0,
        -2.0..=2.0,
        -2.0..=2.0,
    ];
    validation::prop::run(&interior, |z| {
        check_definitions(d64, z, &worst);
        check_definitions(d32, z, &worst);
        Ok(())
    });
    worst.print("ic_descriptor_definitions");
}

#[test]
fn ic_descriptor_definitions() {
    check_definitions_both(descriptor::describe::<f64>, descriptor::describe::<f32>);
}

/// The descriptor with `rho_angle` measured from `λ` to `ρ`, the opposite sense.
#[cfg(feature = "controls")]
fn reversed_angle<R: P>(ic: &kernel::decode::Ic<R>) -> ICDescriptorOf<R> {
    let mut d = descriptor::describe(ic);
    d.rho_angle = -d.rho_angle;
    d
}

negative_control!(
    ic_descriptor_definitions,
    "rho_angle measured from λ to ρ must fail",
    expected = "rho_angle = ",
    check_definitions_both(reversed_angle::<f64>, reversed_angle::<f32>)
);

// ── ic_descriptor_rho_angle_axis (REQ-PAY-088) ─────────────────────────────────────────────────────────────────

/// `rho_angle` from `ρ × λ` and `ρ · λ`, behind a function pointer, so a control runs the same check on a broken one.
type Angle<R> = fn(R, R) -> R;

/// Checks `angle` against dd_decoder §3.6's cases at `R`, `pi_bits` the bits of `π̂`, the image of π: each result's
/// bits, and each within `(−π̂, π̂]`. `tiny` is far below an ulp of π at `R`, so `atan2(±tiny, −1)` rounds to `±π`'s
/// nearest value: above `π̂` at f32, `π̂` itself at f64.
fn check_axis<R: P>(angle: Angle<R>, pi_bits: u64, tiny: f64) {
    let pi = descriptor::pi_image::<R>();
    assert_eq!(pi.bits(), pi_bits, "π̂ is not the image of π at {}", R::NAME);
    assert!(pi.f() <= PI, "π̂ = {} lies above π at {}", pi.f(), R::NAME);
    let atan2 = |y: f64, x: f64| R::of(y).atan2(R::of(x));
    let cases = [
        // Antiparallel: π̂, whatever the sign of the zero.
        (0.0, -1.0, pi),
        (-0.0, -1.0, pi),
        // Parallel: 0.
        (0.0, 1.0, R::zero()),
        (-0.0, 1.0, R::zero()),
        // The pole, `ρ × λ = ρ · λ = 0`, every sign of zero: 0.
        (0.0, 0.0, R::zero()),
        (-0.0, 0.0, R::zero()),
        (0.0, -0.0, R::zero()),
        (-0.0, -0.0, R::zero()),
        // Next to the antiparallel axis: `atan2` gives π's nearest value or its negative; both read `π̂`.
        (tiny, -1.0, pi),
        (-tiny, -1.0, pi),
        // Elsewhere, `atan2`'s own value.
        (1.0, 0.0, atan2(1.0, 0.0)),
        (-1.0, 0.0, atan2(-1.0, 0.0)),
        (1.0, -1.0, atan2(1.0, -1.0)),
        (-1.0, -1.0, atan2(-1.0, -1.0)),
        (1.0, 1.0, atan2(1.0, 1.0)),
    ];
    for (cross, dot, want) in cases {
        let got = angle(R::of(cross), R::of(dot));
        assert_eq!(
            got.bits(),
            want.bits(),
            "rho_angle({cross:?}, {dot:?}) = {:?}, not {:?}, at {}",
            got.f(),
            want.f(),
            R::NAME
        );
        assert!(
            got.f() > -pi.f() && got.f() <= pi.f(),
            "rho_angle({cross:?}, {dot:?}) = {:?} is outside (−π̂, π̂] at {}",
            got.f(),
            R::NAME
        );
    }
}

fn check_axis_both(a64: Angle<f64>, a32: Angle<f32>) {
    check_axis(a64, PI.to_bits(), 1e-300);
    check_axis(a32, 0x4049_0FDA, 1e-10);
}

#[test]
fn ic_descriptor_rho_angle_axis() {
    check_axis_both(descriptor::angle::<f64>, descriptor::angle::<f32>);
}

/// `atan2(ρ × λ, ρ · λ)` alone, with no case chosen: −π at `ρ × λ = −0`.
#[cfg(feature = "controls")]
fn bare_atan2<R: P>(cross: R, dot: R) -> R {
    cross.atan2(dot)
}

negative_control!(
    ic_descriptor_rho_angle_axis,
    "rho_angle as atan2 alone must fail",
    expected = "rho_angle(-0.0, -1.0) = ",
    check_axis_both(bare_atan2::<f64>, bare_atan2::<f32>)
);

// ── ic_descriptor_floors (REQ-TOOL-163) ─────────────────────────────────────────────────────────────────────────

/// The register's provisional floors (`crates/ledger/src/constants.rs`, `LOG_FLOOR_RHO_RATIO` and
/// `LOG_FLOOR_R_MIN_PAIR_0`; R-419), which kernel's tests cannot read from the ledger (kernel → ledger is a
/// build-dependency only, R-185): `rho_ratio`'s 2⁻²³, `r_min_pair_0`'s 2⁻¹⁴.
const RHO_RATIO_FLOOR: f64 = 1.0 / 8_388_608.0;
const R_MIN_FLOOR: f64 = 1.0 / 16_384.0;

/// A field's decoded values over a domain: their count, minimum, quantiles and count below `floor`.
struct Spread {
    values: Vec<f64>,
}

impl Spread {
    fn new() -> Self {
        Spread { values: Vec::new() }
    }
    fn quantile(&self, q: f64) -> f64 {
        let mut v = self.values.clone();
        v.sort_by(f64::total_cmp);
        v[((v.len() - 1) as f64 * q).round() as usize]
    }
    fn below(&self, floor: f64) -> usize {
        self.values.iter().filter(|&&x| x < floor).count()
    }
    fn report(&self, what: &str, floor: f64) -> String {
        format!(
            "{what}: n = {}, min = {:.3e}, 1% = {:.3e}, 50% = {:.3e}, 99% = {:.3e}, below the floor {:.3e}: {}",
            self.values.len(),
            self.quantile(0.0),
            self.quantile(0.01),
            self.quantile(0.5),
            self.quantile(0.99),
            floor,
            self.below(floor)
        )
    }
}

/// The two fields' decoded values at `R` over the validated hypercube `[0, 1]⁸` (chart_decoder_contract Part 2) and
/// over the whole domain, saturation included; as the descriptor stores them, at `R`.
fn spreads<R: P>(run: Run<R>) -> [(&'static str, Spread, Spread); 2] {
    let hyper = [
        0.0f64..=1.0,
        0.0..=1.0,
        0.0..=1.0,
        0.0..=1.0,
        0.0..=1.0,
        0.0..=1.0,
        0.0..=1.0,
        0.0..=1.0,
    ];
    let (mut ratio_h, mut rmin_h) = (Spread::new(), Spread::new());
    let (mut ratio_d, mut rmin_d) = (Spread::new(), Spread::new());
    let cell = std::cell::RefCell::new((&mut ratio_h, &mut rmin_h));
    validation::prop::run(&hyper, |z| {
        let d = run(&latent::<R>(z)).descriptor;
        let mut c = cell.borrow_mut();
        c.0.values.push(d.rho_ratio.f());
        c.1.values.push(d.r_min_pair_0.f());
        Ok(())
    });
    let cell = std::cell::RefCell::new((&mut ratio_d, &mut rmin_d));
    over(any_z(), |z| {
        let dec = run(&latent::<R>(z));
        let d = dec.descriptor;
        // 0 is reached only at the collision pole, where the emitted pair coincide (dd_decoder §3.6).
        let r = dec.ic.r;
        let coincide = r[0] == r[1] || r[1] == r[2] || r[2] == r[0];
        for (what, x) in [("rho_ratio", d.rho_ratio), ("r_min_pair_0", d.r_min_pair_0)] {
            assert!(
                x.f() > 0.0 || (x.f() == 0.0 && coincide),
                "{what} = {:?} at z = {z:?}, {}, and no two bodies coincide",
                x,
                R::NAME
            );
        }
        let mut c = cell.borrow_mut();
        c.0.values.push(d.rho_ratio.f());
        c.1.values.push(d.r_min_pair_0.f());
    });
    [
        ("rho_ratio", ratio_h, ratio_d),
        ("r_min_pair_0", rmin_h, rmin_d),
    ]
}

/// Checks that over the validated hypercube every decoded `rho_ratio` and `r_min_pair_0` lies above its floor (the
/// population the view resolves is wholly above it), and that over the whole domain each is > 0, or 0 only where two
/// emitted bodies coincide (the collision pole, dd_decoder §3.6); and reports both fields over the hypercube and the
/// whole domain.
fn check_floors<R: P>(run: Run<R>, floors: [f64; 2]) {
    for ((what, hyper, domain), floor) in spreads(run).into_iter().zip(floors) {
        eprintln!(
            "{}",
            hyper.report(&format!("{what} at {}, [0, 1]⁸", R::NAME), floor)
        );
        eprintln!(
            "{}",
            domain.report(&format!("{what} at {}, the domain", R::NAME), floor)
        );
        assert_eq!(
            hyper.below(floor),
            0,
            "{what} at {}: a decoded value over [0, 1]⁸ is below its floor {floor:e}",
            R::NAME
        );
    }
    // At f32, `rho_ratio` below its floor is the decode's last grid step alone: `α` saturated at the f32 image of π/2
    // (R-417), where `cot α ≈ 7.5e-8`; the step before it, `cot α ≈ 1.9e-7`, is above the floor.
    if R::NAME == "f32" {
        over(any_z(), |z| {
            let d = run(&latent::<R>(z));
            if d.descriptor.rho_ratio.f() < floors[0] {
                assert_eq!(
                    d.config.alpha.bits(),
                    R::LIT_6.bits(),
                    "rho_ratio = {} below its floor at α = {:?}, not the saturated α, at z = {z:?}",
                    d.descriptor.rho_ratio.f(),
                    d.config.alpha
                );
            }
        });
    }
}

#[test]
fn ic_descriptor_floors() {
    let floors = [RHO_RATIO_FLOOR, R_MIN_FLOOR];
    check_floors(decode::decode::<Defaults, f64>, floors);
    check_floors(decode::decode::<Defaults, f32>, floors);
    check_floors(decode::decode::<Stick, f64>, floors);
    check_floors(decode::decode::<Stick, f32>, floors);
}

negative_control!(
    ic_descriptor_floors,
    "a floor above the hypercube's smallest r_min_pair_0 must fail",
    expected = "r_min_pair_0 at f64: a decoded value over [0, 1]⁸ is below its floor",
    check_floors(decode::decode::<Defaults, f64>, [RHO_RATIO_FLOOR, 2.0])
);
