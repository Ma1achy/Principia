//! Fragment assembly, the compositor and the screen (systems_architecture §7.1).
//!
//! So far: the one assembler, stain graph → `[prelude][node functions][shade()]` ([`assemble`]; render contract Part 2,
//! lowering Part 2), and the CPU mirror of the shared prelude and the presentation layer ([`present`]), whose WGSL lives in
//! `shaders/wgsl/lib/` (render_gui_spec §10.1; render contract Part 5); and the colour spaces of dd_colouring §3.1,
//! sRGB, OKLab and OKLCH, in f32 and f64 ([`colour::space`]), whose WGSL is the prelude's and
//! `shaders/wgsl/lib/colour_space.wgsl`'s.
//!
//! The image embedding lives here too (`embed`): its record format, tiled writer, read searches and majority-vote
//! reader. The plan layout names no export crate.

pub mod assemble;
pub mod colour;
pub mod embed;
pub mod present;
