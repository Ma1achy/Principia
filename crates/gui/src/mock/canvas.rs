//! The mock's canvas (RQ-247): the `wgpu` device and queue it opens and hands the app, which builds egui-wgpu from
//! them, and the figure's stand-in, smooth procedural noise (`noise.wgsl`), drawn into the app's pass for the mock's
//! chart, which the engine side hands it before each draw (REQ-GUI-171). The backend is `PRIN_GPU_BACKEND`'s, or the
//! platform's when it is unset, and the one chosen is logged (R-206).

use std::sync::Mutex;

use engine::contract::canvas::{Canvas, CanvasContext};
use engine::contract::sim_config::{Latent, Plane};

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

/// The figure's pipeline and the bind group of its uniform, built for one target format.
struct Built {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

/// The mock's canvas.
pub struct MockCanvas {
    context: CanvasContext,
    /// The figure's pipeline, built for the first target format it is drawn on.
    pipeline: Mutex<Option<Built>>,
    /// The chart's uniform buffer.
    buffer: wgpu::Buffer,
    /// The chart the next draw is for.
    chart: Mutex<[[f32; 4]; 3]>,
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
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mock chart"),
            size: std::mem::size_of::<[[f32; 4]; 3]>() as u64,
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
            chart: Mutex::new([[0.0; 4]; 3]),
        })
    }

    /// The next draws are for the chart `plane`.
    pub fn set_chart(&self, plane: &Plane) {
        *self.chart.lock().expect("the chart lock") = uniform(plane);
    }

    fn pipeline(&self, format: wgpu::TextureFormat) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
        let mut cached = self.pipeline.lock().expect("the pipeline lock");
        if let Some(built) = cached.as_ref() {
            if built.format == format {
                return (built.pipeline.clone(), built.bind_group.clone());
            }
        }
        let device = &self.context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mock figure"),
            source: wgpu::ShaderSource::Wgsl(include_str!("noise.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mock figure"),
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mock chart"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.buffer.as_entire_binding(),
            }],
        });
        *cached = Some(Built {
            format,
            pipeline: pipeline.clone(),
            bind_group: bind_group.clone(),
        });
        (pipeline, bind_group)
    }
}

impl Canvas for MockCanvas {
    fn context(&self) -> &CanvasContext {
        &self.context
    }

    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        let chart = *self.chart.lock().expect("the chart lock");
        let bytes: Vec<u8> = chart
            .iter()
            .flatten()
            .flat_map(|v| v.to_ne_bytes())
            .collect();
        // Staged now, the write lands before the command buffer this pass is recorded into is submitted.
        self.context.queue.write_buffer(&self.buffer, 0, &bytes);
        let (pipeline, bind_group) = self.pipeline(format);
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
