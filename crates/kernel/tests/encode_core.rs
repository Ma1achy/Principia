//! Encode `E = (block inverses) ∘ C` on the CPU, at f64 and f32, the one source's two instantiations (`kernel::encode`;
//! TASK-M2-15). The GPU-f32 build's checks are `engine/tests/encode_gpu.rs`.
//! - `encode_block_inverses`: forward substituted into inverse for fuzzed interior `z`, under the default links and
//!   under a second selection: the Jacobi map, `α` and `β` recover the decoder's own, and each block recovers its `z`;
//!   a `|q| > q_max` input sets `lookup_clamped` (REQ-ENC-007).
//! - `encode_clamp_edges`: each control's clamp bit at its edge, one step either side, for every link of both
//!   selections; a clamped input's `z` is the clamp's image; the simplex corner gives no NaN (REQ-ENC-007, REQ-ENC-014).
//! - `encode_collinear_beta`: dd_encode §5 test 10 (R-438) at the Euler landmarks, `λ̃_y ∈ {−1e-13, −0, +0, +1e-13}`,
//!   and the pole landmark on its own (REQ-ENC-007).
//! - `encode_t1_gauge`: dd_encode §5 test 2, `E(g·x) = E(x)` over random `g` per §3.3, deadband states under turns,
//!   and the mirror tie twice (REQ-ENC-003).
//! - `encode_notices`: canonical inputs raise no notice; each operation forced alone raises exactly its own
//!   (REQ-ENC-014; `lookup_rescaled`'s raising waits on RQ-295, so its value `λ` is checked).
//! - `stage_encode_native`: the compute entry points `kernel::stage::encode::{encode, beta}`, called natively, write
//!   each sample's encode and `β` in their word layout, and nothing past the last sample.
//!
//! **Tolerances** (applied per R-369: a test decision, as dd_decoder §5 test 1's "precision-appropriate tolerance"),
//! each a multiple of the precision's epsilon, set from the worst case these tests measure and print, with headroom.
//! `z` is compared only on the interior, `|z| ≤ 4`, where `d logit/ds` and `d artanh/dt` are bounded (dd_encode §3.5).
//! `ε_phys` (REQ-ENC-024) is not yet calibrated; test 10's "within `ε_phys`" components are held to a multiple of the
//! precision's epsilon, tighter than any round-trip tolerance over interior `z`.
//!
//! **Domain** (pitfalls §10): encode alone, on states held fixed; no trajectory.

use kernel::decode::config::reconstruct;
use kernel::decode::mass::from_masses;
use kernel::decode::momentum::to_particles;
use kernel::decode::{decode, Ic, Latent};
use kernel::encode::blocks::{alpha, beta, clamps, guard, jacobi, Ends, SimplexClamp};
use kernel::encode::{encode, Encoded, Report};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{
    Alpha, Beta, Defaults, Link, LinkReal, Momentum, Select, Selection, SigmoidAlpha, SigmoidBeta,
    SigmoidQ, SoftmaxTanh, SoftsignAlpha, SoftsignBeta, SoftsignQ, StickBreaking, TanhAlpha,
    TanhBeta, TanhQ,
};
use kernel::payload::PayloadReal;
use kernel::real::IeeeBits;
use kernel::stage::decode::{ic_words, write_ic};
use kernel::stage::encode::{
    beta as beta_entry, encode as encode_entry, encode_words, notices, read_encoded, write_encoded,
    EncodeOut,
};
use spirv_std::glam::UVec3;
use validation::decode_f32_eps::SplitMix;
use validation::gauge::{uniform, Gauge};
use validation::negative_control;

/// The second selection: the edge-reaching simplex link and the tanh and softsign links, so that every inverse kind
/// is exercised (dd_encode §4 seam 2/13: a link swapped, encode's inverse swaps with it).
type Alt = Select<StickBreaking, TanhAlpha, SoftsignBeta, TanhQ, SoftsignQ, SigmoidQ, TanhQ>;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision encode is instantiated at, with conversions to and from f64 for the checks.
trait P: LinkReal + IeeeBits + PayloadReal + std::fmt::LowerExp {
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    fn eps() -> f64;
    fn up(self) -> Self;
    fn down(self) -> Self;
    fn bits(self) -> u64;
}

impl P for f64 {
    fn of(x: f64) -> f64 {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn eps() -> f64 {
        f64::EPSILON
    }
    fn up(self) -> f64 {
        self.next_up()
    }
    fn down(self) -> f64 {
        self.next_down()
    }
    fn bits(self) -> u64 {
        self.to_bits()
    }
}

impl P for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
    fn f(self) -> f64 {
        f64::from(self)
    }
    fn eps() -> f64 {
        f64::from(f32::EPSILON)
    }
    fn up(self) -> f32 {
        self.next_up()
    }
    fn down(self) -> f32 {
        self.next_down()
    }
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
}

fn ic_of<R: P>(x: &Ic<f64>) -> Ic<R> {
    Ic {
        m: x.m.map(R::of),
        r: x.r.map(|v| v.map(R::of)),
        p: x.p.map(|v| v.map(R::of)),
    }
}

/// A random interior `z`, each control uniform on `[−span, span)`.
fn random_z<R: P>(rng: &mut SplitMix, span: f64) -> [R; 8] {
    std::array::from_fn(|_| R::of(uniform(rng, -span, span)))
}

/// The bits of every output of `e`: `z`, `α`, `β`, `λ` and the notices.
fn out_bits<R: P>(e: &Encoded<R>) -> Vec<u64> {
    let r = &e.report;
    e.z.z
        .iter()
        .chain(&[e.blocks.alpha, e.blocks.beta, r.lambda])
        .map(|x| x.bits())
        .chain([
            u64::from(r.coincident),
            u64::from(r.clamped_controls),
            u64::from(r.mirrored),
        ])
        .collect()
}

/// The interior span of the fuzzed controls: `|z| ≤ 4`, inside every clamp (the module's note).
const SPAN: f64 = 4.0;

// ── encode_block_inverses (REQ-ENC-007) ─────────────────────────────────────────────────────────────────────────

/// The worst errors of the forward-into-inverse check, in eps: the Jacobi map, `α` and `β`, then `z`'s config,
/// momentum and mass blocks.
#[derive(Debug, Default)]
struct BlockWorst {
    jacobi: f64,
    angles: f64,
    config: f64,
    momentum: f64,
    mass: f64,
}

/// Limits on [`BlockWorst`], in eps.
struct BlockLimits {
    jacobi: f64,
    angles: f64,
    z: f64,
}

/// `|a − b|` over `scale`, in eps.
fn err<R: P>(a: R, b: R, scale: f64) -> f64 {
    (a.f() - b.f()).abs() / (scale * R::eps())
}

/// The larger of `a` and `b`, or NaN if either is (pitfalls §9): a worst-case fold that `f64::max` would make drop a
/// NaN error, so a NaN output fails the limit it is held to.
fn worse(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// The forward-into-inverse check on one decoded `z`: the Jacobi map on the decoded state recovers the decoder's `ρ̃`,
/// `λ̃`, `p_ρ` and `p_λ`; `α` and `β` from the decoder's `ρ̃` and `λ̃` recover its own; and the encode of the decoded
/// state recovers each block of `z`, with no notice.
fn check_block<S, R>(z: [R; 8], worst: &mut BlockWorst, limits: &BlockLimits)
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: P,
{
    let d = decode::<S, R>(&Latent::new(z));
    let j = jacobi(&d.ic);
    let pairs = [
        (j.rho_t, d.config.rho_t),
        (j.lambda_t, d.config.lambda_t),
        (j.p_rho, d.momentum.p_rho),
        (j.p_lambda, d.momentum.p_lambda),
    ];
    for (got, want) in pairs {
        for k in 0..2 {
            worst.jacobi = worse(worst.jacobi, err(got[k], want[k], 1.0));
        }
    }
    let (a, b) = (
        alpha(d.config.rho_t, d.config.lambda_t),
        beta(d.config.lambda_t),
    );
    worst.angles = worse(
        worse(worst.angles, err(a, d.config.alpha, 1.0)),
        err(b, d.config.beta, 1.0),
    );
    let e = encode::<S, R>(&d.ic);
    for (k, (got, want)) in e.z.z.iter().zip(z).enumerate() {
        let x = err(*got, want, 1.0 + want.f().abs());
        let slot = match k {
            0 | 1 => &mut worst.config,
            2..=5 => &mut worst.momentum,
            _ => &mut worst.mass,
        };
        *slot = worse(*slot, x);
    }
    let at = format!("z = {z:?}");
    assert!(
        worst.jacobi <= limits.jacobi,
        "the Jacobi map misses the decoder's by {:.1} eps at {at}",
        worst.jacobi
    );
    assert!(
        worst.angles <= limits.angles,
        "α or β misses the decoder's by {:.1} eps at {at}",
        worst.angles
    );
    for (name, x) in [
        ("config", worst.config),
        ("momentum", worst.momentum),
        ("mass", worst.mass),
    ] {
        assert!(
            x <= limits.z,
            "the {name} block recovers z only to {x:.1} eps at {at}"
        );
    }
    assert!(
        !e.report.lookup_clamped() && !e.report.lookup_mirrored(),
        "an interior decode raised a notice, {:?}, at {at}",
        e.report
    );
}

fn block_inverses<S, R>(seed: u64, limits: &BlockLimits) -> BlockWorst
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: P,
{
    let mut rng = SplitMix::new(seed);
    let mut worst = BlockWorst::default();
    for _ in 0..20_000 {
        check_block::<S, R>(random_z(&mut rng, SPAN), &mut worst, limits);
    }
    worst
}

const BLOCK_LIMITS: BlockLimits = BlockLimits {
    jacobi: 8.0,
    angles: 4.0,
    z: 4096.0,
};

/// A canonical state of `z` under the defaults with `p_ρ` set to `p_rho`.
fn with_p_rho<R: P>(z: [R; 8], p_rho: [R; 2]) -> Ic<R> {
    let d = decode::<Defaults, R>(&Latent::new(z));
    with_momenta(z, p_rho, d.momentum.p_lambda)
}

/// A canonical state of `z` under the defaults with its momenta set to `p_rho` and `p_lambda`.
fn with_momenta<R: P>(z: [R; 8], p_rho: [R; 2], p_lambda: [R; 2]) -> Ic<R> {
    let d = decode::<Defaults, R>(&Latent::new(z));
    let masses = from_masses(d.ic.m);
    Ic {
        p: to_particles(&masses, p_rho, p_lambda),
        ..d.ic
    }
}

/// `|q| > q_max` in `p_ρ`'s components, either sign: `lookup_clamped` and exactly the control's bit; and `q_max·(1 − ½ε)`,
/// past σ's clamp, too; a `q` well inside raises nothing.
fn check_q_clamp<R: P>() {
    let z = [R::zero(); 8];
    let q = R::Q_MAX;
    let cases = [
        ([R::of(2.5), R::zero()], 1u32 << 2),
        ([R::zero(), R::of(-2.5)], 1 << 3),
        ([q + q, R::zero()], 1 << 2),
        ([R::zero(), q * (R::one() - R::EPS_Q / R::of(2.0))], 1 << 3),
        ([R::of(1.9), R::of(-1.9)], 0),
    ];
    for (p_rho, bits) in cases {
        let e = encode::<Defaults, R>(&with_p_rho(z, p_rho));
        assert_eq!(
            e.report.clamped_controls, bits,
            "p_ρ = {p_rho:?} clamps controls {:#b}, not {bits:#b}",
            e.report.clamped_controls
        );
        assert_eq!(
            e.report.lookup_clamped(),
            bits != 0,
            "lookup_clamped is {} at p_ρ = {p_rho:?}",
            e.report.lookup_clamped()
        );
    }
    // `p_λ`'s components, `q₂` and `q₃`, each its own bit, and with `p_ρ`'s.
    let lambda_cases = [
        ([R::zero(); 2], [R::of(2.5), R::zero()], 1u32 << 4),
        ([R::zero(); 2], [R::zero(), R::of(-2.5)], 1 << 5),
        (
            [R::of(2.5), R::zero()],
            [R::zero(), R::of(2.5)],
            (1 << 2) | (1 << 5),
        ),
        (
            [R::zero(), R::of(-2.5)],
            [R::of(-2.5), R::zero()],
            (1 << 3) | (1 << 4),
        ),
    ];
    for (p_rho, p_lambda, bits) in lambda_cases {
        let e = encode::<Defaults, R>(&with_momenta(z, p_rho, p_lambda));
        assert_eq!(
            e.report.clamped_controls, bits,
            "p_ρ = {p_rho:?}, p_λ = {p_lambda:?} clamps controls {:#b}, not {bits:#b}",
            e.report.clamped_controls
        );
    }
    // The masses' bits 6 and 7, `μ₁ = log(m₁/m₀)` and `μ₂` past `±(1 − ε_μ)·μ_max = ±5(1 − 10⁻⁶)`: `log(0.99/0.004)`
    // is 5.5 and `log(0.006/0.004)` 0.4; `log(0.5/10⁻³)` and `log(0.499/10⁻³)` are 6.2.
    let base = with_p_rho(z, [R::zero(); 2]);
    for (m, bits) in [
        ([0.004, 0.99, 0.006], 1u32 << 6),
        ([0.004, 0.006, 0.99], 1 << 7),
        ([1e-3, 0.5, 0.499], (1 << 6) | (1 << 7)),
        ([0.3, 0.3, 0.4], 0),
    ] {
        let x = Ic {
            m: m.map(R::of),
            ..base
        };
        let e = encode::<Defaults, R>(&x);
        assert_eq!(
            e.report.clamped_controls, bits,
            "m = {m:?} clamps controls {:#b}, not {bits:#b}",
            e.report.clamped_controls
        );
    }
}

#[test]
fn encode_block_inverses() {
    let runs = [
        (
            "defaults f64",
            block_inverses::<Defaults, f64>(1, &BLOCK_LIMITS),
        ),
        (
            "defaults f32",
            block_inverses::<Defaults, f32>(2, &BLOCK_LIMITS),
        ),
        ("alt f64", block_inverses::<Alt, f64>(3, &BLOCK_LIMITS)),
        ("alt f32", block_inverses::<Alt, f32>(4, &BLOCK_LIMITS)),
    ];
    for (name, w) in runs {
        eprintln!("encode_block_inverses {name}: worst (eps) {w:?}");
    }
    check_q_clamp::<f64>();
    check_q_clamp::<f32>();
}

negative_control!(
    encode_block_inverses,
    "a z the decode does not reach must fail the block's recovery",
    expected = "block recovers z only to",
    {
        // `z_μ1 = 8` is past the mass clamp's image, `atanh(1 − ε_μ) ≈ 7.25`: the inverse returns the image.
        let mut z = [0.0f64; 8];
        z[6] = 8.0;
        check_block::<Defaults, f64>(z, &mut BlockWorst::default(), &BLOCK_LIMITS);
    }
);

// ── encode_clamp_edges (REQ-ENC-007, REQ-ENC-014) ───────────────────────────────────────────────────────────────

/// `L`'s clamp bit at the ends of the range the decode reaches, `lo` and `hi`, and one step past each: clear on the
/// range, set past it, at the codomain's ends and at a NaN. And the bit agrees with the inverse's own clamp outside a
/// band of `8·eps·max(|a|, |b|)` about each end, the rounding of the clamp's argument (`kernel::encode::blocks`' note):
/// past the band the inverse returns the clamp's image, bit for bit, as at the codomain's ends; inside it, it does not.
/// Returns the range.
fn check_edges<L, R>(name: &str) -> (R, R)
where
    L: Link<1, 1>,
    L::Codomain: Ends,
    R: P,
{
    let (a, b) = <L::Codomain as Ends>::ends::<R>();
    let [lo] = L::forward(L::inverse([a]));
    let [hi] = L::forward(L::inverse([b]));
    let at = |v: R| format!("{name} at v = {v:e}");
    for (v, clamped) in [
        (lo, false),
        (hi, false),
        (lo.down(), true),
        (hi.up(), true),
        (a, true),
        (b, true),
        (R::nan(), true),
    ] {
        assert_eq!(
            clamps::<L, R>(v),
            clamped,
            "the clamp bit is {} {}",
            !clamped,
            at(v)
        );
    }
    let band = R::of(8.0 * R::eps() * a.f().abs().max(b.f().abs()));
    let image = |v: R| L::inverse([v])[0].bits();
    for (v, end, clamped) in [
        (lo - band, a, true),
        (hi + band, b, true),
        (lo + band, a, false),
        (hi - band, b, false),
    ] {
        assert_eq!(
            image(v) == image(end),
            clamped,
            "the inverse's own clamp {} {}",
            if clamped { "did not act" } else { "acted" },
            at(v)
        );
        assert_eq!(
            clamps::<L, R>(v),
            clamped,
            "the clamp bit is {} {}",
            !clamped,
            at(v)
        );
    }
    (lo, hi)
}

fn edges_at<R: P>() {
    check_edges::<SigmoidAlpha, R>("sigmoid_alpha");
    check_edges::<TanhAlpha, R>("tanh_alpha");
    check_edges::<SoftsignAlpha, R>("softsign_alpha");
    check_edges::<SigmoidBeta, R>("sigmoid_beta");
    check_edges::<TanhBeta, R>("tanh_beta");
    check_edges::<SoftsignBeta, R>("softsign_beta");
    check_edges::<SigmoidQ, R>("sigmoid_q");
    check_edges::<TanhQ, R>("tanh_q");
    check_edges::<SoftsignQ, R>("softsign_q");
}

/// The ends the codomains give: `α`'s `α_min` and the registry's `π/2`, `β`'s 0 and `π̂`, the momenta's `±q_max`.
fn check_ends<R: P>() {
    let pi = R::LIT_7;
    let cases = [
        (Alpha::ends::<R>(), (R::ALPHA_MIN, R::LIT_6), "α"),
        (Beta::ends::<R>(), (R::zero(), pi), "β"),
        (Momentum::ends::<R>(), (-R::Q_MAX, R::Q_MAX), "q"),
    ];
    for (got, want, name) in cases {
        assert!(
            got.0.bits() == want.0.bits() && got.1.bits() == want.1.bits(),
            "{name}'s ends are {got:?}, not {want:?}"
        );
    }
}

/// A mass point `(m₀, r·m₀, 1 − m₀ − m₁)`, `m₀` a power of two, so `m₁/m₀ = r` exactly: `m₀ = 2⁻⁸` for `r > 1`, `¼`
/// otherwise, so that `m₂/m₀` stays well inside `e^{±cap}` for `r` near either edge.
fn ratio_point<R: P>(r: R) -> [R; 3] {
    let m0 = R::of(if r > R::one() { 1.0 / 256.0 } else { 0.25 });
    let m1 = r * m0;
    [m0, m1, R::one() - m0 - m1]
}

/// The `r` nearest `e^{target}` whose `log` is `target` exactly, searched over 64 steps either side; `None` if none.
fn exact_log<R: P>(target: R) -> Option<R> {
    let mut r = Float::exp(target);
    for _ in 0..64 {
        r = r.down();
    }
    (0..128)
        .map(|_| {
            r = r.up();
            r
        })
        .find(|r| Float::ln(*r) == target)
}

use spirv_std::num_traits::Float;

/// `softmax_tanh`'s clamp bits: `log(m₁/m₀)` exactly at `±cap` is not clamped, one step past is; each control alone.
fn check_softmax_edges<R: P>() {
    let cap = (R::one() - R::EPS_MU) * R::MU_MAX;
    for (sign, label) in [(R::one(), "+cap"), (-R::one(), "−cap")] {
        let target = sign * cap;
        let mut edge = exact_log(target).unwrap_or_else(|| panic!("no r with log r = {label}"));
        // The outermost `r` whose `log` is the edge exactly, and the next one out, whose `log` is past it.
        let out = |r: R| if sign > R::zero() { r.up() } else { r.down() };
        while Float::ln(out(edge)) == target {
            edge = out(edge);
        }
        let past = out(edge);
        let at_edge = ratio_point(edge);
        let past_edge = ratio_point(past);
        assert_eq!(
            <SoftmaxTanh as SimplexClamp>::clamps(at_edge),
            0,
            "softmax_tanh clamps at log(m₁/m₀) = {label} exactly"
        );
        assert_eq!(
            <SoftmaxTanh as SimplexClamp>::clamps(past_edge),
            1,
            "softmax_tanh does not clamp z_μ1 alone one step past {label}"
        );
        // The same edge on m₂: the second bit alone.
        let swap = |m: [R; 3]| [m[0], m[2], m[1]];
        assert_eq!(
            <SoftmaxTanh as SimplexClamp>::clamps(swap(past_edge)),
            2,
            "softmax_tanh does not clamp z_μ2 alone one step past {label}"
        );
        assert_eq!(
            SoftmaxTanh::inverse(past_edge)[0].bits(),
            SoftmaxTanh::inverse(ratio_point(Float::exp(sign * R::of(10.0))))[0].bits(),
            "softmax_tanh's own clamp did not act one step past {label}"
        );
    }
}

/// `stick_breaking`'s clamp bits on each of its arguments, `2(m₁ + m₂) − 1` and `2m₂/(m₁ + m₂) − 1`, inside and past
/// `±(1 − ε_μ)²`.
fn check_stick_edges<R: P>() {
    let keep = R::one() - R::EPS_MU;
    let cap = (keep * keep).f();
    // `s = m₁ + m₂` with `2s − 1` a little inside or past `±cap`, `u = ½`: bit 0 alone.
    let point = |s: f64, u: f64| -> [R; 3] {
        let (s, u) = (R::of(s), R::of(u));
        [R::one() - s, s * (R::one() - u), s * u]
    };
    let margin = 64.0 * R::eps();
    let cases = [
        (point((1.0 + cap * (1.0 - margin)) / 2.0, 0.5), 0u32),
        (point((1.0 + cap * (1.0 + margin)) / 2.0, 0.5), 1),
        (point((1.0 - cap * (1.0 + margin)) / 2.0, 0.5), 1),
        (point(0.5, (1.0 + cap * (1.0 - margin)) / 2.0), 0),
        (point(0.5, (1.0 + cap * (1.0 + margin)) / 2.0), 2),
        (point(0.5, (1.0 - cap * (1.0 + margin)) / 2.0), 2),
        (point(0.5, (1.0 - cap * (1.0 - margin)) / 2.0), 0),
    ];
    for (m, bits) in cases {
        assert_eq!(
            <StickBreaking as SimplexClamp>::clamps(m),
            bits,
            "stick_breaking clamps {:#b} at m = {m:?}, not {bits:#b}",
            <StickBreaking as SimplexClamp>::clamps(m)
        );
    }
}

/// The simplex's corners and edges through [`guard`]: every mass that is not positive is the smallest positive normal,
/// so the inverse sees no `0/0`; each gives a finite `z` with the clamp flagged, the corner `m₂ = 1` the `m₀ = m₁` limit
/// `z_μ1 = 0`.
fn check_corners<R: P>() {
    let (zero, one, half) = (R::zero(), R::one(), R::of(0.5));
    let tiny = R::min_positive_value();
    for m in [
        [zero, -zero, R::nan()],
        [R::nan(), zero, -zero],
        [-zero, R::nan(), zero],
    ] {
        assert_eq!(
            guard(m).map(P::bits),
            [tiny; 3].map(P::bits),
            "guard keeps a mass that is not positive in {m:?}"
        );
    }
    assert_eq!(
        guard([half, R::of(0.25), R::of(0.25)]).map(P::bits),
        [half, R::of(0.25), R::of(0.25)].map(P::bits),
        "guard moves a positive mass"
    );
    for m in [
        [zero, zero, one],
        [zero, one, zero],
        [one, zero, zero],
        [zero, half, half],
        [half, zero, half],
    ] {
        for (name, z, bits) in [
            (
                "softmax_tanh",
                SoftmaxTanh::inverse(guard(m)),
                <SoftmaxTanh as SimplexClamp>::clamps(guard(m)),
            ),
            (
                "stick_breaking",
                StickBreaking::inverse(guard(m)),
                <StickBreaking as SimplexClamp>::clamps(guard(m)),
            ),
        ] {
            assert!(
                z.iter().all(|x| x.is_finite()),
                "{name}'s inverse is {z:?} at the corner {m:?}"
            );
            assert_ne!(bits, 0, "{name} does not flag the corner {m:?}");
        }
    }
    let corner = SoftmaxTanh::inverse(guard([zero, zero, one]));
    assert_eq!(
        corner[0], zero,
        "softmax_tanh's z_μ1 at the corner m₂ = 1 is {:e}, not the m₀ = m₁ limit 0",
        corner[0]
    );
}

#[test]
fn encode_clamp_edges() {
    edges_at::<f64>();
    edges_at::<f32>();
    check_ends::<f64>();
    check_ends::<f32>();
    check_softmax_edges::<f64>();
    check_softmax_edges::<f32>();
    check_stick_edges::<f64>();
    check_stick_edges::<f32>();
    check_corners::<f64>();
    check_corners::<f32>();
}

negative_control!(
    encode_clamp_edges,
    "a test that reads the range one step inside its end must fail",
    expected = "the clamp bit is",
    {
        let (lo, _) = check_edges::<SigmoidBeta, f64>("sigmoid_beta");
        assert!(
            clamps::<SigmoidBeta, f64>(lo.up()),
            "the clamp bit is false at sigmoid_beta at v = {:e}",
            lo.up()
        );
    }
);

// ── encode_collinear_beta (REQ-ENC-007; dd_encode §5 test 10, R-438) ────────────────────────────────────────────

/// The collinear central configuration with bodies `order = [left, middle, right]` on the x axis under masses `m`:
/// the middle body's place `t ∈ (0, 1)` between the outer ones at 0 and 1, by bisection on Euler's condition that each
/// body's acceleration is proportional to its distance from the left one, `(a_mid − a_left)/t = a_right − a_left`.
fn euler_positions(m: [f64; 3], order: [usize; 3]) -> [f64; 3] {
    let accel = |x: [f64; 3], i: usize| -> f64 {
        (0..3)
            .filter(|&j| j != i)
            .map(|j| {
                let d = x[j] - x[i];
                m[j] * d.signum() / (d * d)
            })
            .sum()
    };
    let [l, c, r] = order;
    let place = |t: f64| {
        let mut x = [0.0; 3];
        x[c] = t;
        x[r] = 1.0;
        x
    };
    let gap = |t: f64| {
        let x = place(t);
        (accel(x, c) - accel(x, l)) / t - (accel(x, r) - accel(x, l))
    };
    let (mut lo, mut hi) = (1e-9, 1.0 - 1e-9);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if gap(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    place(0.5 * (lo + hi))
}

/// The canonical-frame `(ρ̃, λ̃)` of the collinear positions `x` (on the x axis) under `m`, at `I = 1`, `ρ̃` on `+x`.
fn collinear_jacobi(m: [f64; 3], x: [f64; 3]) -> ([f64; 2], [f64; 2]) {
    let sign = (x[1] - x[0]).signum();
    let r = x.map(|v| [sign * v, 0.0]);
    let j = jacobi(&Ic {
        m,
        r,
        p: [[0.0; 2]; 3],
    });
    let norm = j.rho_t[0].hypot(j.lambda_t[0]);
    ([j.rho_t[0] / norm, 0.0], [j.lambda_t[0] / norm, 0.0])
}

/// The input state: masses `m`, `ρ̃` and `λ̃` reconstructed by the decoder's own map, and the Jacobi momenta `p_ρ`,
/// `p_λ` through its own.
fn state<R: P>(
    m: [f64; 3],
    rho_t: [R; 2],
    lambda_t: [R; 2],
    p_rho: [R; 2],
    p_lambda: [R; 2],
) -> Ic<R> {
    let masses = from_masses(m.map(R::of));
    let (_, _, r) = reconstruct(&masses, rho_t, lambda_t);
    Ic {
        m: masses.m,
        r,
        p: to_particles(&masses, p_rho, p_lambda),
    }
}

const P_RHO: [f64; 2] = [0.3, -0.2];
const P_LAMBDA: [f64; 2] = [-0.4, 0.25];

/// The four `λ̃_y` inputs of test 10.
fn deadband_inputs<R: P>() -> [R; 4] {
    [R::of(-1e-13), -R::zero(), R::zero(), R::of(1e-13)]
}

/// The worst round-trip errors at one landmark, in eps: `λ̃_y` and `λ̃_x` past their bounds, and the rest.
#[derive(Debug, Default)]
struct RoundTrip {
    lambda_y: f64,
    lambda_x: f64,
    rest: f64,
}

/// Rounding headroom of the round trip, in eps.
const ROUND_TRIP_EPS: f64 = 64.0;

/// Test 10 at one landmark off the pole: `ρ̃`, `λ̃_x` of the landmark at `I = 1`, body 2's `side` the sign of `λ̃_x`. The
/// four `λ̃_y` inputs give `β` near π (side `−x`) or near 0 (`+x`), `π̂` or 0 exactly at `±0`, never near `−π`; they
/// encode to the same `z`, with `lookup_clamped`; and `decode(encode(x))` keeps the side, `λ̃_y` and `λ̃_x` within
/// their bounds, the rest within rounding (R-438, "Contradiction flagged, applied").
fn check_landmark<R: P>(
    m: [f64; 3],
    rho_t: [f64; 2],
    lambda_t: [f64; 2],
    side: f64,
    worst: &mut RoundTrip,
) {
    assert!(lambda_t[0] != 0.0, "the landmark sits on the pole");
    let pi = R::LIT_7;
    let mut z_seen: Option<[u64; 8]> = None;
    for y in deadband_inputs::<R>() {
        let lt = [R::of(lambda_t[0]), y];
        let rt = rho_t.map(R::of);
        let x = state(m, rt, lt, P_RHO.map(R::of), P_LAMBDA.map(R::of));
        let e = encode::<Defaults, R>(&x);
        let at = format!("λ̃ = ({:e}, {y:e}), m = {m:?}", lt[0]);
        let b = e.blocks.beta;
        let slope = y.f().abs() / lambda_t[0].abs();
        if side < 0.0 {
            assert!(
                (b.f() - std::f64::consts::PI).abs() <= slope + 4.0 * R::eps(),
                "β = {b:e}, not π, with body 2 on the −x side, at {at}"
            );
            if y == R::zero() {
                assert_eq!(
                    b.bits(),
                    pi.bits(),
                    "β = {b:e}, not π̂ by the select, at {at}"
                );
            }
        } else {
            assert!(
                b.f().abs() <= slope + 4.0 * R::eps() && b >= R::zero(),
                "β = {b:e}, not 0, with body 2 on the +x side, at {at}"
            );
        }
        assert!(
            e.report.lookup_clamped(),
            "lookup_clamped is not set at {at}"
        );
        assert!(!e.report.lookup_mirrored(), "the deadband mirrored at {at}");
        let bits = e.z.z.map(P::bits);
        match z_seen {
            None => z_seen = Some(bits),
            Some(first) => assert_eq!(
                bits, first,
                "the four λ̃_y inputs encode to different z at {at}"
            ),
        }
        // The round trip, in mass-weighted Jacobi coordinates.
        let d = decode::<Defaults, R>(&e.z);
        let norm = R::of(lambda_t[0].abs()).f();
        let eps_z = R::EPS_Z.f();
        let back = d.config.lambda_t;
        assert_eq!(
            back[0].f().signum(),
            side.signum(),
            "decode(encode(x)) put body 2 on the other side at {at}"
        );
        let round = ROUND_TRIP_EPS * R::eps();
        let dy = (back[1].f() - y.f()).abs();
        let bound_y = std::f64::consts::PI * eps_z * norm + y.f().abs();
        worst.lambda_y = worse(worst.lambda_y, (dy - bound_y) / R::eps());
        assert!(
            dy <= bound_y + round,
            "|Δλ̃_y| = {dy:e} over π·ε_z·‖λ̃‖ + |λ̃_y| = {bound_y:e} at {at}"
        );
        let dx = (back[0].f() - lambda_t[0]).abs();
        let bound_x = norm * (1.0 - (std::f64::consts::PI * eps_z).cos());
        worst.lambda_x = worse(worst.lambda_x, (dx - bound_x) / R::eps());
        assert!(
            dx <= bound_x + round,
            "|Δλ̃_x| = {dx:e} over ‖λ̃‖(1 − cos(π·ε_z)) = {bound_x:e} at {at}"
        );
        let rest = [
            (d.ic.m.to_vec(), x.m.to_vec()),
            (d.config.rho_t.to_vec(), rt.to_vec()),
            (d.momentum.p_rho.to_vec(), P_RHO.map(R::of).to_vec()),
            (d.momentum.p_lambda.to_vec(), P_LAMBDA.map(R::of).to_vec()),
        ];
        for (got, want) in rest {
            for (g, w) in got.iter().zip(&want) {
                let e = err(*g, *w, 1.0);
                worst.rest = worse(worst.rest, e);
                assert!(
                    e <= ROUND_TRIP_EPS,
                    "a mass, ρ̃, p_ρ or p_λ is {e:.1} eps off at {at}"
                );
            }
        }
    }
}

/// Test 10's pole landmark on its own: `m₀ = m₁`, body 2 in the middle, at the inner pair's centre of mass, so `λ̃ = 0`
/// exactly. `λ̃_y = ±0` gives `β = 0` by the select; `λ̃_y = ±1e-13` with `λ̃_x = 0` gives `β = π/2` as `atan2`
/// computes it at the precision; each sets `lookup_clamped`, gives no NaN, and the same bits twice.
///
/// The masses are `(¼, ¼, ½)` (applied per R-369, a test decision): every product `C` and the Jacobi map form of a
/// mass and a coordinate is then exact, so `C`'s CoM is 0 and `λ̃_x` stays exactly 0 through the encode. With masses
/// such as `(0.3, 0.3, 0.4)`, `C`'s `fma` CoM leaves a residual of about `1e-17` in `λ̃_x`, and `β = atan2(1e-13, 1e-17)`
/// is `π/2` only to about `1e-4`, `β` being ill-conditioned at the pole: the landmark is no longer on it.
fn check_pole<R: P>() {
    let m = [0.25, 0.25, 0.5];
    for y in deadband_inputs::<R>() {
        let x = state(
            m,
            [R::one(), R::zero()],
            [R::zero(), y],
            P_RHO.map(R::of),
            P_LAMBDA.map(R::of),
        );
        let e = encode::<Defaults, R>(&x);
        let at = format!("the pole landmark, λ̃_y = {y:e}");
        let b = e.blocks.beta;
        let want = if y == R::zero() {
            R::zero()
        } else {
            Float::atan2(R::one(), R::zero())
        };
        assert_eq!(b.bits(), want.bits(), "β = {b:e}, not {want:e}, at {at}");
        assert!(
            e.report.lookup_clamped(),
            "lookup_clamped is not set at {at}"
        );
        assert!(
            e.z.z.iter().all(|v| !v.is_nan()),
            "a NaN in z = {:?} at {at}",
            e.z.z
        );
        assert_eq!(
            out_bits(&encode::<Defaults, R>(&x)),
            out_bits(&e),
            "the pole encodes differently twice at {at}"
        );
    }
    // `β` at `λ̃ = 0` itself, each sign of each zero: 0 by the select (R-437), never `atan2(+0, −0) = π`.
    for lambda_t in [
        [R::zero(), R::zero()],
        [-R::zero(), R::zero()],
        [R::zero(), -R::zero()],
        [-R::zero(), -R::zero()],
    ] {
        let b = beta(lambda_t);
        assert_eq!(
            b.bits(),
            R::zero().bits(),
            "β = {b:e}, not +0, at λ̃ = {lambda_t:?}"
        );
    }
}

/// Test 10 at one precision: masses `(0.2, 0.3, 0.5)`, the three Euler landmarks, body 0, 1 or 2 in the middle; the
/// first puts body 2 on the `−x` side, the second on `+x`; the third, body 2 in the middle, off the pole since
/// `m₀ ≠ m₁`. Then the pole landmark.
fn collinear_at<R: P>() -> RoundTrip {
    let m = [0.2, 0.3, 0.5];
    let mut worst = RoundTrip::default();
    for (order, side) in [([2, 0, 1], -1.0), ([0, 1, 2], 1.0), ([0, 2, 1], 0.0)] {
        let x = euler_positions(m, order);
        let (rho_t, lambda_t) = collinear_jacobi(m, x);
        if side != 0.0 {
            assert_eq!(
                lambda_t[0].signum(),
                side,
                "the landmark {order:?} puts body 2 on the other side"
            );
        }
        assert!(
            lambda_t[0].abs() > 1e-3,
            "the landmark {order:?} is within 1e-3 of the pole"
        );
        check_landmark::<R>(m, rho_t, lambda_t, lambda_t[0].signum(), &mut worst);
    }
    check_pole::<R>();
    worst
}

/// Just off the `−x` half-axis, `λ̃_y = ±1e-13` and `±1e-9`: the true `β` is within `10⁻¹³` of π, which a
/// round-to-nearest f32 `atan2` returns as f32's nearest π, above `π̂`; `β` is then `π̂` by the select, never above it,
/// at either sign of `λ̃_y` (C8/Q5; dd_encode §3.1). At f64, `π̂` is f64's nearest π, which `atan2` never exceeds. Below
/// `π̂`, as at `λ̃_y = 1e-6`, `β` is `atan2`'s own value.
fn check_near_half_axis<R: P>() {
    let pi = R::LIT_7;
    for (x, y) in [
        (-17.157846897416682, 1.110852467838388e-13),
        (-1.0, 1e-13),
        (-0.6, -1e-9),
    ] {
        let lambda_t = [R::of(x), R::of(y)];
        let b = beta(lambda_t);
        let raw = Float::atan2(Float::abs(lambda_t[1]), lambda_t[0]);
        let want = if raw > pi { pi } else { raw };
        assert_eq!(
            b.bits(),
            want.bits(),
            "β = {b:e}, not {want:e}, at λ̃ = {lambda_t:?}"
        );
        assert!(b <= pi, "β = {b:e} above π̂ = {pi:e} at λ̃ = {lambda_t:?}");
    }
    let lambda_t = [R::of(-1.0), R::of(1e-6)];
    let raw = Float::atan2(lambda_t[1], lambda_t[0]);
    assert!(raw < pi, "atan2 at λ̃ = {lambda_t:?} is not below π̂");
    assert_eq!(
        beta(lambda_t).bits(),
        raw.bits(),
        "β is not atan2's value below π̂"
    );
}

#[test]
fn encode_collinear_beta() {
    check_near_half_axis::<f64>();
    check_near_half_axis::<f32>();
    let w64 = collinear_at::<f64>();
    let w32 = collinear_at::<f32>();
    eprintln!("encode_collinear_beta: worst past the bounds (eps): f64 {w64:?}; f32 {w32:?}");
}

negative_control!(
    encode_collinear_beta,
    "a landmark with body 2 on the +x side, checked against the −x side's β = π, must fail",
    expected = "not π, with body 2 on the −x side",
    {
        let m = [0.2, 0.3, 0.5];
        let (rho_t, lambda_t) = collinear_jacobi(m, euler_positions(m, [0, 1, 2]));
        check_landmark::<f64>(m, rho_t, lambda_t, -1.0, &mut RoundTrip::default());
    }
);

// ── encode_t1_gauge (REQ-ENC-003; dd_encode §5 test 2) ──────────────────────────────────────────────────────────

/// `|E(a) − E(b)|` over `z`, each control's difference over `1 + |z|`, in eps.
fn z_gap<R: P>(a: &Encoded<R>, b: &Encoded<R>) -> f64 {
    a.z.z
        .iter()
        .zip(&b.z.z)
        .map(|(x, y)| err(*x, *y, 1.0 + y.f().abs()))
        .fold(0.0, worse)
}

/// The T1 limit, in eps of the precision, over `1 + |z|`: `z`'s error after `C` on a moved and rescaled state,
/// conditioned by the interior's `d logit/ds` and `d artanh/dt`.
const T1_EPS: f64 = 128.0;

/// A random interior state: the default decode of a random `z`, `|z| ≤ 4`, at f64. Its `λ̃_y = sin α·sin β` is at
/// least about `1.5e-3`, far outside the deadband at either precision.
fn interior_state(rng: &mut SplitMix) -> Ic<f64> {
    decode::<Defaults, f64>(&Latent::new(random_z(rng, SPAN))).ic
}

/// `g`'s conditioning of `C`'s arithmetic on a state of unit size: `1 + ‖a‖/s + ‖u‖·√s`. Removing a translation `a`
/// from positions of size `s`, or a boost `u` from momenta of size `s^{−1/2}`, leaves an absolute rounding of the
/// larger, so a relative one of that ratio.
fn kappa(g: &Gauge) -> f64 {
    1.0 + g.a[0].hypot(g.a[1]) / g.s + g.u[0].hypot(g.u[1]) * g.s.sqrt()
}

/// T1 on one state and one `g`: `E(g·x)` and `E(x)` agree to [`T1_EPS`] times `g`'s [`kappa`]; `E(x)` does not
/// mirror and `E(g·x)` mirrors iff `g` does (both off the deadband); `λ` scales as `1/s`. Returns the `z` gap over
/// `κ`, in eps.
fn check_t1<R: P>(x: &Ic<f64>, g: &Gauge) -> f64 {
    let ex = encode::<Defaults, R>(&ic_of(x));
    let egx = encode::<Defaults, R>(&ic_of(&g.act(x)));
    let k = kappa(g);
    let gap = z_gap(&egx, &ex) / k;
    let at = format!("g = {g:?}, x = {x:?}");
    assert!(
        gap <= T1_EPS,
        "E(g·x) differs from E(x) by {gap:.1} eps (over κ = {k:.1}) at {at}"
    );
    assert!(!ex.report.mirrored, "E(x) mirrored a canonical x at {at}");
    assert_eq!(
        egx.report.mirrored, g.mirror,
        "E(g·x)'s mirror is not g's at {at}"
    );
    let scaled = egx.report.lambda.f() * g.s / ex.report.lambda.f();
    assert!(
        (scaled - 1.0).abs() <= 64.0 * R::eps() * k,
        "λ·s is {scaled}, not E(x)'s λ, at {at}"
    );
    gap
}

fn t1_generic<R: P>(seed: u64) -> f64 {
    let mut rng = SplitMix::new(seed);
    let mut worst = 0.0f64;
    for _ in 0..2000 {
        let x = interior_state(&mut rng);
        for _ in 0..5 {
            let g = Gauge::random(&mut rng, 10.0, 8.0);
            worst = worse(worst, check_t1::<R>(&x, &g));
        }
    }
    worst
}

/// A near-collinear state in the deadband: `ρ̃ = (cos α, 0)`, `λ̃ = (±sin α, y)`, `|y| ≤ 0.9·δ_λ`, interior momenta.
fn deadband_state(rng: &mut SplitMix) -> Ic<f64> {
    let a = uniform(rng, 0.1, 1.4);
    let side = if rng.next_u64() & 1 == 1 { 1.0 } else { -1.0 };
    let y = uniform(rng, -0.9, 0.9) * f64::DELTA_LAMBDA;
    let p = |rng: &mut SplitMix| [uniform(rng, -1.5, 1.5), uniform(rng, -1.5, 1.5)];
    let m = [uniform(rng, 0.1, 0.5), uniform(rng, 0.1, 0.5), 0.0];
    let m = [m[0], m[1], 1.0 - m[0] - m[1]];
    let (p_rho, p_lambda) = (p(rng), p(rng));
    state(m, [a.cos(), 0.0], [side * a.sin(), y], p_rho, p_lambda)
}

/// T1 across the deadband (f64): `g` turns, moves, boosts and rescales, never mirrors, so `C` turns the state back
/// with `λ̃_y` still inside `±δ_λ` to rounding: no mirror either way, and `E(g·x) = E(x)`. Each is encoded twice, with
/// the same bits. Returns the `z` gap, in eps.
fn t1_deadband(seed: u64) -> f64 {
    let mut rng = SplitMix::new(seed);
    let mut worst = 0.0f64;
    for _ in 0..2000 {
        let x = deadband_state(&mut rng);
        for _ in 0..5 {
            let g = Gauge {
                mirror: false,
                ..Gauge::random(&mut rng, 1.0, 2.0)
            };
            let gx = g.act(&x);
            let (ex, egx) = (encode::<Defaults, f64>(&x), encode::<Defaults, f64>(&gx));
            let at = format!("g = {g:?}, x = {x:?}");
            assert!(
                !ex.report.mirrored && !egx.report.mirrored,
                "a deadband state mirrored at {at}"
            );
            assert_eq!(
                out_bits(&encode::<Defaults, f64>(&gx)),
                out_bits(&egx),
                "the tie encodes differently twice at {at}"
            );
            let gap = z_gap(&egx, &ex) / kappa(&g);
            assert!(
                gap <= T1_EPS,
                "E(g·x) differs from E(x) by {gap:.1} eps across the deadband at {at}"
            );
            worst = worse(worst, gap);
        }
    }
    worst
}

/// The exact tie, at each precision: masses `(¼, ¼, ½)`, `r₀ = r₁ = (−1, a)`, `r₂ = (1, −a)`, at rest, so that `λ̃_y`
/// is `−a` exactly (`kernel::canon`'s tie): at `a = δ_λ`, `λ̃_y = −δ_λ`, no mirror, the same bits twice; one step past,
/// a mirror.
fn check_tie<R: P>() {
    let tie = |a: R| -> Ic<R> {
        let (q, h, one) = (R::of(0.25), R::of(0.5), R::one());
        Ic {
            m: [q, q, h],
            r: [[-one, a], [-one, a], [one, -a]],
            p: [[R::zero(); 2]; 3],
        }
    };
    let d = R::DELTA_LAMBDA;
    for (a, mirror) in [(d, false), (-d, false), (d.up(), true)] {
        let first = encode::<Defaults, R>(&tie(a));
        let again = encode::<Defaults, R>(&tie(a));
        assert_eq!(
            first.report.mirrored, mirror,
            "the tie at λ̃_y = {:e} mirrors: {}",
            -a, first.report.mirrored
        );
        assert_eq!(
            out_bits(&first),
            out_bits(&again),
            "the tie at λ̃_y = {:e} encodes differently twice",
            -a
        );
    }
}

#[test]
fn encode_t1_gauge() {
    let generic64 = t1_generic::<f64>(11);
    let generic32 = t1_generic::<f32>(12);
    let deadband = t1_deadband(13);
    check_tie::<f64>();
    check_tie::<f32>();
    eprintln!(
        "encode_t1_gauge: worst z gap (eps): generic f64 {generic64:.1}, f32 {generic32:.1}; deadband f64 {deadband:.1}"
    );
}

negative_control!(
    encode_t1_gauge,
    "a g that mirrors the positions but not the momenta must break T1",
    expected = "differs from E(x)",
    {
        let mut rng = SplitMix::new(22);
        let x = interior_state(&mut rng);
        let g = Gauge {
            mirror: true,
            ..Gauge::identity()
        };
        let mut gx = g.act(&x);
        gx.p = x.p;
        let (ex, egx) = (encode::<Defaults, f64>(&x), encode::<Defaults, f64>(&gx));
        let gap = z_gap(&egx, &ex);
        assert!(gap <= T1_EPS, "E(g·x) differs from E(x) by {gap:.1} eps");
    }
);

// ── encode_notices (REQ-ENC-014) ────────────────────────────────────────────────────────────────────────────────

/// The notices `e` raised: `(lookup_clamped, lookup_mirrored)`, and `λ`.
fn raised<R: P>(e: &Encoded<R>) -> (bool, bool, f64) {
    (
        e.report.lookup_clamped(),
        e.report.lookup_mirrored(),
        e.report.lambda.f(),
    )
}

/// One forced operation's notices: `clamped` and `mirrored` as given, and `λ` within 16 eps of `lambda`.
fn check_notice<R: P>(name: &str, x: &Ic<R>, clamped: bool, mirrored: bool, lambda: f64) {
    let (c, m, l) = raised(&encode::<Defaults, R>(x));
    assert_eq!(
        (c, m),
        (clamped, mirrored),
        "{name}: (lookup_clamped, lookup_mirrored) = {:?}, not {:?}",
        (c, m),
        (clamped, mirrored)
    );
    assert!(
        (l - lambda).abs() <= 16.0 * R::eps() * lambda,
        "{name}: λ = {l}, not {lambda}"
    );
}

fn notices_at<R: P>(seed: u64) {
    let mut rng = SplitMix::new(seed);
    // Canonical inputs, the decode's own output: no notice, λ = 1 to rounding.
    for _ in 0..2000 {
        let z: [R; 8] = random_z(&mut rng, SPAN);
        let x = decode::<Defaults, R>(&Latent::new(z)).ic;
        check_notice("a canonical input", &x, false, false, 1.0);
    }
    let x = interior_state(&mut rng);
    let moved = Gauge {
        mirror: false,
        s: 1.0,
        ..Gauge::random(&mut rng, 10.0, 0.0)
    };
    check_notice(
        "translated, boosted and turned",
        &ic_of::<R>(&moved.act(&x)),
        false,
        false,
        1.0,
    );
    let scaled = Gauge {
        s: 4.0,
        ..Gauge::identity()
    };
    check_notice(
        "rescaled by 4",
        &ic_of::<R>(&scaled.act(&x)),
        false,
        false,
        0.25,
    );
    let mirrored = Gauge {
        mirror: true,
        ..Gauge::identity()
    };
    check_notice("mirrored", &ic_of::<R>(&mirrored.act(&x)), false, true, 1.0);
    let z = [R::zero(); 8];
    check_notice(
        "|q| > q_max",
        &with_p_rho(z, [R::of(2.5), R::zero()]),
        true,
        false,
        1.0,
    );
    // The coincident inner pair at `I = 1` exactly: `r₀ = r₁ = (−1, 0)`, `r₂ = (1, 0)`, masses `(¼, ¼, ½)`.
    let (q, h, one, zero) = (R::of(0.25), R::of(0.5), R::one(), R::zero());
    let coincident = Ic {
        m: [q, q, h],
        r: [[-one, zero], [-one, zero], [one, zero]],
        p: [
            [R::of(0.1), R::of(-0.2)],
            [R::of(0.3), zero],
            [R::of(-0.2), R::of(0.1)],
        ],
    };
    check_notice("the coincident inner pair", &coincident, true, false, 1.0);
    let e = encode::<Defaults, R>(&coincident);
    assert!(
        e.report.coincident,
        "the coincident inner pair is not reported as coincident (R-13)"
    );
}

/// [`Report`]'s two notices from its fields: `lookup_clamped` iff the coincident pair or any control's bit.
fn check_report() {
    let report = |coincident, clamped_controls, mirrored| Report {
        lambda: 1.0f64,
        coincident,
        clamped_controls,
        mirrored,
    };
    for (r, clamped, mirrored) in [
        (report(false, 0, false), false, false),
        (report(true, 0, false), true, false),
        (report(false, 1 << 7, false), true, false),
        (report(true, 1, false), true, false),
        (report(false, 0, true), false, true),
    ] {
        assert_eq!(
            (r.lookup_clamped(), r.lookup_mirrored()),
            (clamped, mirrored),
            "the report {r:?} raises the wrong notices"
        );
    }
}

#[test]
fn encode_notices() {
    notices_at::<f64>(31);
    notices_at::<f32>(32);
    check_report();
}

negative_control!(
    encode_notices,
    "a canonical input read with a raised mirror must fail",
    expected = "(lookup_clamped, lookup_mirrored)",
    {
        let mut rng = SplitMix::new(33);
        let x = interior_state(&mut rng);
        check_notice("a canonical input", &x, false, true, 1.0);
    }
);

// ── stage_encode_native ─────────────────────────────────────────────────────────────────────────────────────────

/// The words of an [`EncodeOut`], for a bitwise comparison.
fn out_words(o: &EncodeOut) -> Vec<u32> {
    o.z.iter()
        .chain(&[o.alpha, o.beta, o.report.lambda])
        .map(|x| x.to_bits())
        .chain([
            u32::from(o.report.coincident),
            o.report.clamped_controls,
            u32::from(o.report.mirrored),
        ])
        .collect()
}

fn check_stage(states: &[Ic<f32>]) {
    let mut words = vec![0; states.len() * ic_words()];
    for (i, x) in states.iter().enumerate() {
        write_ic(&mut words, i, x);
    }
    let sentinel = 0xDEAD_BEEF;
    let mut out = vec![sentinel; (states.len() + 1) * encode_words()];
    for i in 0..=states.len() {
        encode_entry(UVec3::new(i as u32, 0, 0), &words, &mut out);
    }
    for (i, x) in states.iter().enumerate() {
        let want = EncodeOut::of(&encode::<Defaults, f32>(x));
        assert_eq!(
            out_words(&read_encoded(&out, i)),
            out_words(&want),
            "the entry point's sample {i} is not its encode"
        );
    }
    assert!(
        out[states.len() * encode_words()..]
            .iter()
            .all(|w| *w == sentinel),
        "the entry point wrote past the last sample"
    );
}

#[test]
fn stage_encode_native() {
    let mut rng = SplitMix::new(41);
    let mut states: Vec<Ic<f32>> = (0..32).map(|_| ic_of(&interior_state(&mut rng))).collect();
    let mirrored = Gauge {
        mirror: true,
        ..Gauge::identity()
    };
    states.push(ic_of(&mirrored.act(&interior_state(&mut rng))));
    states.push(with_p_rho([0.0f32; 8], [2.5, -2.5]));
    let (q, h) = (0.25f32, 0.5f32);
    states.push(Ic {
        m: [q, q, h],
        r: [[-1.0, 0.0], [-1.0, 0.0], [1.0, 0.0]],
        p: [[0.0; 2]; 3],
    });
    check_stage(&states);
    // The notices word's bits, and its round trip through the words.
    for (coincident, mirrored, controls) in [
        (true, false, 0),
        (false, true, 0),
        (false, false, 0xA5),
        (true, true, 0xFF),
    ] {
        let report = Report {
            lambda: 0.5f32,
            coincident,
            clamped_controls: controls,
            mirrored,
        };
        let want = u32::from(coincident) | (u32::from(mirrored) << 1) | (controls << 8);
        assert_eq!(notices(&report), want, "the notices word of {report:?}");
        let e = Encoded {
            report,
            ..encode::<Defaults, f32>(&states[0])
        };
        let mut out = vec![0; encode_words()];
        write_encoded(&mut out, 0, &e);
        assert_eq!(
            read_encoded(&out, 0).report,
            report,
            "the report's words do not round-trip"
        );
    }
    // `β`'s entry point on its operands' bits, `±0` included.
    let pairs: [[f32; 2]; 8] = [
        [-0.5, 0.0],
        [-0.5, -0.0],
        [0.0, 0.0],
        [-0.0, -0.0],
        [0.5, 0.0],
        [0.0, 1e-13],
        [-0.3, -0.2],
        [0.3, 0.4],
    ];
    let words: Vec<u32> = pairs.as_flattened().iter().map(|x| x.to_bits()).collect();
    let mut out = vec![0xDEAD_BEEFu32; pairs.len() + 1];
    for i in 0..=pairs.len() {
        beta_entry(UVec3::new(i as u32, 0, 0), &words, &mut out);
    }
    for (i, pair) in pairs.iter().enumerate() {
        assert_eq!(out[i], beta(*pair).to_bits(), "β's entry point at {pair:?}");
    }
    assert_eq!(
        out[pairs.len()],
        0xDEAD_BEEF,
        "β's entry point wrote past the last pair"
    );
}

negative_control!(
    stage_encode_native,
    "a sample read back from the next sample's words must fail",
    expected = "is not its encode",
    {
        let mut rng = SplitMix::new(42);
        let states: Vec<Ic<f32>> = (0..2).map(|_| ic_of(&interior_state(&mut rng))).collect();
        let mut words = vec![0; states.len() * ic_words()];
        for (i, x) in states.iter().enumerate() {
            write_ic(&mut words, i, x);
        }
        let mut out = vec![0; states.len() * encode_words()];
        for i in 0..states.len() {
            encode_entry(UVec3::new(i as u32, 0, 0), &words, &mut out);
        }
        let want = EncodeOut::of(&encode::<Defaults, f32>(&states[0]));
        assert_eq!(
            out_words(&read_encoded(&out, 1)),
            out_words(&want),
            "the entry point's sample 0 is not its encode"
        );
    }
);
