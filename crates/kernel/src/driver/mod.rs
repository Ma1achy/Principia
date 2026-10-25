//! The driver layer: the wrapper's control of the bound occupant (integrator_contract Part 1, Part 3, Part 4). Its
//! substep control ([`substep`]): the position-quantised `d²`, the `N_sub` bucket over the sim key's threshold table and
//! the per-run substep record; and the host's generator of that table ([`nsub_table`]; R-502), which the SPIR-V build
//! does not take. Like the physics layer, it allocates nothing (Part 2c rule 5; `cargo xtask lint no-alloc`).

#[cfg(not(target_arch = "spirv"))]
pub mod nsub_table;
pub mod substep;
