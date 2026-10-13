//! The latent control vector `z ∈ ℝ⁸` (chart_decoder_contract Part 1, Part 2; chart_reference §1; REQ-SYS-010):
//! exactly `2 config ⊕ 4 momentum ⊕ 2 mass`, the masses inside it, and no coordinate on a gauge direction. Rotation
//! and scale are fixed by the canonical-frame decode (ρ̃ on +x, R̃ = 1, [`crate::decode::config`]), translation and
//! boost by the reconstruction's CoM, so no control is spent on any of them.
//!
//! The block layout is fixed (chart_decoder_contract Part 2, "Block ordering"):
//!
//! ```text
//! z = ( z_α, z_β | z_q0, z_q1, z_q2, z_q3 | z_μ1, z_μ2 )
//!       z[0:2]     z[2:6]                   z[6:8]
//!       config     momentum                 mass
//! ```
//!
//! It is the link registry's slot layout (`crates/ledger/src/links.rs`, `slots()`): α at `z[0]`, β at `z[1]`,
//! q0–q3 at `z[2]`–`z[5]`, the mass control from `z[6]`. Each accessor reads its block by constant indices, so no
//! runtime index reaches the GPU build (gpu_determinism_note rule 5).

/// The 8 latent controls, in the fixed block layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Latent<R> {
    /// `z[0..8]`.
    pub z: [R; 8],
}

impl<R: Copy> Latent<R> {
    /// The controls `z`, in the block layout.
    #[inline]
    pub fn new(z: [R; 8]) -> Self {
        Latent { z }
    }

    /// The configuration block, `z[0:2] = (z_α, z_β)`.
    #[inline]
    pub fn config(&self) -> [R; 2] {
        [self.z[0], self.z[1]]
    }

    /// The momentum block, `z[2:6] = (z_q0, z_q1, z_q2, z_q3)`.
    #[inline]
    pub fn momentum(&self) -> [R; 4] {
        [self.z[2], self.z[3], self.z[4], self.z[5]]
    }

    /// The mass block, `z[6:8] = (z_μ1, z_μ2)`.
    #[inline]
    pub fn mass(&self) -> [R; 2] {
        [self.z[6], self.z[7]]
    }
}
