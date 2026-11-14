//! The flag an output reads from the collision test's floor (integrator contract Part 7, "The floor's record,
//! defined"; dd_simstate_payload §2, `coll_floor_bits`, and §5; R-954, R-973; REQ-EVT-035).
//!
//! The payload stores, per sample, `coll_floor`: the largest floor the collision test met over its near-firing states
//! and the `d_min` state, as the code `c + 41` of the floor `2^c`, 1 to 145, with 0 "not recorded". The flag is not
//! stored: an output reads it here, from the sim key's `r_coll`, which every output is keyed by
//! (`engine::contract::sim_uniforms::SimUniforms::r_coll`), as `r_coll < 2^(code − 41)`. The floor is a power of two,
//! exact in f64, so the comparison is exact.

use kernel::detect::collision::floor_of_code;

/// The greatest code the record writes, 145, the floor `2¹⁰⁴` (dd_simstate_payload §2).
const CODE_MAX: u32 = 145;

/// Whether the sim key's `r_coll` lay below the collision test's floor in the sample whose `coll_floor` is `code`:
/// `Some(r_coll < 2^(code − 41))` for a written code, 1 to 145; `None` for 0, "not recorded" (a failed sample's), and
/// for a code the record never writes, 146 to 255. At `r_coll = 0` a written code reads `Some(true)`.
pub fn below_floor(r_coll: f64, code: u32) -> Option<bool> {
    (code != 0 && code <= CODE_MAX).then(|| r_coll < f64::from(floor_of_code(code)))
}
