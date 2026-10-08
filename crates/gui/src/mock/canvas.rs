//! The mock's canvas (RQ-247): the `wgpu` device and queue it opens and hands the app, which builds egui-wgpu from
//! them, and the figure's stand-in, smooth procedural noise (`noise.wgsl`), drawn into the app's pass. The backend is
//! `PRIN_GPU_BACKEND`'s, or the platform's when it is unset, and the one chosen is logged (R-206).

use std::sync::Mutex;

use engine::contract::canvas::{Canvas, CanvasContext};

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

/// The mock's canvas.
pub struct MockCanvas {
    context: CanvasContext,
    /// The figure's pipeline, built for the first target format it is drawn on.
    pipeline: Mutex<Option<(wgpu::TextureFormat, wgpu::RenderPipeline)>>,
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
        Ok(Self {
            context: CanvasContext {
                instance,
                adapter,
                device,
                queue,
            },
            pipeline: Mutex::new(None),
        })
    }

    fn pipeline(&self, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        let mut cached = self.pipeline.lock().expect("the pipeline lock");
        if let Some((built, pipeline)) = cached.as_ref() {
            if *built == format {
                return pipeline.clone();
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
        *cached = Some((format, pipeline.clone()));
        pipeline
    }
}

impl Canvas for MockCanvas {
    fn context(&self) -> &CanvasContext {
        &self.context
    }

    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        pass.set_pipeline(&self.pipeline(format));
        pass.draw(0..3, 0..1);
    }
}
