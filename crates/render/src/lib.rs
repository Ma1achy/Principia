//! Fragment assembly, the compositor and the screen (systems_architecture §7.1).
//!
//! So far: the CPU mirror of the shared prelude and the presentation layer ([`present`]), whose WGSL lives in
//! `shaders/wgsl/lib/` (render_gui_spec §10.1; render contract Part 5).
//!
//! The image embedding lives here too (`embed`): its record format, tiled writer and majority-vote reader. The plan
//! layout names no export crate.

pub mod embed;
pub mod present;
