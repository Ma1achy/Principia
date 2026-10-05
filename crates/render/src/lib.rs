//! Fragment assembly, the compositor and the screen (systems_architecture §7.1).
//!
//! The one assembler, stain graph → `[prelude][node functions][shade()]` ([`assemble`]; render contract Part 2,
//! lowering Part 2), and the CPU mirror of the shared prelude and the presentation layer ([`present`]), whose WGSL lives in
//! `shaders/wgsl/lib/` (render_gui_spec §10.1; render contract Part 5); and the colour spaces of dd_colouring §3.1,
//! sRGB, OKLab and OKLCH, in f32 and f64 ([`colour::space`]), whose WGSL is the prelude's and
//! `shaders/wgsl/lib/colour_space.wgsl`'s.
//!
//! The fragment pipeline at runtime (TASK-M1-05): the assembled stain hashed, compiled asynchronously, cached and kept
//! last-valid per node ([`pipeline_cache`]); runtime snippets and changed occupant files swapped in ([`hot_reload`]);
//! the debug views baked one source per field on demand ([`debug_bake`]); the three fixed compositor pipelines
//! ([`compositor`]); and the render loop, which fills profiler schema v1's frame record each frame ([`frame_record`]).
//!
//! The occupant algebra's front end (TASK-M7-03): a colour or brightness occupant's expression tree to readable WGSL,
//! one function per node, its parameters uniforms clamped to their adopted ranges ([`codegen`], [`codegen::schema`];
//! colour_composition §1, §5, §8), which reaches the GPU through the one assembler.
//!
//! The image embedding lives here too (`embed`): its record format, tiled writer, read searches and majority-vote
//! reader. The plan layout names no export crate.

pub mod assemble;
pub mod codegen;
pub mod colour;
pub mod compositor;
pub mod debug_bake;
pub mod embed;
pub mod frame_record;
pub mod hot_reload;
pub mod pipeline_cache;
pub mod present;
