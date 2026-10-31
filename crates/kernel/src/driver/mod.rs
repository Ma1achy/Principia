//! The driver layer: the wrapper's control of the bound occupant (integrator_contract Part 1, Part 3, Part 4). The
//! wrapper itself ([`wrapper`]): `integrate(ic, uniforms) → SimState`, the macro-step loop on its integer horizon
//! counter; the occupant seam it calls, `ADVANCE`, and the steppers' implementation of it ([`advance`]; R-19); the
//! per-substep callback it passes in ([`cadence`]). Its substep control ([`substep`]): the position-quantised `d²`, the
//! `N_sub` bucket over the sim key's threshold table and the per-run substep record; and the host's generator of that
//! table ([`nsub_table`]; R-502), which the SPIR-V build does not take. The Welford diffusion's per-macro-step update
//! and closed-form time moments ([`welford`]; payload §4). Like the physics layer, it allocates nothing
//! (Part 2c rule 5; `cargo xtask lint no-alloc`).

pub mod advance;
pub mod cadence;
#[cfg(not(target_arch = "spirv"))]
pub mod nsub_table;
pub mod substep;
pub mod welford;
pub mod wrapper;
