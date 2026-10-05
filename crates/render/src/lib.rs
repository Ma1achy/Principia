//! Fragment assembly, the compositor and the screen (systems_architecture §7.1).
//!
//! The one assembler, stain graph → `[prelude][node functions][shade()]` ([`assemble`]; render contract Part 2,
//! lowering Part 2), and the CPU mirror of the shared prelude and the presentation layer ([`present`]), whose WGSL lives in
//! `shaders/wgsl/lib/` (render_gui_spec §10.1; render contract Part 5).
//!
//! The fragment pipeline at runtime (TASK-M1-05): the assembled stain hashed, compiled asynchronously, cached and kept
//! last-valid per node ([`pipeline_cache`]); runtime snippets and changed occupant files swapped in ([`hot_reload`]);
//! the debug views baked one source per field on demand ([`debug_bake`]); the three fixed compositor pipelines
//! ([`compositor`]); and the render loop, which fills profiler schema v1's frame record each frame ([`frame_record`]).
//!
//! The image embedding lives here too (`embed`): its record format, tiled writer, read searches and majority-vote
//! reader. The plan layout names no export crate.

pub mod assemble;
pub mod compositor;
pub mod debug_bake;
pub mod embed;
pub mod frame_record;
pub mod hot_reload;
pub mod pipeline_cache;
pub mod present;
