//! What `hold_lz.rs`, `hold_outcomes.rs` and `hold_rank.rs` share: a seeded generator of states `(m, r, v)` on the
//! canonical section (`Σm = 1`, CoM at the origin, `I = 1`, `Σmv = 0`), and the held quantities read in f64 from the
//! inputs and outputs of either precision, which are exact in f64.

#![allow(dead_code)] // Each test file uses part of the module.

use kernel::decode::hold::{hold, Held};
use kernel::decode::invariant::ExactSign;

use crate::decode_support::P;

/// One body's two components, three bodies.
pub type Bodies = [[f64; 2]; 3];

/// A seeded splitmix64 stream: the draws are the same on every run and every machine.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform on `[0, 1)`.
    pub fn uniform(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform on `[lo, hi)`.
    pub fn between(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.uniform()
    }

    /// Standard normal, Box–Muller.
    pub fn normal(&mut self) -> f64 {
        let u = self.uniform().max(f64::MIN_POSITIVE);
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * self.uniform()).cos()
    }
}

/// `J r`.
pub fn jr(r: &Bodies) -> Bodies {
    [
        [-r[0][1], r[0][0]],
        [-r[1][1], r[1][0]],
        [-r[2][1], r[2][0]],
    ]
}

/// `⟨a, b⟩_m`.
pub fn ip(m: &[f64; 3], a: &Bodies, b: &Bodies) -> f64 {
    (0..3)
        .map(|i| m[i] * (a[i][0] * b[i][0] + a[i][1] * b[i][1]))
        .sum()
}

/// `L_z = ⟨Jr, v⟩_m`.
pub fn lz(m: &[f64; 3], r: &Bodies, v: &Bodies) -> f64 {
    ip(m, &jr(r), v)
}

/// `−Σ mᵢmⱼ/rᵢⱼ`.
pub fn potential(m: &[f64; 3], r: &Bodies) -> f64 {
    let d = |i: usize, j: usize| ((r[i][0] - r[j][0]).powi(2) + (r[i][1] - r[j][1]).powi(2)).sqrt();
    -(m[0] * m[1] / d(0, 1) + m[0] * m[2] / d(0, 2) + m[1] * m[2] / d(1, 2))
}

/// `K = ½‖v‖²_m`.
pub fn kinetic(m: &[f64; 3], v: &Bodies) -> f64 {
    0.5 * ip(m, v, v)
}

/// `|Σ mᵢvᵢ|`'s larger component.
pub fn momentum_sum(m: &[f64; 3], v: &Bodies) -> f64 {
    let s = |k: usize| (0..3).map(|i| m[i] * v[i][k]).sum::<f64>().abs();
    s(0).max(s(1))
}

/// A state with Dirichlet-ish masses, `I = 1` and the CoM at the origin, in f64.
pub fn state(rng: &mut Rng) -> ([f64; 3], Bodies) {
    let u = [
        rng.between(0.1, 1.0),
        rng.between(0.1, 1.0),
        rng.between(0.1, 1.0),
    ];
    let total: f64 = u.iter().sum();
    let m = [u[0] / total, u[1] / total, u[2] / total];
    let mut r = [[0.0; 2]; 3];
    for body in &mut r {
        *body = [rng.normal(), rng.normal()];
    }
    centre(&m, &mut r);
    let i = ip(&m, &r, &r);
    for body in &mut r {
        body[0] /= i.sqrt();
        body[1] /= i.sqrt();
    }
    (m, r)
}

/// `a` with the mass-weighted mean removed.
pub fn centre(m: &[f64; 3], a: &mut Bodies) {
    for k in 0..2 {
        let mean: f64 = (0..3).map(|i| m[i] * a[i][k]).sum();
        for body in a.iter_mut() {
            body[k] -= mean;
        }
    }
}

/// A random velocity with `Σmv = 0`.
pub fn velocity(m: &[f64; 3], rng: &mut Rng) -> Bodies {
    let mut v = [[0.0; 2]; 3];
    for body in &mut v {
        *body = [rng.normal(), rng.normal()];
    }
    centre(m, &mut v);
    v
}

/// A near-rigid velocity: `c·Jr` plus a `δv` of relative size `ratio`, orthogonal to `Jr`, with `Σmv = 0`.
pub fn near_rigid(m: &[f64; 3], r: &Bodies, ratio: f64, rng: &mut Rng) -> Bodies {
    let w = jr(r);
    let i = ip(m, &w, &w);
    let mut d = velocity(m, rng);
    let k = ip(m, &w, &d) / i;
    for b in 0..3 {
        for c in 0..2 {
            d[b][c] -= k * w[b][c];
        }
    }
    let n = ip(m, &d, &d).sqrt();
    let c = rng.normal() * 1.2 + 0.1;
    let mut v = [[0.0; 2]; 3];
    for b in 0..3 {
        for k in 0..2 {
            v[b][k] = c * w[b][k] + ratio * c.abs() * d[b][k] / n;
        }
    }
    v
}

/// `a` rounded to the precision `R`.
pub fn to<R: P>(a: &Bodies) -> [[R; 2]; 3] {
    a.map(|b| b.map(R::of))
}

/// `a` read back in f64, exactly.
pub fn from<R: P>(a: &[[R; 2]; 3]) -> Bodies {
    a.map(|b| b.map(R::f))
}

/// `hold` at the precision `R` on f64 inputs rounded to it: the outputs in f64, and the outcome.
pub fn held<R: P + ExactSign>(
    m: &[f64; 3],
    r: &Bodies,
    v: &Bodies,
    u: f64,
    r_min: f64,
    lz_star: f64,
    e_star: Option<f64>,
) -> Held<R> {
    hold(
        m.map(R::of),
        &to::<R>(r),
        &to::<R>(v),
        R::of(u),
        R::of(r_min),
        R::of(lz_star),
        e_star.is_some(),
        R::of(e_star.unwrap_or(0.0)),
    )
}
