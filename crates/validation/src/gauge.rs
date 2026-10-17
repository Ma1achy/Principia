//! The gauge group's action, as a test utility (dd_encode §3.3, "The gauge group action, explicit (this is what T1
//! sweeps)"; inverse-encode contract Part 1): for `g = (a, u, θ, s, Π)`, a translation `a ∈ ℝ²`, a boost `u ∈ ℝ²`, a
//! rotation `θ`, a scale `s > 0` and a mirror `Π ∈ {𝟙, diag(1, −1)}`,
//!
//! ```text
//! rᵢ' = s · R(θ) · Π · rᵢ + a
//! pᵢ' = s^{−1/2} · R(θ) · Π · pᵢ + mᵢ · u
//! ```
//!
//! The momenta scale by `s^{−1/2}`, the similarity symmetry (`r → λr`, `t → λ^{3/2}t`), and `Π` acts on the momenta as
//! on the positions: `s^{+1/2}`, or `Π` left off `p`, are the two errors the explicit form exists to prevent.

use kernel::decode::Ic;

use crate::decode_f32_eps::SplitMix;

/// One gauge element `g = (a, u, θ, s, Π)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gauge {
    /// The translation `a`.
    pub a: [f64; 2],
    /// The boost `u`.
    pub u: [f64; 2],
    /// The rotation angle `θ`.
    pub theta: f64,
    /// The scale `s > 0`.
    pub s: f64,
    /// `Π = diag(1, −1)` where set, else the identity.
    pub mirror: bool,
}

/// A uniform draw on `[lo, hi)` from `rng`'s next 53 bits.
pub fn uniform(rng: &mut SplitMix, lo: f64, hi: f64) -> f64 {
    let unit = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
    lo + (hi - lo) * unit
}

impl Gauge {
    /// The identity element.
    pub fn identity() -> Gauge {
        Gauge {
            a: [0.0; 2],
            u: [0.0; 2],
            theta: 0.0,
            s: 1.0,
            mirror: false,
        }
    }

    /// A random element: `a` and `u` uniform on `[−span, span)²`, `θ` on `[−π, π)`, `log₂ s` uniform on
    /// `[−scale_octaves, scale_octaves)`, and `Π` drawn with probability ½.
    pub fn random(rng: &mut SplitMix, span: f64, scale_octaves: f64) -> Gauge {
        let pi = std::f64::consts::PI;
        Gauge {
            a: [uniform(rng, -span, span), uniform(rng, -span, span)],
            u: [uniform(rng, -span, span), uniform(rng, -span, span)],
            theta: uniform(rng, -pi, pi),
            s: uniform(rng, -scale_octaves, scale_octaves).exp2(),
            mirror: rng.next_u64() & 1 == 1,
        }
    }

    /// `R(θ)·Π·v`.
    fn turn(&self, v: [f64; 2]) -> [f64; 2] {
        let y = if self.mirror { -v[1] } else { v[1] };
        let (sin, cos) = self.theta.sin_cos();
        [cos * v[0] - sin * y, sin * v[0] + cos * y]
    }

    /// `g·x`: every position and every momentum, per the module's formula. The masses are unchanged.
    pub fn act(&self, x: &Ic<f64>) -> Ic<f64> {
        let root = self.s.sqrt();
        let r = x.r.map(|v| {
            let t = self.turn(v);
            [self.s * t[0] + self.a[0], self.s * t[1] + self.a[1]]
        });
        let mut p = x.p;
        for (pi, mi) in p.iter_mut().zip(x.m) {
            let t = self.turn(*pi);
            *pi = [t[0] / root + mi * self.u[0], t[1] / root + mi * self.u[1]];
        }
        Ic { m: x.m, r, p }
    }
}
