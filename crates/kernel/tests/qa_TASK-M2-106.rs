//! QA tests for TASK-M2-106, written from REQ-CHART-212 and R-991, not from the implementation: every f64
//! transcendental the CPU chart evaluator reaches goes through the pinned pure-software libm (0.2.16,
//! `force-soft-floats`), `sqrt` excepted, so a record is the same on every machine. The implementer's tests check `Real`'s
//! methods against libm (`real_pinned_bits`) and the source for stray calls (`transcendental_pin`); these check the
//! evaluator's own call sites, from outside, by what they compute:
//!
//! - `qa_m2106_call_sites_take_the_pin`: at f64, on fixed inputs and a seeded sweep, each call site below gives, bit for
//!   bit, its formula with every transcendental the pinned libm's, written here from the formula its source names:
//!   - `decode::config::hyperspherical`, `ρ̃ = cos α·(1, 0)`, `λ̃ = sin α·(cos β, sin β)` (`sin`, `cos`);
//!   - `decode::descriptor::angle`, dd_decoder §3.6 off the axis, `atan2(cross, dot)`, a value at or below `−π̂` read
//!     as `π̂` (`atan2`);
//!   - `SigmoidAlpha`'s forward `α_min + (π/2 − 2α_min)·σ(z)`, `σ(x) = 1/(1 + e⁻ˣ)` (`exp`), its inverse
//!     `logit(clamp((α − α_min)/(π/2 − 2α_min), ε_z, 1 − ε_z))`, `logit(s) = ln(s/(1 − s))` (`ln`), and its
//!     `log_det`, `ln(π/2 − 2α_min) − softplus(−z) − softplus(z)`, `softplus(x) = max(x, 0) + ln_1p(e^(−|x|))` (`ln`,
//!     `ln_1p`, `exp`).
//!
//!   Control: the same expectations with libm's `sin` moved one ulp up, R-991's own control on the measurement, are
//!   not met: the comparison sees one ulp of one function.
//! - `qa_m2106_inputs_tell_std_from_the_pin` (macOS, the measurement's reference machine): on the same inputs, each
//!   call site's expectation formed through the platform's library (`f64`'s inherent methods, the path before the pin)
//!   differs from the pinned one at one input at least, so the first test would fail on a call site left on std. Control:
//!   the "platform" expectation formed through libm too finds no such input.
//!
//! **Domain** (pitfalls §10): stateless call sites at f64 on the host; the cross-machine claim is the measurement's
//! (`cargo xtask measure libm-pin`), not these tests'.

use kernel::decode::config::hyperspherical;
use kernel::decode::descriptor::angle;
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Link, Literals, SigmoidAlpha};
use validation::negative_control;

/// The transcendentals an expectation is formed through.
#[derive(Clone, Copy)]
struct Lib {
    sin: fn(f64) -> f64,
    cos: fn(f64) -> f64,
    atan2: fn(f64, f64) -> f64,
    exp: fn(f64) -> f64,
    ln: fn(f64) -> f64,
    ln_1p: fn(f64) -> f64,
}

/// The pinned libm's.
const PINNED: Lib = Lib {
    sin: libm::sin,
    cos: libm::cos,
    atan2: libm::atan2,
    exp: libm::exp,
    ln: libm::log,
    ln_1p: libm::log1p,
};

/// The platform's, `f64`'s inherent methods: the path before the pin.
#[cfg(target_os = "macos")]
const PLATFORM: Lib = Lib {
    sin: f64::sin,
    cos: f64::cos,
    atan2: f64::atan2,
    exp: f64::exp,
    ln: f64::ln,
    ln_1p: f64::ln_1p,
};

/// SplitMix64: seeded, so the draws repeat from run to run.
struct Draws(u64);

impl Draws {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let z = (self.0 ^ (self.0 >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[lo, hi)`.
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 / (1u64 << 53) as f64)
    }
}

/// How many points the sweep draws per call site.
const SWEEP: usize = 1 << 16;

/// Inputs where libm 0.2.16 and a platform's library are known to differ (`sin` not correctly rounded in libm at the
/// first two, so a correctly rounded platform `sin` differs; the Mac's `sin`, `cos` and `atan2` at the others), read
/// as bits, then the sweep.
fn angles() -> Vec<f64> {
    let mut out: Vec<f64> = [
        0xc023_fa30_a118_13de_u64,
        0xc015_5b4e_049f_4aee,
        0xc021_cb09_47a6_86b4,
        0xbfe2_b074_a5ac_7510,
        0x401a_a809_f48e_44c0,
    ]
    .into_iter()
    .map(f64::from_bits)
    .collect();
    let mut draws = Draws(0x5EED_0106_0000_0001);
    out.extend((0..SWEEP).map(|_| draws.range(-8.0, 8.0)));
    out
}

/// `(cross, dot)` pairs off the axis: a known pair, then the sweep, neither coordinate zero.
fn pairs() -> Vec<(f64, f64)> {
    let mut out = vec![(
        f64::from_bits(0x4013_6493_9f24_f986),
        f64::from_bits(0xbff0_6cb5_a026_bb80),
    )];
    let mut draws = Draws(0x5EED_0106_0000_0002);
    out.extend(
        (0..SWEEP)
            .map(|_| (draws.range(-4.0, 4.0), draws.range(-4.0, 4.0)))
            .filter(|&(c, d)| c != 0.0 && d != 0.0),
    );
    out
}

/// The links' latent inputs.
fn latents() -> Vec<f64> {
    let mut draws = Draws(0x5EED_0106_0000_0003);
    let mut out = vec![0.0, 1.0, -1.0, 20.0, -20.0];
    out.extend((0..SWEEP).map(|_| draws.range(-30.0, 30.0)));
    out
}

/// `(where, got, want)` for every call site at every input, `want` formed through `lib`.
fn cases(lib: Lib) -> Vec<(String, Vec<u64>, Vec<u64>)> {
    let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    let mut out = Vec::new();
    let alphas = angles();
    for (i, &a) in alphas.iter().enumerate() {
        let b = alphas[(i * 7 + 3) % alphas.len()];
        let (rho, lambda) = hyperspherical::<f64>(a, b);
        let s = (lib.sin)(a);
        let want = [(lib.cos)(a), 0.0, s * (lib.cos)(b), s * (lib.sin)(b)];
        out.push((
            format!("hyperspherical({a:e}, {b:e})"),
            bits(&[rho[0], rho[1], lambda[0], lambda[1]]),
            bits(&want),
        ));
    }
    let pi = std::f64::consts::PI;
    for (c, d) in pairs() {
        let a = (lib.atan2)(c, d);
        let want = if a <= -pi { pi } else { a };
        out.push((
            format!("angle({c:e}, {d:e})"),
            bits(&[angle::<f64>(c, d)]),
            bits(&[want]),
        ));
    }
    let (a_min, eps_z) = (
        <f64 as ChartConstants>::ALPHA_MIN,
        <f64 as ChartConstants>::EPS_Z,
    );
    let span = <f64 as Literals>::LIT_6 - 2.0 * a_min;
    let sigmoid = |x: f64| 1.0 / (1.0 + (lib.exp)(-x));
    let softplus = |x: f64| x.max(0.0) + (lib.ln_1p)((lib.exp)(-x.abs()));
    for z in latents() {
        let [alpha] = <SigmoidAlpha as Link<1, 1>>::forward([z]);
        let want = a_min + span * sigmoid(z);
        out.push((
            format!("SigmoidAlpha::forward({z:e})"),
            bits(&[alpha]),
            bits(&[want]),
        ));
        let [back] = <SigmoidAlpha as Link<1, 1>>::inverse([alpha]);
        let s = ((alpha - a_min) / span).max(eps_z).min(1.0 - eps_z);
        out.push((
            format!("SigmoidAlpha::inverse({alpha:e})"),
            bits(&[back]),
            bits(&[(lib.ln)(s / (1.0 - s))]),
        ));
        let det = <SigmoidAlpha as Link<1, 1>>::log_det([z]);
        let want = (lib.ln)(span) + -softplus(-z) + -softplus(z);
        out.push((
            format!("SigmoidAlpha::log_det({z:e})"),
            bits(&[det]),
            bits(&[want]),
        ));
    }
    out
}

/// Every call site gives its formula through `lib`, bit for bit.
fn check_pinned(lib: Lib) {
    let cases = cases(lib);
    for (site, got, want) in &cases {
        assert_eq!(
            got, want,
            "{site} is not the pinned libm's formula, bit for bit: {got:x?} against {want:x?}"
        );
    }
    eprintln!(
        "qa_m2106_call_sites_take_the_pin: {} cases bit for bit",
        cases.len()
    );
}

#[test]
fn qa_m2106_call_sites_take_the_pin() {
    check_pinned(PINNED);
}

/// libm's `sin` one ulp up, R-991's control on the measurement.
#[cfg(feature = "controls")]
fn sin_up(x: f64) -> f64 {
    f64::from_bits(libm::sin(x).to_bits() + 1)
}

negative_control!(
    qa_m2106_call_sites_take_the_pin,
    "the expectations with libm's sin one ulp up must not be met",
    expected = "is not the pinned libm's formula",
    check_pinned(Lib {
        sin: sin_up,
        ..PINNED
    })
);

/// At each call site, the platform's formula differs from the pinned one at one input at least: returns the count per
/// site name.
#[cfg(target_os = "macos")]
fn check_discriminates(platform: Lib) {
    use std::collections::BTreeMap;
    let pinned = cases(PINNED);
    let other = cases(platform);
    let mut differ: BTreeMap<&str, usize> = BTreeMap::new();
    for ((site, _, want), (_, _, alt)) in pinned.iter().zip(&other) {
        let name = site.split('(').next().unwrap_or(site);
        *differ.entry(name).or_default() += usize::from(want != alt);
    }
    eprintln!("qa_m2106_inputs_tell_std_from_the_pin: inputs where the platform differs, per site: {differ:?}");
    for (name, n) in &differ {
        assert!(
            *n > 0,
            "at {name} no input tells the platform's library from the pinned libm: {differ:?}"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn qa_m2106_inputs_tell_std_from_the_pin() {
    check_discriminates(PLATFORM);
}

#[cfg(target_os = "macos")]
negative_control!(
    qa_m2106_inputs_tell_std_from_the_pin,
    "a platform expectation formed through libm must tell no input apart",
    expected = "no input tells the platform's library from the pinned libm",
    check_discriminates(PINNED)
);
