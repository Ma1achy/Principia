//! The decoder's mirror step removed, latent `β` over the full circle (R-865, R-898, R-1020; REQ-DEC-055; dd_decoder
//! §3.2 and §3.3, dd_encode §3.2), under the offset link `β = z_β + π/2`:
//! - `mirror_step_removed_lower_half`: a lower-half state, `β ∈ (π, 2π)`, moved by a random gauge without the mirror,
//!   encodes to `β ∈ (π, 2π)`, `z_β ∈ (π/2, 3π/2)`, and decodes back to itself within REQ-ENC-024's `ε_phys` in `‖·‖_phys`, `L_z`'s sign
//!   kept, not to its mirror image. Control: the mirror step left in fails.
//! - `mirror_step_removed_syzygy_trace`: a latent-chart trace through a syzygy at `β = 0` (`z_β = −π/2`) and at
//!   `β = π` (`z_β = π/2`), the samples'
//!   encoded `z_β` each placed on the lift nearest the previous sample's (`kernel::encode::by_kind::nearest_lift`), the
//!   first about the chart's centre, is continuous: every step within twice the path's own and the trace within
//!   `ε_phys` of the path. Control: a σ-type link onto the circle, which jumps at `β = 0`, fails.
//!
//! **Domain** (pitfalls §10): encode and decode on states held fixed, a path of them for the trace; no integration.

#[cfg(feature = "controls")]
use kernel::canon::reflect;
use kernel::decode::{decode, Ic, Latent};
use kernel::encode::by_kind::nearest_lift;
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use validation::decode_f32_eps::SplitMix;
use validation::gauge::{uniform, Gauge};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};

const PI: f64 = std::f64::consts::PI;

/// The offset link's `π/2` at f64 (R-1020).
const C: f64 = std::f64::consts::FRAC_PI_2;

/// An encode under test, from a state to its `z`: the kernel's, or a control's.
type Enc = fn(&Ic<f64>) -> [f64; 8];

fn kernel_encode(x: &Ic<f64>) -> [f64; 8] {
    encode::<Defaults, f64>(x).z.z
}

/// `L_z = Σ rᵢ × pᵢ`.
fn lz(x: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| x.r[i][0] * x.p[i][1] - x.r[i][1] * x.p[i][0])
        .sum()
}

/// A random interior `z` with `β = z_β + π/2 ∈ (π, 2π)`, away from the ends.
fn lower_z(rng: &mut SplitMix) -> [f64; 8] {
    let mut z: [f64; 8] = core::array::from_fn(|_| uniform(rng, -1.5, 1.5));
    z[1] = uniform(rng, PI + 0.05, 2.0 * PI - 0.05) - C;
    z
}

/// The lower-half check on `enc` (the module's note): returns the worst residual in `ε_phys`.
fn check_lower_half(enc: Enc) -> f64 {
    let mut rng = SplitMix::new(0x0865);
    let mut worst = 0.0f64;
    for n in 0..500 {
        let x = decode::<Defaults, f64>(&Latent::new(lower_z(&mut rng))).ic;
        let g = Gauge {
            mirror: false,
            ..Gauge::random(&mut rng, 4.0, 2.0)
        };
        let z = enc(&g.act(&x));
        let beta = z[1] + C;
        assert!(
            beta > PI && beta < 2.0 * PI,
            "state {n}: a lower-half state encodes to β = {beta} (z_β = {}), not in (π, 2π)",
            z[1]
        );
        let back = decode::<Defaults, f64>(&Latent::new(z)).ic;
        let d = phys(&x, &back) / eps_phys::<f64>();
        assert!(
            d <= 1.0 && lz(&back).signum() == lz(&x).signum(),
            "state {n}: a lower-half state decodes back {d:.2} ε_phys from itself, L_z {} against {}",
            lz(&back),
            lz(&x)
        );
        worst = worst.max(d);
    }
    worst
}

#[test]
fn mirror_step_removed_lower_half() {
    let worst = check_lower_half(kernel_encode);
    eprintln!(
        "mirror_step_removed_lower_half: 500 lower-half states; worst residual {worst:.3} ε_phys"
    );
}

negative_control!(
    mirror_step_removed_lower_half,
    "an encode with the mirror step left in sends a lower-half state to its mirror image's z and must fail",
    expected = "not in (π, 2π)",
    {
        check_lower_half(|x| {
            let c = kernel::canon::canonicalise(x);
            let folded = reflect(&c.ic, c.lambda_t_y < 0.0);
            encode::<Defaults, f64>(&folded).z.z
        });
    }
);

/// The path's step in `β`, and its half-length either side of the syzygy.
const STEP: f64 = 1.0 / 256.0;
const HALF: f64 = 0.25;

/// The trace check on `enc` at the syzygy `centre`, `z_β = −π/2` or `π/2`, `β = 0` or `π` (the module's note): the path
/// `z(t)`, `z_β = centre + t`
/// for `t` in `[−HALF, HALF]` by [`STEP`], the other controls fixed; each state turned and moved by its own gauge
/// without the mirror; each sample's `z_β` placed on the lift nearest the previous one's, the first about `centre`.
/// Returns the largest step of the trace.
fn check_trace(enc: Enc, centre: f64) -> f64 {
    let mut rng = SplitMix::new(0x0898);
    let base: [f64; 8] = [0.4, 0.0, -0.2, 0.3, 0.1, -0.25, 0.2, -0.3];
    let n = (2.0 * HALF / STEP) as usize;
    let mut prev = centre;
    // Each sample: `t`, the path's `z_β`, and the trace's placed `z_β`.
    let mut trace = Vec::with_capacity(n + 1);
    for k in 0..=n {
        let t = -HALF + STEP * k as f64;
        let mut z = base;
        z[1] = centre + t;
        let x = decode::<Defaults, f64>(&Latent::new(z)).ic;
        let g = Gauge {
            mirror: false,
            ..Gauge::random(&mut rng, 4.0, 2.0)
        };
        let placed = nearest_lift(enc(&g.act(&x))[1], prev);
        trace.push((t, z[1], placed));
        prev = placed;
    }
    // Continuity first, over the whole trace; then the trace against the path.
    let mut largest = 0.0f64;
    for w in trace.windows(2) {
        let ((_, _, from), (t, _, to)) = (w[0], w[1]);
        let jump = (to - from).abs();
        assert!(
            jump <= 2.0 * STEP,
            "the trace through z_β = {centre} jumps by {jump} at t = {t}, from {from} to {to}"
        );
        largest = largest.max(jump);
    }
    for &(t, path, placed) in &trace {
        assert!(
            (placed - path).abs() <= 1e-9,
            "the trace through z_β = {centre} is at {placed}, not the path's {path}, at t = {t}"
        );
    }
    largest
}

#[test]
fn mirror_step_removed_syzygy_trace() {
    let (at_zero, at_pi) = (
        check_trace(kernel_encode, -C),
        check_trace(kernel_encode, C),
    );
    // The lift itself: ties to the smaller k, and every reference within a period.
    let tau = 2.0 * PI;
    assert_eq!(
        nearest_lift(0.0, PI),
        0.0,
        "a tie at +π takes the smaller k"
    );
    assert_eq!(
        nearest_lift(PI, 0.0),
        -PI,
        "a tie at −π takes the smaller k"
    );
    assert_eq!(nearest_lift(0.1, 7.0), 0.1 + tau);
    assert_eq!(nearest_lift(6.2, -0.1), 6.2 - tau);
    assert_eq!(nearest_lift(3.0, 3.0), 3.0);
    eprintln!(
        "mirror_step_removed_syzygy_trace: {} samples per syzygy; largest step {at_zero:e} at β = 0, {at_pi:e} at \
         β = π (the path's {STEP:e})",
        (2.0 * HALF / STEP) as usize + 1
    );
}

negative_control!(
    mirror_step_removed_syzygy_trace,
    "a σ-type link onto the circle, z = logit(β/2π), jumps from +∞ to −∞ at β = 0 and must fail",
    expected = "jumps by",
    {
        check_trace(
            |x| {
                let e = encode::<Defaults, f64>(x);
                let mut z = e.z.z;
                let s = e.blocks.beta / (2.0 * PI);
                z[1] = (s / (1.0 - s)).ln();
                z
            },
            -C,
        );
    }
);

/// The encode's representative (inverse_encode_contract Part 1's T2; dd_decoder § 3.2; R-1020): over states on the
/// whole circle, each in a random gauge without the mirror, and the two half-axes and the pole, `enc`'s `z_β` lies in
/// `[−c, 3c]`, `c = π̂/2`, the offset link's inverse of the signed `β ∈ [0, 2π̂]`; the `+x` half-axis and the pole give
/// `−c`, the `−x` half-axis `c`, exactly. Returns the extremes met.
fn check_representative(enc: Enc) -> (f64, f64) {
    let c = C;
    let mut rng = SplitMix::new(0x1020);
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for n in 0..2000 {
        let mut z: [f64; 8] = core::array::from_fn(|_| uniform(&mut rng, -1.5, 1.5));
        z[1] = uniform(&mut rng, -c, 3.0 * c);
        let x = decode::<Defaults, f64>(&Latent::new(z)).ic;
        let g = Gauge {
            mirror: false,
            ..Gauge::random(&mut rng, 4.0, 2.0)
        };
        let zb = enc(&g.act(&x))[1];
        assert!(
            (-c..=3.0 * c).contains(&zb),
            "state {n}: z_β = {zb} is outside the representative [−π/2, 3π/2]"
        );
        (lo, hi) = (lo.min(zb), hi.max(zb));
    }
    // The landmarks, exact: `z_β = ∓c` decode to the half-axes, `α` near 0 the pole.
    for (z_beta, want) in [(-c, -c), (c, c)] {
        let x =
            decode::<Defaults, f64>(&Latent::new([0.4, z_beta, 0.0, 0.0, 0.0, 0.0, 0.2, -0.3])).ic;
        assert_eq!(
            enc(&x)[1],
            want,
            "the half-axis at z_β = {z_beta} encodes to {}",
            enc(&x)[1]
        );
    }
    let pole = decode::<Defaults, f64>(&Latent::new([-1e3, 1.0, 0.0, 0.0, 0.0, 0.0, 0.2, -0.3])).ic;
    assert_eq!(
        enc(&pole)[1],
        -c,
        "the pole encodes to z_β = {}",
        enc(&pole)[1]
    );
    (lo, hi)
}

#[test]
fn mirror_step_removed_representative() {
    let (lo, hi) = check_representative(kernel_encode);
    eprintln!("mirror_step_removed_representative: z_β over 2000 states in [{lo}, {hi}]");
}

negative_control!(
    mirror_step_removed_representative,
    "an encode returning the identity's representative, z_β = β ∈ [0, 2π̂], leaves [−π/2, 3π/2] and must fail",
    expected = "is outside the representative",
    {
        check_representative(|x| {
            let e = encode::<Defaults, f64>(x);
            let mut z = e.z.z;
            z[1] = e.blocks.beta;
            z
        });
    }
);
