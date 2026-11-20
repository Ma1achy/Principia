//! The sweeps that hold the kernel's own f32 `atan2` (`kernel::arctan`; RQ-628, decided by the orchestrator per R-369,
//! option (a)) to its bound: no worse than libm's `atan2f`, the arithmetic `ε_phys` was calibrated on. The kernel's
//! tests (`crates/kernel/tests/arctan.rs`) run them on the CPU, and engine's (`crates/engine/tests/arctan_gpu.rs`) on
//! the GPU as the backend compiles the function.
//!
//! - [`reduced`]: the reduced arctangent, `atan2(t, 1)` for f32 `t ∈ [0, 1]`, every `stride`-th one by its bits (all of
//!   them at stride 1), and 1 itself.
//! - [`plane`]: a dense two-dimensional sweep of `(y, x)`: magnitudes log-uniform over every f32 exponent, subnormals
//!   included, every quadrant, and signed zeros, poles (`x = ±0`, `y = ±0`, both) and the diagonals `|y| = |x|` mixed in.
//! - [`alphas`]: the encode's round trip, every `stride`-th f32 `α ∈ [0, π/2]` by its bits, from which a test forms
//!   `α' = atan2(sin α, cos α)` with the decode's f32 `sin` and `cos`, as the encode reads `α` back from the decode's
//!   `‖λ̃‖` and `‖ρ̃‖` ([`RoundTrip`]).
//!
//! Each is measured against f64 `atan2` ([`Accuracy`]): the error in ULP of the f32 nearest the exact value, the count
//! of results that are not that nearest f32, and the count more than one f32 step from it.

use crate::decode_f32_eps::SplitMix;

/// The bits of f32 `1`, the reduced sweep's last value.
pub const ONE_BITS: u32 = 0x3F80_0000;

/// The bits of the largest f32 at or below `π/2` (f32's nearest `π/2` lies above it).
pub const HALF_PI_BELOW_BITS: u32 = 0x3FC9_0FDA;

/// The reduced sweep's arguments `t ∈ [0, 1]`: every `stride`-th f32 from `+0` by its bits, and 1.
pub fn reduced(stride: u32) -> impl Iterator<Item = f32> {
    (0..=ONE_BITS / stride)
        .map(move |i| f32::from_bits(i * stride))
        .chain(core::iter::once(1.0))
}

/// The round trip's angles `α ∈ [0, π/2]`: every `stride`-th f32 from `+0` by its bits.
pub fn alphas(stride: u32) -> impl Iterator<Item = f32> {
    (0..=HALF_PI_BELOW_BITS / stride).map(move |i| f32::from_bits(i * stride))
}

/// One of the plane sweep's magnitudes: a uniform exponent field over every finite f32 (0 a subnormal) and a uniform
/// fraction, so the magnitudes are log-uniform from the least subnormal to the largest finite f32.
fn magnitude(rng: &mut SplitMix) -> f32 {
    let r = rng.next_u64();
    let field = (r % 255) as u32;
    let fraction = (r >> 32) as u32 & 0x007F_FFFF;
    f32::from_bits((field << 23) | fraction)
}

/// The plane sweep's `n` pairs `(y, x)` from `seed` (the module's note): of every 16 draws, on average one has
/// `y = ±0`, one `x = ±0`, one `|y| = |x|`, and one in 256 both zero; each sign is drawn.
pub fn plane(n: usize, seed: u64) -> Vec<[f32; 2]> {
    let mut rng = SplitMix::new(seed);
    (0..n)
        .map(|_| {
            let mut y = magnitude(&mut rng);
            let mut x = magnitude(&mut rng);
            let r = rng.next_u64();
            match (r >> 8) & 0xF {
                0 => y = 0.0,
                1 => x = 0.0,
                2 => x = y,
                _ => {}
            }
            if (r >> 12) & 0xFF == 0 {
                (y, x) = (0.0, 0.0);
            }
            let y = if r & 1 == 1 { -y } else { y };
            let x = if r & 2 == 2 { -x } else { x };
            [y, x]
        })
        .collect()
}

/// `v`'s place on a line of every f32 in order, `±0` both at 0, so that the difference of two places is the number of
/// f32 steps between them.
pub fn place(v: f32) -> i64 {
    let b = v.to_bits();
    let mag = i64::from(b & 0x7FFF_FFFF);
    if b >> 31 == 1 {
        -mag
    } else {
        mag
    }
}

/// The f32 steps between `a` and `b`.
pub fn steps(a: f32, b: f32) -> i64 {
    (place(a) - place(b)).abs()
}

/// `got`'s error from `exact` in ULP of `exact`'s nearest f32: the gap from its magnitude to the next f32 up, the least
/// subnormal at 0.
pub fn ulps(got: f32, exact: f64) -> f64 {
    let near = (exact as f32).abs();
    let ulp = f64::from(f32::from_bits(near.to_bits() + 1) - near);
    (f64::from(got) - exact).abs() / ulp
}

/// A sweep's accuracy against f64 `atan2`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Accuracy {
    /// The results measured.
    pub n: u64,
    /// The results that are not the f32 nearest the exact value.
    pub off: u64,
    /// The results more than one f32 step from it.
    pub far: u64,
    /// The largest error, in ULP ([`ulps`]).
    pub worst: f64,
    /// Where it is, `(y, x)`.
    pub worst_at: [f32; 2],
}

impl Accuracy {
    /// Adds the result `got` of `atan2(y, x)`. A NaN operand is skipped: its result is checked as a special value.
    pub fn add(&mut self, [y, x]: [f32; 2], got: f32) {
        if y.is_nan() || x.is_nan() {
            return;
        }
        let exact = f64::from(y).atan2(f64::from(x));
        let near = exact as f32;
        self.n += 1;
        self.off += u64::from(got.to_bits() != near.to_bits());
        self.far += u64::from(steps(got, near) > 1);
        let e = ulps(got, exact);
        if e > self.worst {
            self.worst = e;
            self.worst_at = [y, x];
        }
    }

    /// The two sweeps' parts, together.
    pub fn merge(self, o: Accuracy) -> Accuracy {
        let (worst, worst_at) = if o.worst > self.worst {
            (o.worst, o.worst_at)
        } else {
            (self.worst, self.worst_at)
        };
        Accuracy {
            n: self.n + o.n,
            off: self.off + o.off,
            far: self.far + o.far,
            worst,
            worst_at,
        }
    }
}

/// The encode's round trip over [`alphas`]: `α' = atan2(sin α, cos α)` with the decode's f32 `sin` and `cos`, and the
/// decode's `cos α'` and `sin α'` against `cos α` and `sin α`, the relative error the decode's `‖ρ̃‖` and `‖λ̃‖` take
/// (RQ-628).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundTrip {
    /// The angles measured.
    pub n: u64,
    /// The angles with `α' ≠ α`.
    pub moved: u64,
    /// The most f32 steps from `α` to `α'`.
    pub worst_steps: i64,
    /// The largest `|cos α' − cos α|/cos α`, in f64 from the f32 values.
    pub worst_cos: f64,
    /// The largest `|sin α' − sin α|/sin α`, `sin α = 0` skipped.
    pub worst_sin: f64,
}

impl RoundTrip {
    /// Adds `α`, its `sin α` and `cos α`, the result `α'`, and `cos α'` and `sin α'`.
    pub fn add(&mut self, alpha: f32, [s, c]: [f32; 2], back: f32, [s2, c2]: [f32; 2]) {
        self.n += 1;
        self.moved += u64::from(back != alpha);
        self.worst_steps = self.worst_steps.max(steps(back, alpha));
        if c != 0.0 {
            let rel = ((f64::from(c2) - f64::from(c)) / f64::from(c)).abs();
            self.worst_cos = self.worst_cos.max(rel);
        }
        if s != 0.0 {
            let rel = ((f64::from(s2) - f64::from(s)) / f64::from(s)).abs();
            self.worst_sin = self.worst_sin.max(rel);
        }
    }

    /// The two sweeps' parts, together.
    pub fn merge(self, o: RoundTrip) -> RoundTrip {
        RoundTrip {
            n: self.n + o.n,
            moved: self.moved + o.moved,
            worst_steps: self.worst_steps.max(o.worst_steps),
            worst_cos: self.worst_cos.max(o.worst_cos),
            worst_sin: self.worst_sin.max(o.worst_sin),
        }
    }

    /// The share of angles moved.
    pub fn share(&self) -> f64 {
        self.moved as f64 / self.n as f64
    }
}
