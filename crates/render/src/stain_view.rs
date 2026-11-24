//! The stain view: a `w × h` field's stored records drawn through a stain graph, into the caller's render pass or an
//! offscreen texture, at a placement in its viewport.
//!
//! **Two passes, one compile path.** The stain is the pipeline cache's ([`crate::pipeline_cache`]), assembled by the one
//! assembler ([`crate::assemble`]), drawn by the render loop's colour stage ([`RenderLoop::draw_stain`]) into its fresh
//! layer, `w × h` texels of [`LAYER_FORMAT`], one per sample: the colour pass shades sample `y · w + x` at layer pixel
//! `(x, y)`. A fixed pass, never assembled, then places the layer in the caller's viewport
//! (`shaders/wgsl/compositor/place.wgsl`), built once for the caller's target format. So a graph edit goes through the
//! cache, which compiles only a fragment key it has not compiled ([`PipelineCache::request`]); a param edit writes the
//! live node's uniform block and compiles nothing ([`RenderLoop::set_param`]); and a graph that fails to compile keeps
//! the last valid pipeline, per node, with the reason ([`StainView::errors`]).
//!
//! **The records** ([`Records`]) are the stored payload of the field's samples, as the GPU reads them: the `SimState`
//! buffer's elements at the tier's stored variant, the word buffer's `vec4<u32>` and the `ICDescriptor`s, little-endian
//! at the ledger's offsets, one of each per sample, row by row from the bottom left, sample `(x, y)` at `y · w + x`
//! (`engine::field::Field`'s order). The engine's synthetic payload harness writes them (`engine::synthetic::Bytes`).
//!
//! **The placement** ([`Placement`]) is the GUI's (`gui::layout::Placement`): the field's `[0, 1]²` as a rect in the
//! viewport's normalized device coordinates, Y-up. The placement pass is the GUI's real field's (`gui/src/real/field.wgsl`)
//! arithmetic, line for line, so a sample lands on the same pixels as the GUI's outcome image of the same field.
//!
//! **No sample is forbidden.** The colour pass shades through `shade_sample`, whose validity lane is unclassified
//! (`crate::assemble`), and the records carry no chart classification. The plan is unclear here (the thin slice names
//! only the payload records as input); chose to leave `ctx.validity.forbidden` false because the real field's samples
//! come from a chart window the engine already decoded, and a `decode_failed` sample still draws the canvas background
//! through the stain. A field whose window crosses a forbidden region needs the classification bound here.
//!
//! **Colour spaces.** The stain writes linear RGB; the layer stores it sRGB-encoded, 8 bits. An sRGB-typed target gets
//! the layer's linear view and encodes on store; any other target gets its stored bytes unconverted
//! ([`Layers::fresh_encoded`](crate::compositor::Layers::fresh_encoded)), gamma-space, as the GUI's field image reaches
//! its target. Either way a texel's 8-bit code is the stain's, unchanged.

use ledger::gen::read::Tier;

use crate::compositor::{checked, LAYER_FORMAT};
use crate::frame_record::{Applied, RenderLoop};
use crate::headless::{read_back, Image, Target};
use crate::pipeline_cache::{
    bytes, CompileError, FrameInputs, NodeKey, PipelineCache, RequestError, Requested,
};

pub use crate::assemble::Stain;

/// The placement pass's WGSL.
pub const PLACE_WGSL: &str = include_str!("../shaders/wgsl/compositor/place.wgsl");

/// The placement uniform's size in bytes: two `vec4<f32>`.
const PLACE_BYTES: u64 = 32;

/// Where the field is drawn in a viewport: the shown figure's rect, the field's `[0, 1]²`, as its centre and half-size
/// in the viewport's normalized device coordinates, Y-up (the GUI's `gui::layout::Placement`, field for field). A
/// pixel past the rect is the canvas background.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// The rect's centre.
    pub centre: [f32; 2],
    /// The rect's half-size: 1 where it spans the viewport.
    pub half: [f32; 2],
}

impl Placement {
    /// The field over the whole viewport.
    pub const IDENTITY: Placement = Placement {
        centre: [0.0, 0.0],
        half: [1.0, 1.0],
    };
}

/// A field's stored records, as bytes, one element of each buffer per sample, row by row from the bottom left: sample
/// `(x, y)`, `y` up from the bottom row, is element `y · w + x`.
#[derive(Clone, Copy, Debug)]
pub struct Records<'a> {
    /// The `SimState` buffer's elements: `SimStateFTLE` at a tier with FTLE, `SimStateBase` without.
    pub simstate: &'a [u8],
    /// The word buffer's elements, `vec4<u32>`.
    pub word: &'a [u8],
    /// The `ICDescriptor` buffer's elements.
    pub ic: &'a [u8],
}

/// The size in bytes of one element of the ledger's struct `name`.
fn stride(name: &str) -> Result<usize, String> {
    ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == name)
        .map(|s| ledger::gen::rust::offsets(&s).1 as usize)
        .ok_or_else(|| format!("the ledger has no struct `{name}`"))
}

impl Records<'_> {
    /// Each buffer's element size in bytes at `tier`: the `SimState` buffer's, the word's and the `ICDescriptor`'s.
    pub fn strides(tier: Tier) -> Result<[usize; 3], String> {
        let simstate = if tier.has_ftle {
            "SimStateFTLE"
        } else {
            "SimStateBase"
        };
        Ok([stride(simstate)?, 16, stride("ICDescriptor")?])
    }

    /// The records checked as `samples` samples' at `tier`: each buffer `samples` elements long.
    fn check(&self, samples: usize, tier: Tier) -> Result<(), String> {
        let names = ["SimState", "word", "ICDescriptor"];
        let given = [self.simstate.len(), self.word.len(), self.ic.len()];
        for ((name, len), stride) in names.iter().zip(given).zip(Records::strides(tier)?) {
            if len != samples * stride {
                return Err(format!(
                    "the {name} records are {len} B; {samples} samples of {stride} B are {} B",
                    samples * stride
                ));
            }
        }
        Ok(())
    }
}

/// The placement pass: its pipeline, built for one target format, its uniform and its bind group.
struct Place {
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
}

/// A view that draws a stain over a `w × h` field's records ([the module](self)).
pub struct StainView {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    size: [u32; 2],
    tier: Tier,
    render: RenderLoop,
    inputs: FrameInputs,
    place: Place,
    placement: Placement,
    /// The canvas background, 8-bit sRGB.
    background: [u8; 3],
}

impl StainView {
    /// A view on `device` drawing a `size = [w, h]` field's `records` at `tier` into `format` targets, with the read
    /// side's uniforms `inputs`, the field over the whole viewport ([`Placement::IDENTITY`]), the canvas background
    /// `crate::present::CANVAS_BACKGROUND_SRGB8`. No stain is drawn until one is set ([`StainView::set_stain`]) and has
    /// compiled; until then the field is the background. Refused for an empty field, records of another length, or a
    /// format the placement pass cannot draw into.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        size: [u32; 2],
        records: Records<'_>,
        tier: Tier,
        inputs: FrameInputs,
    ) -> Result<StainView, String> {
        let [w, h] = size;
        if w == 0 || h == 0 {
            return Err(format!("a {w} × {h} field has no sample"));
        }
        let render = RenderLoop::new(device, queue, LAYER_FORMAT, w, h)?;
        let place = place(device, format, &render)?;
        let mut view = StainView {
            device: device.clone(),
            queue: queue.clone(),
            format,
            size,
            tier,
            render,
            inputs,
            place,
            placement: Placement::IDENTITY,
            background: crate::present::CANVAS_BACKGROUND_SRGB8,
        };
        view.set_records(records)?;
        view.write_place();
        Ok(view)
    }

    /// The target format it draws into.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// The field's size, `[w, h]` samples.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Replaces the field's records with `records`, of the same size and tier: new buffers, bound with the next draw.
    /// Refused, the old records kept, for records of another length.
    pub fn set_records(&mut self, records: Records<'_>) -> Result<(), String> {
        let [w, h] = self.size;
        records.check(w as usize * h as usize, self.tier)?;
        use wgpu::util::DeviceExt;
        let buffer = |label: &str, contents: &[u8]| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents,
                    usage: wgpu::BufferUsages::STORAGE,
                })
        };
        let word = self
            .tier
            .has_word
            .then(|| buffer("stain view word", records.word));
        self.render
            .set_sim(buffer("stain view simstate", records.simstate), word);
        self.render.set_ic(buffer("stain view ic", records.ic));
        Ok(())
    }

    /// Sets the read side's uniforms (the masses, the ensemble and the read params), written with the next draw: a
    /// uniform, so it compiles nothing.
    pub fn set_inputs(&mut self, inputs: FrameInputs) {
        self.inputs = inputs;
    }

    /// Asks for `stain`, its nodes keyed by `keys` (the caller's stable node ids, one per node, distinct), as the stain
    /// drawn: at once when its fragment key has compiled ([`Requested::Hit`]: nothing compiles), otherwise through the
    /// cache's compile worker, swapped in by a later draw when it has compiled ([`StainView::wait`] waits for it). A
    /// stain that fails to compile keeps the last valid picture: each failing node at its last valid occupant, or the
    /// previous pipeline whole, and the reason is in [`StainView::errors`]. The node params are not in the stain; they
    /// are set by key ([`StainView::set_param`]) and kept across graph edits.
    pub fn set_stain(
        &mut self,
        stain: &Stain,
        keys: &[NodeKey],
    ) -> Result<Requested, RequestError> {
        self.render.cache().request(stain, keys, self.tier)
    }

    /// Sets the param `name` of the node keyed `key` to `value`: written to its uniform block, never a compile
    /// ([`RenderLoop::set_param`]). Refused when the node is live and does not declare `name` or admit `value`.
    pub fn set_param(
        &mut self,
        key: NodeKey,
        name: &str,
        value: &[f64],
    ) -> Result<Applied, String> {
        self.render.set_param(key, name, value)
    }

    /// Waits until every stain asked for has compiled or failed, and swaps in the latest: for startup and the tests.
    /// A frame never waits; [`StainView::prepare`] takes a finished compile without waiting.
    pub fn wait(&mut self) {
        self.render.cache().wait();
    }

    /// The pipeline cache: its compile count, its last valid occupants and the stain as it renders.
    pub fn cache(&mut self) -> &mut PipelineCache {
        self.render.cache()
    }

    /// The errors of the latest stain asked for that finished: empty when it compiled whole. A
    /// [`CompileError::Node`] names the node (its position in the stain asked for and its key) that keeps its last
    /// valid occupant; a [`CompileError::Stain`] says why the previous pipeline stays.
    pub fn errors(&mut self) -> Vec<CompileError> {
        self.render.cache().errors().to_vec()
    }

    /// How many pipelines the cache has compiled or tried to: a param edit, and a graph edit back to a compiled key,
    /// compile none.
    pub fn compiles(&mut self) -> u64 {
        self.render.cache().compiles()
    }

    /// How many times the view has made a pipeline's bind groups ([`RenderLoop::binds`]): once per pipeline swapped in
    /// and once per records change, never for a param edit.
    pub fn binds(&self) -> u64 {
        self.render.binds()
    }

    /// Sets the canvas background, 8-bit sRGB, the colour the caller's window is cleared to: past the placement, and
    /// where the stain draws the background (a `decode_failed` sample). A uniform: it compiles nothing.
    pub fn set_background(&mut self, background: [u8; 3]) {
        self.background = background;
        self.write_place();
    }

    /// Sets where the next draws put the field in the viewport: a uniform.
    pub fn set_placement(&mut self, placement: Placement) {
        self.placement = placement;
        self.write_place();
    }

    /// The placement the next draws use.
    pub fn placement(&self) -> Placement {
        self.placement
    }

    /// The placement uniform: the placement, and the background in the space the target is written in, linear for an
    /// sRGB-typed target, encoded for any other.
    fn write_place(&self) {
        let Placement { centre, half } = self.placement;
        let background = if self.format.is_srgb() {
            crate::present::srgb8(self.background).map(|c| c as f32)
        } else {
            self.background.map(|c| f32::from(c) / 255.0)
        };
        let words = [
            centre[0],
            centre[1],
            half[0],
            half[1],
            background[0],
            background[1],
            background[2],
            1.0,
        ]
        .map(f32::to_bits);
        self.queue
            .write_buffer(&self.place.uniform, 0, &bytes(&words));
    }

    /// Draws the stain into the view's layer, a sample a texel, taking a finished compile first: call it once a frame,
    /// before [`StainView::paint`]. With no stain compiled, or records the stain cannot bind, the layer is cleared,
    /// the field drawn as the background, and why is returned.
    pub fn prepare(&mut self) -> Result<(), String> {
        let background = crate::present::srgb8(self.background).map(|c| c as f32);
        self.render.set_background(background);
        self.render.draw_stain(&self.inputs)
    }

    /// Records the placement pass into `pass`, a pass over a target of the view's format: the layer at the placement,
    /// over the pass's viewport, the background past it. It draws what the last [`StainView::prepare`] drew.
    pub fn paint(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.place.pipeline);
        pass.set_bind_group(0, &self.place.group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Prepares and draws the view into a new `width × height` texture of its format, at its placement, and reads it
    /// back: the offscreen render. The format must allow `COPY_SRC`. A wgpu error is returned; a draw with no stain
    /// compiled is the background, as [`StainView::prepare`] says.
    pub fn render(&mut self, width: u32, height: u32) -> Result<Image, String> {
        // A draw with nothing compiled is drawn anyway: the background, as the GUI would show it.
        let _ = self.prepare();
        let target = Target {
            width,
            height,
            format: self.format,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("stain view target"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        checked(&self.device, "the stain view's placement", || {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("stain view"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
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
                self.paint(&mut pass);
            }
            self.queue.submit([encoder.finish()]);
        })?;
        read_back(&self.device, &self.queue, &texture, target)
    }
}

/// The placement pass for `format` targets, reading `render`'s fresh layer through the view the format needs.
fn place(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    render: &RenderLoop,
) -> Result<Place, String> {
    let layer = if format.is_srgb() {
        render.layers().fresh()
    } else {
        render.layers().fresh_encoded()
    };
    checked(device, "the stain view's placement pass", || {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("stain view placement"),
            source: wgpu::ShaderSource::Wgsl(PLACE_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("stain view placement"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("stain view placement"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("stain view placement"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_place"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_place"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stain view placement"),
            size: PLACE_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stain view placement"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(layer),
                },
            ],
        });
        Place {
            pipeline,
            uniform,
            group,
        }
    })
}

// The view is `Send`, so the GUI can hold it behind its paint callback's lock.
const _: fn() = || {
    fn send<T: Send>() {}
    send::<StainView>();
};
