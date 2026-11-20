//! What `decode_join.rs` and `ic_descriptor.rs` share: the two precisions, the selections, the fuzzed domain, the
//! golden IC's fixture (`fixtures/golden_ic/z0.json`) and REQ-PAY-095's proposed tolerances.

#![allow(dead_code)] // Each test file uses part of the module.

use std::cell::RefCell;
use std::path::Path;

use kernel::decode::{self, Decoded, Latent};
use kernel::generated::links::*;
use kernel::payload::{ICDescriptorOf, PayloadReal};
use proptest::prelude::*;
use validation::prop;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the decoder is instantiated at, with conversions to and from f64 for the checks.
pub trait P: LinkReal + PayloadReal {
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    /// The machine epsilon, as f64.
    fn eps() -> f64;
    /// The value whose bits the fixture gives at this precision (its `f64` or `f32` key).
    fn from_fixture(v: &serde_json::Value) -> Self;
    /// The bits, widened to u64.
    fn bits(self) -> u64;
    /// The distance from `a` to `b` in units in the last place of this precision: the count of representable values
    /// between them, `+0` and `−0` one value.
    fn ulps(a: Self, b: Self) -> u64;
}

/// The bits a fixture entry gives under `key`, from its `0x…` hex string.
fn hex(v: &serde_json::Value, key: &str) -> u64 {
    let s = v[key]
        .as_str()
        .unwrap_or_else(|| panic!("fixture value {v} has no `{key}`"));
    u64::from_str_radix(s.trim_start_matches("0x"), 16)
        .unwrap_or_else(|e| panic!("fixture `{key}` {s} is not hex: {e}"))
}

/// A float's bits mapped to a line on which consecutive values are consecutive integers, `±0` both at 0.
fn ordered(bits: u64, sign: u64) -> i128 {
    let magnitude = (bits & !sign) as i128;
    if bits & sign != 0 {
        -magnitude
    } else {
        magnitude
    }
}

impl P for f64 {
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn eps() -> f64 {
        f64::EPSILON
    }
    fn from_fixture(v: &serde_json::Value) -> Self {
        f64::from_bits(hex(v, "f64"))
    }
    fn bits(self) -> u64 {
        self.to_bits()
    }
    fn ulps(a: Self, b: Self) -> u64 {
        let sign = 1 << 63;
        (ordered(a.to_bits(), sign) - ordered(b.to_bits(), sign)).unsigned_abs() as u64
    }
}

impl P for f32 {
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        self as f64
    }
    fn eps() -> f64 {
        f32::EPSILON as f64
    }
    fn from_fixture(v: &serde_json::Value) -> Self {
        let bits = hex(v, "f32");
        f32::from_bits(u32::try_from(bits).expect("an f32's bits fit 32"))
    }
    fn bits(self) -> u64 {
        self.to_bits().into()
    }
    fn ulps(a: Self, b: Self) -> u64 {
        let sign = 1 << 31;
        (ordered(a.to_bits().into(), sign) - ordered(b.to_bits().into(), sign)).unsigned_abs()
            as u64
    }
}

/// `z` (given in f64) at `R`.
pub fn at<R: P, const N: usize>(z: [f64; N]) -> [R; N] {
    z.map(R::of)
}

/// The latent vector `z` at `R`.
pub fn latent<R: P>(z: [f64; 8]) -> Latent<R> {
    Latent::new(at::<R, 8>(z))
}

// ── Selections ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The default selection with the `stick_breaking` mass link.
pub type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

/// Every tanh link, with the `stick_breaking` mass link.
pub type Tanhs = Select<StickBreaking, TanhAlpha, TanhBeta, TanhQ, TanhQ, TanhQ, TanhQ>;

/// Every softsign link, with the default mass link.
pub type Softsigns =
    Select<SoftmaxTanh, SoftsignAlpha, SoftsignBeta, SoftsignQ, SoftsignQ, SoftsignQ, SoftsignQ>;

/// A decode entry at `R`, behind a function pointer, so a control runs the same check on a broken one.
pub type Entry<R> = fn(&Latent<R>) -> Decoded<R>;

/// Every registered link in some selection, each entry named: the defaults, `stick_breaking`, the tanh and the
/// softsign links.
pub fn selections<R: P>() -> [(&'static str, Entry<R>); 4] {
    [
        ("defaults", decode::decode::<Defaults, R>),
        ("stick_breaking", decode::decode::<Stick, R>),
        ("tanh", decode::decode::<Tanhs, R>),
        ("softsign", decode::decode::<Softsigns, R>),
    ]
}

// ── The domain ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A control drawn from the centre, the transition into saturation, deep saturation, or anywhere up to 10³⁸ in
/// magnitude: every finite `z` is accepted (R-407), and the widest draws stay finite at f32.
pub fn control() -> impl Strategy<Value = f64> {
    prop_oneof![
        -2.0f64..=2.0,
        -20.0f64..=20.0,
        -60.0f64..=60.0,
        (any::<bool>(), 0.0f64..=38.0)
            .prop_map(|(neg, e)| if neg { -1.0 } else { 1.0 } * 10f64.powf(e)),
    ]
}

/// A control at least 0.1 from 0, of either sign: `z_μ1` drawn so, `μ₁ = μ_max·tanh(z_μ1)` is at least 0.5 in
/// magnitude under `softmax_tanh`, so `m₁/m₀ = e^μ₁` is at least `e^0.5` from 1, and the masses are unequal (PIT-9).
pub fn off_centre() -> impl Strategy<Value = f64> {
    (
        any::<bool>(),
        prop_oneof![0.1f64..=2.0, 2.0f64..=60.0, 1.0f64..=1e38],
    )
        .prop_map(|(neg, x)| if neg { -x } else { x })
}

/// A latent vector over the whole domain.
pub fn any_z() -> impl Strategy<Value = [f64; 8]> {
    [
        control(),
        control(),
        control(),
        control(),
        control(),
        control(),
        control(),
        control(),
    ]
}

/// A latent vector whose inner pair's masses are unequal under either mass link (PIT-9): `z_μ1` [`off_centre`].
pub fn unequal_z() -> impl Strategy<Value = [f64; 8]> {
    let c = || control().boxed();
    [c(), c(), c(), c(), c(), c(), off_centre().boxed(), c()]
}

/// Fixed controls besides the fuzzed ones: the centre, the transition and saturation, at each sign.
pub const FIXED: [f64; 11] = [
    -1e38, -1e3, -40.0, -8.0, -1.0, 0.0, 1.3, 8.0, 40.0, 1e3, 1e38,
];

/// The fixed latent vectors: each fixed control in every slot alone, and every slot at it together.
pub fn fixed_z() -> Vec<[f64; 8]> {
    let mut zs = Vec::new();
    for &c in &FIXED {
        zs.push([c; 8]);
        for k in 0..8 {
            let mut z = [0.0; 8];
            z[k] = c;
            zs.push(z);
        }
    }
    zs
}

/// Runs `check` on each fixed `z` and on fuzzed `z` drawn by `draws`.
pub fn over<S: Strategy<Value = [f64; 8]>>(draws: S, check: impl Fn([f64; 8])) {
    for z in fixed_z() {
        check(z);
    }
    prop::run(&draws, |z| {
        check(z);
        Ok(())
    });
}

/// The worst residual of each named quantity a check saw, in units of `eps` or ulps.
#[derive(Default)]
pub struct Worst(RefCell<Vec<(String, f64)>>);

impl Worst {
    pub fn see(&self, name: &str, v: f64) {
        let mut w = self.0.borrow_mut();
        match w.iter_mut().find(|(n, _)| n == name) {
            Some((_, x)) => *x = x.max(v),
            None => w.push((name.to_owned(), v)),
        }
    }
    pub fn print(&self, test: &str) {
        for (n, v) in self.0.borrow().iter() {
            eprintln!("{test}: worst {n} = {v:.3}");
        }
    }
}

// ── The golden IC ───────────────────────────────────────────────────────────────────────────────────────────────

/// The golden IC's fixture, `fixtures/golden_ic/z0.json`: each value's name, group, closed form, and bits rounded to
/// f64 and f32.
pub fn golden() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden_ic/z0.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The fixture's values, in its order.
pub fn golden_values(g: &serde_json::Value) -> Vec<serde_json::Value> {
    g["values"].as_array().expect("`values` is a list").clone()
}

/// The name of a fixture value.
pub fn name(v: &serde_json::Value) -> &str {
    v["name"].as_str().expect("a value has a name")
}

/// The descriptor's field `field` of `d`, by its ledger name (dd_generation_root §3.6).
pub fn field<R: PayloadReal>(d: &ICDescriptorOf<R>, field: &str) -> R {
    match field {
        "m0" => d.m0,
        "m1" => d.m1,
        "m2" => d.m2,
        "q_mass" => d.q_mass,
        "rho_mag" => d.rho_mag,
        "lambda_mag" => d.lambda_mag,
        "rho_ratio" => d.rho_ratio,
        "rho_angle" => d.rho_angle,
        "K_0" => d.K_0,
        "V_0" => d.V_0,
        "virial_ratio" => d.virial_ratio,
        "r_min_pair_0" => d.r_min_pair_0,
        _ => panic!("`{field}` is not an ICDescriptor field"),
    }
}

/// REQ-PAY-095's proposal (R-71, R-421): each golden descriptor field that is not 0 at rest is within this many ulps of
/// its exact value rounded to the precision, the same count at f64 and at f32; the fields 0 at rest, `K_0` and
/// `virial_ratio`, take none and are compared bit-exact. The measured distances, the evidence, are in
/// `ic_descriptor.rs`'s header. Unconfirmed until the M2 gate.
pub const DESCRIPTOR_ULPS: [(&str, u64); 10] = [
    ("m0", 4),
    ("m1", 4),
    ("m2", 4),
    ("q_mass", 4),
    ("rho_mag", 4),
    ("lambda_mag", 4),
    ("rho_ratio", 4),
    ("rho_angle", 4),
    ("V_0", 4),
    ("r_min_pair_0", 4),
];

/// The descriptor's fields that are 0 at the golden IC because it is at rest (R-421), compared bit-exact.
pub const ZERO_AT_REST: [&str; 2] = ["K_0", "virial_ratio"];

/// A field's proposed tolerance in ulps, or `None` for a field 0 at rest.
pub fn descriptor_ulps(field: &str) -> Option<u64> {
    if ZERO_AT_REST.contains(&field) {
        return None;
    }
    let (_, n) = DESCRIPTOR_ULPS
        .iter()
        .find(|(f, _)| *f == field)
        .unwrap_or_else(|| panic!("`{field}` has no REQ-PAY-095 tolerance"));
    Some(*n)
}

/// A descriptor field against the fixture: bit-exact if 0 at rest, else within its REQ-PAY-095 ulps.
pub fn check_field<R: P>(field: &str, got: R, want: R, worst: &Worst) {
    match descriptor_ulps(field) {
        None => assert_eq!(
            got.bits(),
            want.bits(),
            "the golden descriptor's {field} = {got:?} at {}, not bit-exact {want:?}",
            R::NAME
        ),
        Some(n) => {
            let ulps = R::ulps(got, want);
            worst.see(
                &format!("descriptor {field} at {} (ulps)", R::NAME),
                ulps as f64,
            );
            assert!(
                ulps <= n,
                "the golden descriptor's {field} = {got:?} at {}, {ulps} ulps from {want:?}",
                R::NAME
            );
        }
    }
}
