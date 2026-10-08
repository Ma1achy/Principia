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
