//! The real engine's figure: the field `engine::field` computes, uploaded once as a texture, one texel per sample, its
//! outcome palette swatch, and drawn into the figure's rect at the placement the app gives, as the mock's canvas draws
//! its stand-in (`field.wgsl`). The field is computed once, blocking, before the window opens; nothing re-computes it
//! when the view moves yet.

use std::sync::{Arc, Mutex};

use engine::contract::canvas::{Canvas, CanvasContext};
use engine::contract::sim_config::SimConfig;
use engine::field::{Field, FieldBackend};

use crate::layout::Placement;

/// The state the real app starts on: the mock's (`crate::mock::state`), the chart "z_α × z_β" at 01_main.png's `z₀`,
/// the horizon 50, the kernel's default, the playhead at `t = 0`, so both engines open on the same slice. The plan is
/// unclear here (it names no start view); chose the mock's because the app's layout and captures are built around it.
pub fn start() -> (SimConfig, engine::contract::render_state::RenderState) {
    crate::mock::state(0.0)
}

/// The state every image taken from the app is taken on: [`start`]'s, the chart "z_α × z_β" untilted over its default
/// window, with the slice centre `z₀` at the origin, every slice offset 0. The capture's view is the owner's choice of
/// 20 Nov 2026 for every image taken from the app; the app's window still opens on [`start`].
pub fn capture_view() -> (SimConfig, engine::contract::render_state::RenderState) {
    let (mut sim, render) = start();
    sim.plane.z0 = [0.0; 8];
    (sim, render)
}

/// The samples across and up of the field the app shows.
pub const FIELD_SIZE: [usize; 2] = [64, 64];

/// The field of `sim`'s chart over its window, `size` samples, through `backend`: the one call the GUI makes for it.
pub fn compute(
    backend: &dyn FieldBackend,
    sim: &SimConfig,
    size: [usize; 2],
) -> Result<Field, String> {
    backend
        .field(sim, sim.chart.window, size)
        .map_err(|e| format!("gui: the field: {e}"))
}

/// The uniform the field is drawn with, as bytes: the placement and the canvas background, 8-bit sRGB, as
/// `field.wgsl`'s `FieldUniform` lays them out.
pub fn uniform_bytes(placement: Placement, background: [u8; 3]) -> Vec<u8> {
    let [cx, cy] = placement.centre;
    let [hx, hy] = placement.half;
    let [r, g, b] = background.map(|c| f32::from(c) / 255.0);
    [cx, cy, hx, hy, r, g, b, 1.0]
        .into_iter()
        .flat_map(f32::to_ne_bytes)
        .collect()
}

/// The size of the uniform, in bytes: two `vec4<f32>`.
const UNIFORM_SIZE: u64 = 2 * 16;

/// The pipeline, built for one target format, and the bind groups of the plain draw and the placed one.
#[derive(Clone)]
struct Built {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    plain: wgpu::BindGroup,
    placed: wgpu::BindGroup,
}

/// The real engine's canvas: its device, and the field's image on it.
pub struct RealCanvas {
    context: CanvasContext,
    /// The field's image.
    image: wgpu::TextureView,
    /// The uniform of the plain draw, over the whole viewport.
    buffer: wgpu::Buffer,
    /// The uniform of the placed figure, apart, so the two draws in one frame never share a placement.
    placed_buffer: wgpu::Buffer,
    /// The pipeline, built for the first target format it is drawn on.
    pipeline: Mutex<Option<Built>>,
    /// The placement of the placed figure's next draw.
    placement: Mutex<Placement>,
    /// The canvas background, 8-bit sRGB: the colour the window is cleared to.
    background: Mutex<[u8; 3]>,
}

impl RealCanvas {
    /// Opens a device (`crate::mock::canvas::open`, its backend `PRIN_GPU_BACKEND`'s) and uploads `field`.
    pub fn open(field: &Field) -> Result<Self, String> {
        Ok(Self::on(
            crate::mock::canvas::open("gui real canvas")?,
            field,
        ))
    }

    /// The canvas on the open device `context`, `field` uploaded.
    pub fn on(context: CanvasContext, field: &Field) -> Self {
        let device = &context.device;
        let background = crate::mock::canvas::default_background();
        let [w, h] = field.size.map(|n| n as u32);
        let extent = wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("real field"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING.union(wgpu::TextureUsages::COPY_DST),
            view_formats: &[],
        });
        if w > 0 && h > 0 {
            context.queue.write_texture(
                texture.as_image_copy(),
                &field.rgba8(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * w),
                    rows_per_image: Some(h),
                },
                extent,
            );
        }
        let uniform = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: UNIFORM_SIZE,
                usage: wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST),
                mapped_at_creation: false,
            })
        };
        let (buffer, placed_buffer) = (uniform("real field"), uniform("real field placed"));
        Self {
            image: texture.create_view(&Default::default()),
            buffer,
            placed_buffer,
            pipeline: Mutex::new(None),
            placement: Mutex::new(Placement::IDENTITY),
            background: Mutex::new(background),
            context,
        }
    }

    /// The placed figure's next draw puts the field at `placement` in its viewport.
    pub fn set_placement(&self, placement: Placement) {
        *self.placement.lock().expect("the placement lock") = placement;
    }

    /// The placement the placed figure's next draw puts the field at.
    pub fn placement(&self) -> Placement {
        *self.placement.lock().expect("the placement lock")
    }

    /// The canvas background the next draws use, 8-bit sRGB.
    pub fn set_background(&self, background: [u8; 3]) {
        *self.background.lock().expect("the background lock") = background;
    }

    /// The pipeline for `format`, built on first use, and its bind groups.
    fn pipeline(&self, format: wgpu::TextureFormat) -> Built {
        let mut cached = self.pipeline.lock().expect("the pipeline lock");
        if let Some(built) = cached.as_ref().filter(|b| b.format == format) {
            return built.clone();
        }
        let device = &self.context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("real field"),
            source: wgpu::ShaderSource::Wgsl(include_str!("field.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("real field"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let bind = |label, buffer: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.image),
                    },
                ],
            })
        };
        let built = Built {
            format,
            plain: bind("real field", &self.buffer),
            placed: bind("real field placed", &self.placed_buffer),
            pipeline,
        };
        *cached = Some(built.clone());
        built
    }

    /// Draws the field into `pass` on a target of `format`: placed, at [`RealCanvas::set_placement`]'s placement;
    /// plain, over the whole viewport.
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat, placed: bool) {
        let background = *self.background.lock().expect("the background lock");
        let built = self.pipeline(format);
        let (buffer, bind_group, placement) = if placed {
            let placement = *self.placement.lock().expect("the placement lock");
            (&self.placed_buffer, &built.placed, placement)
        } else {
            (&self.buffer, &built.plain, Placement::IDENTITY)
        };
        // Staged now, the write lands before the command buffer this pass is recorded into is submitted.
        self.context
            .queue
            .write_buffer(buffer, 0, &uniform_bytes(placement, background));
        pass.set_pipeline(&built.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl Canvas for RealCanvas {
    fn context(&self) -> &CanvasContext {
        &self.context
    }

    /// The plain draw: the field over the whole viewport, as the Chart builder's quick render draws it.
    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        self.draw(pass, format, false);
    }
}

/// The figure through the canvas trait: the placed draw, the field at [`RealCanvas::set_placement`]'s placement.
pub struct FigureCanvas(pub Arc<RealCanvas>);

impl Canvas for FigureCanvas {
    fn context(&self) -> &CanvasContext {
        self.0.context()
    }

    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        self.0.draw(pass, format, true);
    }
}
