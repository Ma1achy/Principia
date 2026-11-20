//! The descriptor's `rho_angle` read from the configuration block's own `(ρ̃, λ̃)`, `Φ`'s vectors (R-628; dd_decoder
//! §3.6; `kernel::decode::descriptor::config_angle`), on the CPU at f32 and f64:
//! - `rho_angle_latent_is_beta`: on the latent chart's decode, whose `ρ̃ = cos α·(1, 0)` and `λ̃ = sin α·(cos β, sin β)`
//!   (dd_decoder §3.2), `rho_angle` is the configuration block's `β` within a few ε of it, over random `z`, and is
//!   `config_angle` of the decode's configuration bit for bit; `η_E` (`energy_norm::apply_shared`, and `apply` through
//!   the free blocks) keeps it, bit for bit, since it moves only the momenta.
//! - `rho_angle_shape_pole`: on the shape sphere's exponential map at `(½, ½)`, where the point is `n₀`, at the exact
//!   `λ̃ = 0` pole `n₀ = (+1, ±0, ±0)` `rho_angle` is 0's bits at f32 and f64, at the phases `φ_f = 0`, `0.3` and `2`;
//!   near it, `n₀ = (+1, ε, 0)` and `(+1, 0, ε)` with `|ε|` from `10⁻¹²` to `10⁻⁴`, both signs, the f32 decode's
//!   `rho_angle` is within `4ε₃₂` of `Φ`'s f64 angle (physics measured at most `1.6·10⁻⁷` rad).
//!
//! Control: `rho_angle` read from the f32 decode's re-centred positions (`descriptor::describe`, the form before
//! R-628), held to `Φ`'s f64 angle by the same check, is far from it near the pole: `C`'s rounding residue in `λ`
//! swamps `λ̃` there.

use kernel::chart::{Chart, ShapeMap, ShapeSphere};
use kernel::decode::descriptor::{angle, config_angle};
use kernel::decode::energy_norm::{apply, apply_shared};
use kernel::decode::{decode, Decoded, Free, Latent};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use validation::negative_control;

/// The widths checked.
trait Width: LinkReal + PayloadReal + Copy + Into<f64> {
    const EPS: f64;
    const LABEL: &'static str;
    fn of(x: f64) -> Self;
    fn bits(self) -> u64;
}

impl Width for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const LABEL: &'static str = "f32";
    fn of(x: f64) -> Self {
        x as f32
    }
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
}

impl Width for f64 {
    const EPS: f64 = f64::EPSILON;
    const LABEL: &'static str = "f64";
    fn of(x: f64) -> Self {
        x
    }
    fn bits(self) -> u64 {
        self.to_bits()
    }
}

/// A SplitMix64 stream, for the draws.
struct Draws(u64);

impl Draws {
    /// A value on `[lo, hi)`.
    fn on(&mut self, lo: f64, hi: f64) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        lo + (hi - lo) * (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The latent decode's `rho_angle` against its `β`, and through `η_E`, at the width `R`; `rewrite` turns each `η_E`
/// result into the decode checked (the identity, or the control's).
fn check_latent<R: Width>(rewrite: fn(Decoded<R>) -> Decoded<R>) {
    let mut draws = Draws(0x0628);
    for _ in 0..512 {
        let z: [R; 8] = core::array::from_fn(|_| R::of(draws.on(-4.0, 4.0)));
        let d = decode::<Defaults, R>(&Latent::new(z));
        let (got, beta) = (d.descriptor.rho_angle, d.config.beta);
        let at = format!("z = {:?} at {}", z.map(Into::into), R::LABEL);
        assert_eq!(
            got.bits(),
            config_angle(&d.config).bits(),
            "rho_angle is not the configuration's at {at}"
        );
        let (g, b): (f64, f64) = (got.into(), beta.into());
        assert!(
            (g - b).abs() <= 16.0 * R::EPS * b.abs().max(1.0),
            "rho_angle {g:e} is not β = {b:e} at {at}"
        );
        let e = R::of(Into::<f64>::into(d.descriptor.V_0) + 0.5);
        for (name, (n, _)) in [
            ("apply_shared", apply_shared(d, Some(e), false)),
            (
                "apply",
                apply(&mut Free::<Defaults>::new(), d, Some(e), false),
            ),
        ] {
            let n = rewrite(n);
            assert_eq!(
                n.descriptor.rho_angle.bits(),
                got.bits(),
                "{name}'s η_E changed rho_angle at {at}"
            );
        }
    }
}

#[test]
fn rho_angle_latent_is_beta() {
    check_latent::<f32>(|n| n);
    check_latent::<f64>(|n| n);
}

/// `n` with its descriptor rewritten by `describe` from its positions, as `η_E` rewrote it before R-628.
#[cfg(feature = "controls")]
fn positional<R: Width>(n: Decoded<R>) -> Decoded<R> {
    Decoded {
        descriptor: kernel::decode::descriptor::describe(&n.ic),
        ..n
    }
}

negative_control!(
    rho_angle_latent_is_beta,
    "an η_E that rewrites rho_angle from the positions, not the configuration block, must be found",
    expected = "η_E changed rho_angle",
    {
        check_latent::<f32>(positional::<f32>);
        check_latent::<f64>(positional::<f64>);
    }
);

/// The shape sphere at `n₀`, phase `phase`, its masses unequal so that `C`'s residue is not 0.
fn sphere(n0: [f32; 3], phase: f32) -> ShapeSphere<f32> {
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0,
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: 1.0,
        inertia: 1.0,
        phase,
        z_mass: [0.7, -0.4],
        z_mom: [0.1, -0.2, 0.3, 0.05],
    }
}

/// `Φ`'s angle at f64: the f64 decode's configuration block, [`angle`] of its `ρ̃ × λ̃` and `ρ̃ · λ̃`, formed here.
fn phi_angle(c: &ShapeSphere<f32>) -> f64 {
    let k = c.decode::<Defaults, f64>(0.5, 0.5).config;
    let (r, l) = (k.rho_t, k.lambda_t);
    angle(r[0] * l[1] - r[1] * l[0], r[0] * l[0] + r[1] * l[1])
}

/// The phases.
const PHASES: [f32; 3] = [0.0, 0.3, 2.0];

/// The near-pole points: `(+1, ε, 0)` and `(+1, 0, ε)`, both signs, `|ε|` from `10⁻¹²` to `10⁻⁴`, at each phase.
fn near_pole() -> Vec<ShapeSphere<f32>> {
    let mut out = Vec::new();
    for phase in PHASES {
        for k in 0..=8 {
            let e = 10f32.powi(-12 + k);
            for eps in [e, -e] {
                out.push(sphere([1.0, eps, 0.0], phase));
                out.push(sphere([1.0, 0.0, eps], phase));
            }
        }
    }
    out
}

/// `got`, an f32 `rho_angle` at `c`, against `Φ`'s f64 angle within `4ε₃₂`, a whole turn apart counted as none.
fn check_phi(c: &ShapeSphere<f32>, who: &str, got: f32) {
    let truth = phi_angle(c);
    let tau = std::f64::consts::TAU;
    let d = (f64::from(got) - truth).rem_euclid(tau);
    let d = d.min(tau - d);
    assert!(
        d <= 4.0 * f64::from(f32::EPSILON) * truth.abs().max(1.0),
        "the {who} rho_angle {got:e} at n₀ = {:?}, φ_f = {} is not Φ's f64 angle {truth:e}",
        c.n0,
        c.phase
    );
}

#[test]
fn rho_angle_shape_pole() {
    for phase in PHASES {
        for zeros in [[0.0f32, 0.0], [0.0, -0.0], [-0.0, 0.0], [-0.0, -0.0]] {
            let c = sphere([1.0, zeros[0], zeros[1]], phase);
            let (d32, d64) = (
                c.decode::<Defaults, f32>(0.5, 0.5),
                c.decode::<Defaults, f64>(0.5, 0.5),
            );
            assert_eq!(
                [d32.config.lambda_t, d64.config.lambda_t.map(|x| x as f32)]
                    .map(|v| v.map(f32::abs)),
                [[0.0; 2]; 2],
                "Φ's λ̃ at n₀ = {:?} is not 0",
                c.n0
            );
            assert_eq!(
                (
                    d32.descriptor.rho_angle.to_bits(),
                    d64.descriptor.rho_angle.to_bits()
                ),
                (0, 0),
                "rho_angle at the λ̃ = 0 pole n₀ = {:?}, φ_f = {phase} is not 0's bits",
                c.n0
            );
        }
    }
    for c in near_pole() {
        check_phi(
            &c,
            "f32 decode's",
            c.decode::<Defaults, f32>(0.5, 0.5).descriptor.rho_angle,
        );
    }
}

negative_control!(
    rho_angle_shape_pole,
    "rho_angle read from the re-centred positions must be found far from Φ's near the λ̃ = 0 pole",
    expected = "is not Φ's f64 angle",
    for c in near_pole() {
        check_phi(
            &c,
            "positional",
            kernel::decode::descriptor::describe(&c.decode::<Defaults, f32>(0.5, 0.5).ic).rho_angle,
        );
    }
);
