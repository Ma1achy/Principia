//! The mock's canvas (RQ-247): the `wgpu` device and queue it opens and hands the app, which builds egui-wgpu from
//! them, and the figure's stand-in, smooth procedural noise (`noise.wgsl`), drawn into the app's pass for the mock's
//! chart, which the engine side hands it before each draw (REQ-GUI-171), and over it the marks the mock draws into its
//! stand-in, as the real compositor draws them: the hover path and, with tile bounds on, the fake quads' bounds
//! (`marks.wgsl`, one-pixel lines, their geometry computed in Rust by the mock). The Stain editor's preview is the same
//! stand-in drawn in the look the mock takes from its stain graph (`noise.wgsl`'s `fs_stain`; `super::stain::look`),
//! through [`PreviewCanvas`]. The figure is drawn through [`FigureCanvas`] at the placement the app gives it, the
//! shown figure's rect in the viewport, so with the egui layer hidden it draws the field over the whole window at the
//! shown scale, each point where the shown layout puts it (R-406): past the shown rect the stand-in continues as an
//! affine chart, and a pixel there the mock flags invalid ([`MockCanvas::flag_invalid`], a test hook: the stand-in is
//! defined everywhere) is drawn with the render contract's hatch, `debug_invalid`, as forbidden (R-407's fallback). The
//! Chart builder's quick render and the Stain editor's preview draw the field over their whole viewport, never at
//! the figure's placement. The backend is `PRIN_GPU_BACKEND`'s, or the platform's when it is unset, and the one
//! chosen is logged (R-206).

use std::sync::{Arc, Mutex};

use engine::contract::canvas::hatch::{self, Tier};
use engine::contract::canvas::{Canvas, CanvasContext};
use engine::contract::sim_config::{Latent, Plane};

use crate::layout::Placement;

/// The environment variable naming the backend (R-169, R-206).
pub const BACKEND_VAR: &str = "PRIN_GPU_BACKEND";

/// The backend `value` (of [`BACKEND_VAR`]) selects: metal or vulkan; unset selects the platform's, metal on macOS
/// and vulkan elsewhere; any other value is an error (R-206).
pub fn backend(value: Option<&str>) -> Result<wgpu::Backends, String> {
    let name = value.unwrap_or(if cfg!(target_os = "macos") {
        "metal"
    } else {
        "vulkan"
    });
    match name {
        "metal" => Ok(wgpu::Backends::METAL),
        "vulkan" => Ok(wgpu::Backends::VULKAN),
        other => Err(format!(
            "{BACKEND_VAR}={other:?} is not a backend; set it to metal or vulkan"
        )),
    }
}

/// The weights of the six hidden coordinates in the stand-in's third direction, after z_α and z_β (placeholder
/// content): a slice along any of them, or a tilt toward it, changes the picture.
pub const HIDDEN_WEIGHTS: [f64; 6] = [0.7, 0.5, 0.9, 1.3, 0.6, 0.4];

/// `v` projected on the stand-in's three directions: `(z_α, z_β, hidden mix, 0)`.
pub fn project(v: &Latent) -> [f32; 4] {
    let hidden: f64 = v[2..].iter().zip(HIDDEN_WEIGHTS).map(|(z, w)| z * w).sum();
    [v[0] as f32, v[1] as f32, hidden as f32, 0.0]
}

/// The uniform the stand-in is drawn with: `z₀`, `q₁` and `q₂`, projected.
pub fn uniform(plane: &Plane) -> [[f32; 4]; 3] {
    [project(&plane.z0), project(&plane.q1), project(&plane.q2)]
}

/// The binding of the stand-in's uniform, in group 0: the chart, the placement and the region flagged invalid. The
/// shared prelude, which the stand-in's module includes for its hatch, binds its own block at
/// [`hatch::uniforms_binding`], so this is another.
pub const CHART_BINDING: u32 = 1;
/// The binding of the preview's look, in group 0.
pub const LOOK_BINDING: u32 = 2;

/// No region flagged invalid: `[s₀, t₀, s₁, t₁]` with nothing in it.
pub const NO_INVALID: [f32; 4] = [0.0; 4];

/// The stand-in's uniform, as bytes: the chart `chart`, the field's placement `placement` and the region `invalid`
/// flagged invalid, `[s₀, t₀, s₁, t₁]` of the field, as `noise.wgsl`'s `ChartUniform` lays them out.
pub fn uniform_bytes(chart: [[f32; 4]; 3], placement: Placement, invalid: [f32; 4]) -> Vec<u8> {
    let [cx, cy] = placement.centre;
    let [hx, hy] = placement.half;
    chart
        .into_iter()
        .chain([[cx, cy, hx, hy], invalid])
        .flatten()
        .flat_map(f32::to_ne_bytes)
        .collect()
}

/// The size of the stand-in's uniform, in bytes: five `vec4<f32>`.
const UNIFORM_SIZE: u64 = 5 * 16;

/// The stand-in's WGSL: the shared prelude, for `debug_invalid` and the sRGB transfer (R-407's hatch), then
/// `noise.wgsl`.
pub fn noise_source() -> String {
    format!(
        "{}\n{}",
        hatch::prelude(Tier::FULL),
        include_str!("noise.wgsl")
    )
}

/// One line the mock draws into its stand-in: its ends on the figure, `(s, t)`, `t` Y-up, its colour, straight
/// gamma-space RGBA in `[0, 1]`, as the stand-in's own output is, and whether it is drawn only outside the shown
/// figure, none of its pixels whose centre is on `[0, 1]²` (`marks.wgsl`; [`super::quads::on_shown`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    /// Its ends.
    pub ends: [[f32; 2]; 2],
    /// Its colour and opacity.
    pub rgba: [f32; 4],
    /// Drawn only outside the shown figure.
    pub outside: bool,
}

/// `lines` as the marks pipeline's vertices: two per line, each `[s, t, r, g, b, a, outside]`, `outside` 1 or 0.
pub fn vertices(lines: &[Line]) -> Vec<[f32; 7]> {
    lines
        .iter()
        .flat_map(|l| {
            l.ends.map(|[s, t]| {
                let [r, g, b, a] = l.rgba;
                [s, t, r, g, b, a, f32::from(u8::from(l.outside))]
            })
        })
        .collect()
}

/// The figure's pipeline and the marks' pipeline, built for one target format, and the bind groups of their uniform:
/// the plain draw's and the placed figure's.
#[derive(Clone)]
struct Built {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    marks: wgpu::RenderPipeline,
    plain: wgpu::BindGroup,
    placed: wgpu::BindGroup,
}

/// What the placed figure's next draw draws: the field's placement and the marks over its span.
struct Placed {
    placement: Placement,
    marks: Vec<Line>,
}

/// The preview's pipeline and the bind group of the chart's and the look's uniforms, built for one target format.
struct BuiltPreview {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

/// The mock's canvas.
pub struct MockCanvas {
    context: CanvasContext,
    /// The figure's pipeline, built for the first target format it is drawn on.
    pipeline: Mutex<Option<Built>>,
    /// The uniform buffer of the plain draw, over the whole viewport, and the preview's.
    buffer: wgpu::Buffer,
    /// The uniform buffer of the placed figure, apart, so the two draws in one frame never share a placement.
    placed_buffer: wgpu::Buffer,
    /// The chart the next draw is for.
    chart: Mutex<[[f32; 4]; 3]>,
    /// The marks the next plain draw draws over the stand-in.
    marks: Mutex<Vec<Line>>,
    /// The placed figure's next draw.
    placed: Mutex<Placed>,
    /// The region the mock flags invalid, `[s₀, t₀, s₁, t₁]` of the field (a test hook).
    invalid: Mutex<[f32; 4]>,
    /// The preview's pipeline, built for the first target format it is drawn on.
    preview: Mutex<Option<BuiltPreview>>,
    /// The preview's look's uniform buffer.
    look_buffer: wgpu::Buffer,
    /// The look the next preview is drawn in.
    look: Mutex<super::stain::Look>,
}

impl MockCanvas {
    /// Opens a device on [`BACKEND_VAR`]'s backend, and logs the adapter and backend chosen.
    pub fn new() -> Result<Self, String> {
        let backends = backend(std::env::var(BACKEND_VAR).ok().as_deref())?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
            .map_err(|e| format!("no {backends:?} adapter ({BACKEND_VAR}): {e}"))?;
        let info = adapter.get_info();
        eprintln!(
            "gui mock canvas: adapter {} ({:?})",
            info.name, info.backend
        );
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
            .map_err(|e| format!("request_device failed: {e}"))?;
        let uniform = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: UNIFORM_SIZE,
                usage: wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST),
                mapped_at_creation: false,
            })
        };
        let (buffer, placed_buffer) = (uniform("mock chart"), uniform("mock figure placed"));
        let look_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mock stain look"),
            size: std::mem::size_of::<super::stain::Look>() as u64,
            usage: wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });
        Ok(Self {
            context: CanvasContext {
                instance,
                adapter,
                device,
                queue,
            },
            pipeline: Mutex::new(None),
            buffer,
            placed_buffer,
            chart: Mutex::new([[0.0; 4]; 3]),
            marks: Mutex::new(Vec::new()),
            placed: Mutex::new(Placed {
                placement: Placement::IDENTITY,
                marks: Vec::new(),
            }),
            invalid: Mutex::new(NO_INVALID),
            preview: Mutex::new(None),
            look_buffer,
            look: Mutex::new([0.0; 4]),
        })
    }

    /// The next previews are drawn in `look`.
    pub fn set_look(&self, look: super::stain::Look) {
        *self.look.lock().expect("the look lock") = look;
    }

    /// The look the next preview is drawn in.
    pub fn look(&self) -> super::stain::Look {
        *self.look.lock().expect("the look lock")
    }

    /// The preview's pipeline and bind group for `format`.
    pub(crate) fn preview_pipeline(
        &self,
        format: wgpu::TextureFormat,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
        let mut cached = self.preview.lock().expect("the preview lock");
        if let Some(built) = cached.as_ref().filter(|b| b.format == format) {
            return (built.pipeline.clone(), built.bind_group.clone());
        }
        let device = &self.context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mock stain preview"),
            source: wgpu::ShaderSource::Wgsl(noise_source().into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mock stain preview"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_stain"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mock stain preview"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: CHART_BINDING,
                    resource: self.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: LOOK_BINDING,
                    resource: self.look_buffer.as_entire_binding(),
                },
            ],
        });
        *cached = Some(BuiltPreview {
            format,
            pipeline: pipeline.clone(),
            bind_group: bind_group.clone(),
        });
        (pipeline, bind_group)
    }

    /// Draws the preview into `pass`, whose viewport is the preview's rect: the stand-in for the chart in the look.
    pub fn draw_preview(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        let chart = *self.chart.lock().expect("the chart lock");
        let plain = uniform_bytes(chart, Placement::IDENTITY, NO_INVALID);
        self.context.queue.write_buffer(&self.buffer, 0, &plain);
        let look: Vec<u8> = self.look().iter().flat_map(|v| v.to_ne_bytes()).collect();
        self.context.queue.write_buffer(&self.look_buffer, 0, &look);
        let (pipeline, bind_group) = self.preview_pipeline(format);
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// The next draws are for the chart `plane`.
    pub fn set_chart(&self, plane: &Plane) {
        *self.chart.lock().expect("the chart lock") = uniform(plane);
    }

    /// The next plain draws, over the whole viewport, draw `marks` over the stand-in.
    pub fn set_marks(&self, marks: Vec<Line>) {
        *self.marks.lock().expect("the marks lock") = marks;
    }

    /// The placed figure's next draw ([`FigureCanvas`]) puts the field at `placement` in its viewport and draws
    /// `marks`, given over the viewport's span of the field, over the stand-in (R-406).
    pub fn set_placement(&self, placement: Placement, marks: Vec<Line>) {
        *self.placed.lock().expect("the placed lock") = Placed { placement, marks };
    }

    /// The placement the placed figure's next draw puts the field at.
    pub fn placement(&self) -> Placement {
        self.placed.lock().expect("the placed lock").placement
    }

    /// The test hook (R-407's fallback on the mock): the placed figure draws each pixel past the shown rect whose
    /// field point `(s, t)` lies in `region`, `[s₀, t₀, s₁, t₁)`, as forbidden, with the hatch; [`NO_INVALID`] flags
    /// none. Inside the shown rect, `[0, 1]²`, the fallback does not apply.
    pub fn flag_invalid(&self, region: [f32; 4]) {
        *self.invalid.lock().expect("the invalid lock") = region;
    }

    /// The figure's two pipelines, built for `format` on first use, and the bind groups of the plain and the placed
    /// draw.
    fn pipeline(&self, format: wgpu::TextureFormat) -> Built {
        let mut cached = self.pipeline.lock().expect("the pipeline lock");
        if let Some(built) = cached.as_ref().filter(|b| b.format == format) {
            return built.clone();
        }
        let device = &self.context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mock figure"),
            source: wgpu::ShaderSource::Wgsl(noise_source().into()),
        });
        let group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mock figure"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: CHART_BINDING,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(UNIFORM_SIZE),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mock figure"),
            bind_group_layouts: &[Some(&group)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mock figure"),
            layout: Some(&layout),
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
                layout: &group,
                entries: &[wgpu::BindGroupEntry {
                    binding: CHART_BINDING,
                    resource: buffer.as_entire_binding(),
                }],
            })
        };
        let built = Built {
            format,
            pipeline,
            marks: marks_pipeline(device, format, &layout),
            plain: bind("mock chart", &self.buffer),
            placed: bind("mock figure placed", &self.placed_buffer),
        };
        *cached = Some(built.clone());
        built
    }

    /// Draws the stand-in and the marks into `pass` on a target of `format`: placed, the figure at the placement and
    /// with the marks [`MockCanvas::set_placement`] gave, and the region flagged invalid hatched; plain, the field over
    /// the whole viewport with the marks [`MockCanvas::set_marks`] gave, nothing flagged.
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat, placed: bool) {
        let chart = *self.chart.lock().expect("the chart lock");
        let built = self.pipeline(format);
        let (buffer, bind_group, bytes, vertices) = if placed {
            let p = self.placed.lock().expect("the placed lock");
            let invalid = *self.invalid.lock().expect("the invalid lock");
            let bytes = uniform_bytes(chart, p.placement, invalid);
            (
                &self.placed_buffer,
                &built.placed,
                bytes,
                vertices(&p.marks),
            )
        } else {
            let bytes = uniform_bytes(chart, Placement::IDENTITY, NO_INVALID);
            let marks = self.marks.lock().expect("the marks lock");
            (&self.buffer, &built.plain, bytes, vertices(&marks))
        };
        // Staged now, the write lands before the command buffer this pass is recorded into is submitted.
        self.context.queue.write_buffer(buffer, 0, &bytes);
        pass.set_pipeline(&built.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
        if vertices.is_empty() {
            return;
        }
        let bytes: Vec<u8> = vertices
            .iter()
            .flatten()
            .flat_map(|v| v.to_ne_bytes())
            .collect();
        let buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mock marks"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });
        self.context.queue.write_buffer(&buffer, 0, &bytes);
        pass.set_pipeline(&built.marks);
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..vertices.len() as u32, 0..1);
    }
}

impl Canvas for MockCanvas {
    fn context(&self) -> &CanvasContext {
        &self.context
    }

    /// The plain draw: the field over the whole viewport, as the Chart builder's quick render draws it.
    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        self.draw(pass, format, false);
    }
}

/// The figure through the canvas trait: the placed draw, the field at [`MockCanvas::set_placement`]'s placement in a
/// viewport the app sets to the whole window, so with the egui layer hidden it fills the window (R-406).
pub struct FigureCanvas(pub Arc<MockCanvas>);

impl Canvas for FigureCanvas {
    fn context(&self) -> &CanvasContext {
        self.0.context()
    }

    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        self.0.draw(pass, format, true);
    }
}

/// The Stain editor's preview through the canvas trait: [`MockCanvas::draw_preview`] as its figure, so the app draws it
/// as it draws the figure, into the preview's rect.
pub struct PreviewCanvas(pub Arc<MockCanvas>);

impl Canvas for PreviewCanvas {
    fn context(&self) -> &CanvasContext {
        self.0.context()
    }

    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        self.0.draw_preview(pass, format);
    }
}

/// The marks' WGSL: [`super::quads::ON_SHOWN_SLACK`] as `ON_SHOWN_SLACK`, so the shader judges a pixel on the shown
/// figure as the mock does, then `marks.wgsl`.
pub fn marks_source() -> String {
    format!(
        "const ON_SHOWN_SLACK: f32 = {:e};\n{}",
        super::quads::ON_SHOWN_SLACK,
        include_str!("marks.wgsl")
    )
}

/// The marks' pipeline: one-pixel lines, alpha-blended over the stand-in, on the stand-in's `layout`, whose uniform
/// places them.
fn marks_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::PipelineLayout,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mock marks"),
        source: wgpu::ShaderSource::Wgsl(marks_source().into()),
    });
    let attributes = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mock marks"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 7]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}
