//! The ledger generates code into the kernel at build time (R-185). Stub (TASK-M0-01): generates nothing yet.
//!
//! It declares `target_arch = "spirv"`, rust-gpu's target, as an expected cfg: the `#[spirv]` attributes expand to
//! `cfg(target_arch = "spirv")` in this crate, which rustc's check-cfg does not know (canonical_spec §1 item 2).
use ledger as _;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(target_arch, values(\"spirv\"))");
}
