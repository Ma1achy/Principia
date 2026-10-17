//! The physics layer of the shared kernel (integrator_contract Part 2c, "The split"): the three-body force, the energy
//! and `L_z`, over one set of pairwise distances, generic over [`Real`](crate::Real) and instantiated at f32 (the GPU
//! build) and f64 (the CPU build) only (R-265). It is a transcription of dd_integrator §3.1 and integrator_contract
//! Part 1's monitored invariants, with no allocation: no `Vec`, `Box`, `dyn` or `std` (Part 2c rule 5), which
//! `cargo xtask lint no-alloc` checks on every push (REQ-PERF-002).
//!
//! **One distance computation per force evaluation** (REQ-PERF-001; integrator_contract Part 1, "COM projection and
//! invariant monitoring": "The energy evaluation reuses the pairwise distances from the force computation"):
//! [`PairGeom::new`] is the only code that forms a separation, `d²`, `‖·‖` or `‖·‖³`. [`force::force`],
//! [`energy::potential`] and [`PairGeom::r_min`] take the `PairGeom` and no positions, so none of them can form a
//! second one.
//!
//! **Rules held here** (Part 2c, "Rules the new kernel must hold by construction"): the code is bounded by `Real`
//! alone, which carries no precision-dependent constant (rule 1); its one comparison, [`PairGeom::closest`]'s, is on
//! `d²` (rule 2); there is no loop (rule 3); its one min-fold is seeded from pair 0, not from an infinity
//! (rule 4).

pub mod energy;
pub mod force;
pub mod pairs;

pub use pairs::PairGeom;
