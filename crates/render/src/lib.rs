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
//! the debug views baked one source per field on demand ([`debug_bake`]), the debug catalogue's generated views among
//! them, which the occupant registry, the scanned filesystem, surfaces ([`registry`]; TASK-M1-08); the three fixed
//! compositor pipelines ([`compositor`]); and the render loop, which fills profiler schema v1's frame record each frame
//! ([`frame_record`]).
//!
//! The synthetic payload harness's fragment side (TASK-M1-06): the four payload-side buffers and the uniforms bound
//! to the fragment, with the generated WGSL `RenderContext` and `ctx` lanes ([`bind`]); the sample → tile
//! rasterisation, one sample per tile ([`raster`]); and the headless render-to-texture helper the golden tests render
//! through ([`headless`]).
//!
//! The structural views and overlays (TASK-M1-13): the CPU mirror of the `ctx.quad` debug views, the boundary
//! overlay, the fallback tint and the pending hatch ([`structural`]), whose WGSL lives in `shaders/wgsl/frag/debug/` and
//! `shaders/wgsl/frag/post/`.
//!
//! The coordinate convention (TASK-M1-07): the one framebuffer → UV flip, `shaders/wgsl/lib/coords.wgsl` and its Rust
//! twin ([`coords`]), which the raster, picking and the golden coordinate view call; and the image export, which
//! writes the headless readback's rows as they come, top first ([`export`]).
//!
//! The occupant algebra's front end (TASK-M7-03): a colour or brightness occupant's expression tree to readable WGSL,
//! one function per node, its parameters uniforms clamped to their adopted ranges ([`codegen`], [`codegen::schema`];
//! colour_composition §1, §5, §8), which reaches the GPU through the one assembler.
//!
//! The display stage (colour_composition §4.3), outside the pipeline: the colour-vision simulation pass and its CPU
//! mirror ([`display::cvd`]; dd_colouring §3.8; TASK-M7-20).
//!
//! The image embedding lives here too (`embed`): its record format, tiled writer, read searches and majority-vote
//! reader. The plan layout names no export crate.

pub mod assemble;
pub mod bind;
pub mod codegen;
pub mod colour;
pub mod compositor;
pub mod coords;
pub mod debug_bake;
pub mod display;
pub mod embed;
pub mod export;
pub mod frame_record;
pub mod headless;
pub mod hot_reload;
pub mod pipeline_cache;
pub mod present;
pub mod raster;
pub mod registry;
pub mod structural;
