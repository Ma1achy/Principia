//! The saturated controls of `qa_TASK-M2-06_gpu`'s `qa_m206_gpu_saturated_controls`, in one place, so the GPU test and
//! its CPU counterpart (`decode_saturated_beta`) read the same samples in the same order: sample 410 is the defaults'
//! first latent at 10³⁰, every component `+10³⁰`, at the chart's centre.

use kernel::chart::Latent;
use validation::decode_f32_eps::Batch;

/// The magnitudes: from Metal's failure (43.671875, where its `tanh` gives 0, then 44 and 44.5), past f32's and f64's
/// `exp` overflow, and to f32's largest.
pub const MAGNITUDES: [f32; 7] = [43.671_875, 44.0, 44.5, 1e3, 1e4, 1e30, f32::MAX];

/// Charts at saturation: every component at `±s` in all-plus, all-minus and 16 random sign patterns per magnitude;
/// and each component alone at `±s`, the others drawn on `[−2, 2]`. Each decoded at the chart's centre and corners
/// (`q₁`, `q₂` 0 or 1, so the corners move the controls further).
pub fn saturated() -> Vec<Batch> {
    let mut rng = validation::decode_f32_eps::SplitMix::new(2026);
    let mut out = Vec::new();
    let uv = vec![[0.5, 0.5], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for s in MAGNITUDES {
        let mut patterns: Vec<[f32; 8]> = vec![[s; 8], [-s; 8]];
        for _ in 0..16 {
            patterns.push(core::array::from_fn(|_| {
                if rng.next_u64() & 1 == 0 {
                    s
                } else {
                    -s
                }
            }));
        }
        for z in patterns {
            out.push((
                Latent {
                    z0: z,
                    q1: [0.0; 8],
                    q2: [0.0; 8],
                },
                vec![[0.5, 0.5]],
            ));
        }
        for k in 0..8 {
            for sign in [1.0, -1.0] {
                let mut z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-2.0, 2.0));
                z[k] = sign * s;
                let mut q1 = [0.0; 8];
                q1[k] = sign;
                out.push((
                    Latent {
                        z0: z,
                        q1,
                        q2: [0.0; 8],
                    },
                    uv.clone(),
                ));
            }
        }
    }
    out
}
