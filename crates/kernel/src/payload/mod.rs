//! The payload structs, generated from the layout table by `cargo xtask codegen` (dd_generation_root §1;
//! dd_simstate_payload §1): `SimStateFTLE`, `SimStateBase`, the word buffer's `FreeGroupWord` and `ICDescriptor`.

// The generated field names are the ledger's, such as `S`, `C_ty` and `E_0` (dd_simstate_payload §1); the one
// suppression covers the whole generated file.
#[allow(non_snake_case)]
mod generated;

pub use generated::*;
