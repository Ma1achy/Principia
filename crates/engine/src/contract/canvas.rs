//! The canvas, the one sanctioned exception to the data boundary (gui_state_contract §1; RQ-247): the engine side's
//! `wgpu` context, which egui-wgpu is built from so it paints onto the same surface in-process (render_gui_spec §G1),
//! and the figure, drawn into the app's render pass. No state rides on it. It is a second trait, kept apart from the
//! data contract ([`EngineInterface`](crate::contract::interface::EngineInterface)) and outside the conformance suite;
//! the gui crate's mock engine implements it, and the real engine's implementation is TASK-M8-05's (REQ-GUI-070).

/// The engine side's `wgpu` context, handed to the app once, at startup (gui_state_contract §1).
#[derive(Clone, Debug)]
pub struct CanvasContext {
    /// The instance the adapter came from.
    pub instance: wgpu::Instance,
    /// The adapter the device was opened on.
    pub adapter: wgpu::Adapter,
    /// The device the figure is drawn with, which egui-wgpu shares.
    pub device: wgpu::Device,
    /// The device's queue.
    pub queue: wgpu::Queue,
}

/// The canvas the engine side provides (RQ-247).
pub trait Canvas: Send + Sync {
    /// The `wgpu` context egui-wgpu is built from.
    fn context(&self) -> &CanvasContext;

    /// Draws the figure into `pass`, whose viewport the app has set to the figure's rect, on a target of `format`.
    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat);
}

/// The hatch a canvas draws a forbidden pixel with, past the chart's `[0,1]²` (R-407's fallback;
/// chart_decoder_contract § "Past the unit square — each axis's extension type (R-407)"): the render contract's
/// `debug_invalid(frag_xy)` (Part 5), re-exported from where it is generated, so a canvas outside the render crate's
/// assembler, the mock's (TASK-M6-30), draws the one hatch the corpus defines rather than a copy of it.
pub mod hatch {
    /// The shared prelude's WGSL at a tier, which defines `debug_invalid(frag_xy)` (linear RGB) and the sRGB transfer
    /// both ways, `srgb_to_linear` and `linear_to_srgb`; its uniform block's binding, which a module including it
    /// must leave free; and the hatch's colours and stripe width.
    pub use ledger::gen::prelude::{
        hatch, uniforms_binding, wgsl as prelude, Hatch, UniformsBinding,
    };
    /// The tier the prelude is emitted at.
    pub use ledger::gen::read::Tier;
    /// Which of the hatch's two colours `debug_invalid` draws at a pixel, the render crate's CPU mirror.
    pub use render::present::hatch_stripe;
}
