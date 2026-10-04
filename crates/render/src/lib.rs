//! Fragment assembly, the compositor and the screen (systems_architecture §7.1).
//!
//! So far: the one assembler, stain graph → `[prelude][node functions][shade()]` ([`assemble`]; render contract Part 2,
//! lowering Part 2), and the CPU mirror of the shared prelude and the presentation layer ([`present`]), whose WGSL lives in
//! `shaders/wgsl/lib/` (render_gui_spec §10.1; render contract Part 5).
//!
//! The image embedding's record format lives here too (`embed`): the plan layout names no export crate.

pub mod assemble;
pub mod embed;
pub mod present;
