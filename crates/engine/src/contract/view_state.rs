//! The ViewState record (REQ-SYS-011; chart_decoder_contract Part 2.5, "Integrity: the link is part of the experiment",
//! and Part 4, "The operation table"): what an export writes so any chart, tilted or not, restores exactly and a
//! figure's provenance pins its links. It is `SimConfig`'s canonical serialisation, not a copy of any of its fields
//! (R-483 H2): `(z₀, q₁, q₂)`, the chart params (the nonlinear chart's window among them) and each block's link id and
//! tunable parameter values (R-483 X1, X2; R-106) all ride in it, so a tilted chart serialises, saves and restores for
//! free (Part 4) and the links travel with no per-destination code.
//!
//! [`restore`] is the inverse: it reads the text back, bitwise (the canonical numbers are the shortest that read back
//! to the same `f64`, R-318), and refuses a record whose links do not each name a registry entry with exactly that
//! entry's parameters ([`Links::check`](super::sim_config::Links::check)): a parameter stripped from a record is never
//! read back under a default.

use crate::contract::canonical;
use crate::contract::sim_config::SimConfig;

/// The ViewState record of `sim`: its canonical serialisation (gui_state_contract §2; R-309, R-318; R-483 H2).
pub fn export(sim: &SimConfig) -> Result<String, canonical::Error> {
    canonical::to_string(sim)
}

/// The `SimConfig` a ViewState record holds; `Err`, naming why, for a text that is not one, a group or field missing or
/// unknown, or links the registry cannot read as written (R-483 X2, X4).
pub fn restore(record: &str) -> Result<SimConfig, String> {
    serde_json::from_str(record).map_err(|e| format!("the ViewState record does not restore: {e}"))
}
