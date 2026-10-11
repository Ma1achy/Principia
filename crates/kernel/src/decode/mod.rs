//! The shared decoder `D`, the Matter rung (dd_decoder §1, §3): `z → mass × config × momentum → (m, r, p)`, one
//! source for the f32 kernel and the f64 CPU path (R-265), generic over the kernel's `Real` and over the link
//! [`Selection`](crate::generated::links::Selection), so each selection is a monomorphised variant (REQ-GEN-013;
//! lowering Part 2).
//!
//! The blocks so far, in the decode order mass → config (chart_decoder_contract Part 2):
//! - [`latent`]: the 8-component control vector and its fixed block layout.
//! - [`mass`]: `D_mass`, the simplex point and the one tag the block may raise, `DEGENERATE(M01_TINY)`.
//! - [`config`]: `D_cfg`, the canonical-frame hyperspherical decode, unweighted and reconstructed.
//! - [`degenerate`]: the `DEGENERATE` reason type.
//! - [`units`]: the dimensionless system `G = M = I = 1`.
//!
//! **No guard** (REQ-DEC-028; validation ground-truth note, "Degenerate configs"): the decoder accepts any finite `z`
//! (R-407) and lands on no endpoint; degenerate configurations are approached as limits, and the only tag is the
//! narrow, enumerated `DEGENERATE` cause.

pub mod config;
pub mod degenerate;
pub mod latent;
pub mod mass;
pub mod units;

pub use degenerate::Degenerate;
pub use latent::Latent;
