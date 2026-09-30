//! The payload structs, generated from the layout table by `cargo xtask codegen` (dd_generation_root §1;
//! dd_simstate_payload §1): `SimStateFTLE`, `SimStateBase`, the word buffer's `FreeGroupWord` and `ICDescriptor`; and
//! the packed words' pack/unpack/insert accessors, named as payload §6, with the binary16 conversion (payload §2); the
//! word buffer's `fgw_*` accessors and payload §3's frozen continuation table.
//! [`roundtrip`] holds the descriptor parity check, `roundtrip_ctl` (pitfalls §9); [`counters`] the `d_min` packer's
//! per-frame telemetry counters, which the packer's caller passes in (R-281, R-288, R-294).

pub mod counters;
pub mod roundtrip;

// The generated field and accessor names are the ledger's and payload §6's, such as `S`, `C_ty`, `E_0` and
// `pb_dE_max` (dd_simstate_payload §1, §6); the one suppression covers the whole generated file.
#[allow(non_snake_case)]
mod generated;

pub use counters::DminCounters;
pub use generated::*;
