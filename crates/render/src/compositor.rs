//! The compositor (lowering contract Part 3's fragment side, Part 4; caching contract Part 5; systems_architecture §5
//! seam 7): above the stain graph, three fixed shaders, never assembled and never varied, their pipelines created at
//! startup ([`Compositor::new`]): the backdrop pass, which copies a layer into the backdrop render target; the
//! separable blur, run twice, across then down, on the backdrop; and the composite, the fresh layer over the backdrop.
//! Their WGSL is `shaders/wgsl/compositor/` (gui_state_contract §3: fixed passes, not occupants, not scanned).
//!
//! **Layers** ([`Layers`]) are render-sized textures in [`LAYER_FORMAT`]: the fresh layer the colour pass draws, the
//! backdrop and the blur's scratch, three targets of 4 bytes a pixel (canonical spec's `k_r ≈ 3 × render_px × 4`). The
//! backdrop is updated when its reference updates, an at-rest baseline (caching contract Part 5), not every frame; a
//! frame composites the fresh layer over it. Blur means "not current"; nothing here desaturates or tints.

use crate::pipeline_cache::bytes;

/// The layers' format: 8-bit RGBA, sRGB-encoded in memory, so the passes write and read linear colour (canonical
/// spec's 4 bytes a pixel for the render-sized targets).
pub const LAYER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// The most weights a blur pass takes, the centre's and one per tap out from it: the uniform block's capacity, a blur
/// radius of at most 15 render pixels, not the radius. The radius is REQ-RENDER-079, an R-71 calibration requirement
/// (caching contract Part 5) owned by TASK-M5-26 and confirmed by the human at the M5 gate; until then the weights,
/// and so the radius, are the caller's, and the calibration may raise this capacity.
pub const MAX_TAPS: usize = 16;

/// The blur pass's uniform block's size in bytes: direction, tap count, padding, and [`MAX_TAPS`] weights.
const BLUR_BYTES: u64 = 80;

/// The full-target vertex stage, shared by the colour pass and the compositor's passes.
pub const FULL_TARGET_WGSL: &str = include_str!("../shaders/wgsl/compositor/full_target.wgsl");
/// The backdrop pass.
pub const BACKDROP_WGSL: &str = include_str!("../shaders/wgsl/compositor/backdrop.wgsl");
/// The separable blur pass.
pub const BLUR_WGSL: &str = include_str!("../shaders/wgsl/compositor/blur.wgsl");
/// The composite pass.
pub const COMPOSITE_WGSL: &str = include_str!("../shaders/wgsl/compositor/composite.wgsl");

/// `f` run with wgpu's three error scopes open, validation, internal and out-of-memory, so an error it makes is
/// returned, prefixed with `what`, never sent to the device's uncaptured-error handler: a pipeline that fails to
/// create, its shader's backend compile included, is an error the caller surfaces.
pub fn checked<T>(device: &wgpu::Device, what: &str, f: impl FnOnce() -> T) -> Result<T, String> {
    let scopes = [
        device.push_error_scope(wgpu::ErrorFilter::Validation),
        device.push_error_scope(wgpu::ErrorFilter::Internal),
        device.push_error_scope(wgpu::ErrorFilter::OutOfMemory),
    ];
    let out = f();
    let mut first = None;
    for scope in scopes.into_iter().rev() {
        if let Some(e) = pollster::block_on(scope.pop()) {
            first.get_or_insert(e);
        }
    }
    match first {
        Some(e) => Err(format!("{what}: {e}")),
        None => Ok(out),
    }
}

/// The full-target vertex stage as a shader module.
pub fn full_target_vertex(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("full_target"),
        source: wgpu::ShaderSource::Wgsl(FULL_TARGET_WGSL.into()),
    })
}

/// A blur pass's direction: across (x) or down (y).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Across,
    Down,
}

impl Direction {
    fn step(self) -> [i32; 2] {
        match self {
            Direction::Across => [1, 0],
            Direction::Down => [0, 1],
        }
    }
}

/// The blur pass's uniform block for `direction` and `weights`, the centre's first: `(dx, dy, taps, 0)`, then the
/// weights, zero-filled to [`MAX_TAPS`]. Refused for no weight, more than [`MAX_TAPS`], or a weight that is not finite.
pub fn blur_words(direction: Direction, weights: &[f32]) -> Result<Vec<u32>, String> {
    if weights.is_empty() || weights.len() > MAX_TAPS {
        return Err(format!(
            "a blur takes 1 to {MAX_TAPS} weights; {} given",
            weights.len()
        ));
    }
    if let Some(w) = weights.iter().find(|w| !w.is_finite()) {
        return Err(format!("the blur weight {w} is not finite"));
    }
    let [dx, dy] = direction.step();
    let mut words = vec![dx as u32, dy as u32, weights.len() as u32, 0];
    words.extend(weights.iter().map(|w| w.to_bits()));
    words.resize(4 + MAX_TAPS, 0);
    Ok(words)
}

/// One fixed pass: its pipeline and its bind group's layout.
#[derive(Debug)]
struct Pass {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

/// A texture binding entry, unfilterable float, visible to the fragment stage.
fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

/// `entry` of `wgsl` as a pass drawing into `format`, its group 0 laid out as `entries`.
fn pass(
    device: &wgpu::Device,
    vs: &wgpu::ShaderModule,
    wgsl: &str,
    entry: &str,
    entries: &[wgpu::BindGroupLayoutEntry],
    format: wgpu::TextureFormat,
) -> Pass {
    let fs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(entry),
        source: wgpu::ShaderSource::Wgsl(wgsl.into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(entry),
        entries,
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(entry),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entry),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: vs,
            entry_point: Some("full_target"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &fs,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    Pass { pipeline, layout }
}

/// Encodes one full-target draw of `pass` into `target`, cleared to transparent first, with `group` bound.
fn draw(
    encoder: &mut wgpu::CommandEncoder,
    pass: &Pass,
    group: &wgpu::BindGroup,
    target: &wgpu::TextureView,
    label: &str,
) {
    let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    rp.set_pipeline(&pass.pipeline);
    rp.set_bind_group(0, group, &[]);
    rp.draw(0..3, 0..1);
}

/// The three fixed compositor pipelines, created once, at startup (lowering contract Part 4: "Compositor shaders
/// always precompiled").
#[derive(Debug)]
pub struct Compositor {
    backdrop: Pass,
    blur: Pass,
    composite: Pass,
    target: wgpu::TextureFormat,
}

impl Compositor {
    /// The three pipelines: the backdrop and the blur drawing into [`LAYER_FORMAT`], the composite into `target`, the
    /// presented format. A WGSL, validation or pipeline error is returned.
    pub fn new(device: &wgpu::Device, target: wgpu::TextureFormat) -> Result<Compositor, String> {
        checked(device, "the compositor's pipelines", || {
            Compositor::create(device, target)
        })
    }

    fn create(device: &wgpu::Device, target: wgpu::TextureFormat) -> Compositor {
        let vs = full_target_vertex(device);
        let uniform = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        Compositor {
            backdrop: pass(
                device,
                &vs,
                BACKDROP_WGSL,
                "backdrop",
                &[texture_entry(0)],
                LAYER_FORMAT,
            ),
            blur: pass(
                device,
                &vs,
                BLUR_WGSL,
                "blur",
                &[texture_entry(0), uniform],
                LAYER_FORMAT,
            ),
            composite: pass(
                device,
                &vs,
                COMPOSITE_WGSL,
                "composite",
                &[texture_entry(0), texture_entry(1)],
                target,
            ),
            target,
        }
    }

    /// The format the composite draws into.
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target
    }

    /// The render-sized layers, `width` × `height`, and the bind groups the blur and the composite read them through.
    pub fn layers(&self, device: &wgpu::Device, width: u32, height: u32) -> Layers {
        let texture = |label: &str| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: LAYER_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let (fresh, backdrop, scratch) = (texture("fresh"), texture("backdrop"), texture("blur"));
        let blur_buffer = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: BLUR_BYTES,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let (across, down) = (blur_buffer("blur across"), blur_buffer("blur down"));
        let blur_group = |source: &wgpu::TextureView, buffer: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("blur"),
                layout: &self.blur.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffer.as_entire_binding(),
                    },
                ],
            })
        };
        let blur_across = blur_group(&backdrop, &across);
        let blur_down = blur_group(&scratch, &down);
        let composite = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite"),
            layout: &self.composite.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fresh),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&backdrop),
                },
            ],
        });
        Layers {
            width,
            height,
            fresh,
            backdrop,
            scratch,
            blur_buffers: [across, down],
            blur_across,
            blur_down,
            composite,
        }
    }

    /// Encodes the backdrop's update: `source`, a layer of the layers' size other than their backdrop and scratch,
    /// copied into the backdrop, then blurred across and down with `weights`, the centre's first ([`blur_words`]).
    /// Refused for weights [`blur_words`] refuses; nothing is encoded then.
    pub fn update_backdrop(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        layers: &Layers,
        source: &wgpu::TextureView,
        weights: &[f32],
    ) -> Result<(), String> {
        let across = blur_words(Direction::Across, weights)?;
        let down = blur_words(Direction::Down, weights)?;
        queue.write_buffer(&layers.blur_buffers[0], 0, &bytes(&across));
        queue.write_buffer(&layers.blur_buffers[1], 0, &bytes(&down));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backdrop"),
            layout: &self.backdrop.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source),
            }],
        });
        draw(
            encoder,
            &self.backdrop,
            &group,
            &layers.backdrop,
            "backdrop",
        );
        draw(
            encoder,
            &self.blur,
            &layers.blur_across,
            &layers.scratch,
            "blur across",
        );
        draw(
            encoder,
            &self.blur,
            &layers.blur_down,
            &layers.backdrop,
            "blur down",
        );
        Ok(())
    }

    /// Encodes the composite: the fresh layer over the backdrop, into `target`, a view of the layers' size in
    /// [`Compositor::target_format`].
    pub fn composite(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        layers: &Layers,
        target: &wgpu::TextureView,
    ) {
        draw(
            encoder,
            &self.composite,
            &layers.composite,
            target,
            "composite",
        );
    }
}

/// The render-sized layers and the bind groups that read them, from [`Compositor::layers`].
#[derive(Debug)]
pub struct Layers {
    width: u32,
    height: u32,
    fresh: wgpu::TextureView,
    backdrop: wgpu::TextureView,
    scratch: wgpu::TextureView,
    blur_buffers: [wgpu::Buffer; 2],
    blur_across: wgpu::BindGroup,
    blur_down: wgpu::BindGroup,
    composite: wgpu::BindGroup,
}

impl Layers {
    /// The layers' size, in pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The fresh layer, which the colour pass draws.
    pub fn fresh(&self) -> &wgpu::TextureView {
        &self.fresh
    }

    /// The backdrop.
    pub fn backdrop(&self) -> &wgpu::TextureView {
        &self.backdrop
    }
}
