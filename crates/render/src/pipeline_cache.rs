//! The fragment pipeline at runtime (lowering contract Part 2, Part 3's fragment side, Part 4, Part 5; render contract
//! Part 2): an assembled stain becomes a live wgpu pipeline, hashed, compiled asynchronously, cached, and kept
//! last-valid on failure, per node.
//!
//! **Key.** A compiled pipeline is identified by its [`PipelineKey`]: the stain's fragment key (the FNV-1a hash of its
//! canonical form, [`Canonical::fragment_key`](crate::assemble::Canonical::fragment_key)) plus the tier bits the
//! assembled source bakes, `has_ftle` and `has_word`. `has_ensemble` is not in it: the fragment side reads it as a
//! uniform (lowering Part 5; R-145). A node's params are not in it either: they are its uniform block's values
//! ([`CompiledStain::write_param`]), so a slider edit rebinds and never recompiles (lowering Part 3, Part 5).
//!
//! **Async compile.** [`PipelineCache::request`] answers a key already compiled at once and otherwise hands the stain
//! to one compile worker, a thread that assembles it and creates its pipeline while frames go on drawing the current
//! one. [`PipelineCache::poll`], called at the start of a frame, swaps in a finished pipeline; nothing on the frame's
//! path waits for a compiler (lowering Part 4). [`PipelineCache::prepare`] compiles without swapping, for the startup
//! precompile of the render presets (lowering Part 4).
//!
//! **Last-valid, per node** (render contract Part 2: "a broken custom keeps the last valid source in that node; the
//! other nodes are untouched"). The cache keeps, for each node by its caller's stable [`NodeKey`], the last occupant
//! that compiled. When a stain fails to assemble, each node whose occupant differs from its last valid one is tried
//! alone, every other such node at its last valid occupant; a node that fails alone takes its last valid occupant
//! back, the others keep their current ones, and the error is surfaced ([`CompileError::Node`]). Where that cannot
//! render — a failure no node with a last valid occupant explains (a new node's, or two nodes' together), or a stain
//! that fails even with the failing nodes at their last valid occupants — the previous pipeline stays and the error is
//! surfaced ([`CompileError::Stain`]).
//!
//! **The colour pass's entry.** The assembler's source has no entry point; the cache appends [`STAIN_ENTRY`], a
//! fragment entry that shades one sample per pixel, sample `y · width + x`, from the frame's uniform block
//! ([`FrameInputs`]; applied per R-369: the flat layout until the render context binds quads). Its vertex stage is the
//! compositor's full-target triangle ([`full_target_vertex`]).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};

use ledger::gen::prelude;
use wgpu::util::DeviceExt;

use crate::assemble::{
    assemble, AssembleError, Fragment, Node, Occupant, Stain, Tier, Uniform, UniformBlock,
    UniformType,
};
use crate::compositor::{checked, full_target_vertex, LAYER_FORMAT};

/// A node's identity across edits, the caller's (the engine's stable node id): what the per-node last-valid state is
/// kept by. It is not in the canonical form or the key.
pub type NodeKey = u64;

/// What identifies a compiled pipeline (lowering contract Part 5): the fragment key and the tier bits the source
/// bakes. `has_ensemble` is a uniform and not in it (R-145).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    /// The fragment key: the FNV-1a hash of the stain's canonical form.
    pub fragment: u64,
    /// The tier bakes FTLE.
    pub has_ftle: bool,
    /// The tier binds the word.
    pub has_word: bool,
}

impl PipelineKey {
    /// `stain`'s key at `tier`.
    pub fn new(stain: &Stain, tier: Tier) -> PipelineKey {
        PipelineKey {
            fragment: stain.canonical().fragment_key(),
            has_ftle: tier.has_ftle,
            has_word: tier.has_word,
        }
    }

    /// The tier the key bakes.
    pub fn tier(self) -> Tier {
        Tier {
            has_ftle: self.has_ftle,
            has_word: self.has_word,
        }
    }
}

/// The colour pass's entry, appended to the assembled source: the frame's uniform block at `@group(2) @binding(0)`
/// and the fragment entry `stain_fs`, which shades sample `y · width + x` at its pixel and writes the colour, linear
/// RGB, with alpha 1: the colour pass covers its layer opaquely (caching contract Part 5).
pub const STAIN_ENTRY: &str = r"
// ── The colour pass's entry (TASK-M1-05): one sample per pixel, sample y · width + x ──
struct StainFrame {
    masses: vec3<f32>,
    ensemble_spread: f32,
    read: ReadParams,
    width: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}
@group(2) @binding(0) var<uniform> stain_frame: StainFrame;

@fragment
fn stain_fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let i = u32(pos.y) * stain_frame.width + u32(pos.x);
    let rgb = shade_sample(i, pos.xy, stain_frame.ensemble_spread, stain_frame.masses, stain_frame.read);
    return vec4<f32>(rgb, 1.0);
}
";

/// The frame's uniform block's size in bytes: `StainFrame`, twelve words.
pub const FRAME_BYTES: u64 = 48;

/// The prelude's uniform block's size in bytes, [`prelude::uniform_words`].
const PRELUDE_BYTES: u64 = 16;

/// What the colour pass reads each frame, as uniforms (lowering Part 3: view-only state is uniform; a change never
/// recompiles): the ensemble's copy count, for `has_ensemble` (R-145), and the read side's arguments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInputs {
    /// E, the ensemble's copy count: `has_ensemble` is `E ≥ 1`.
    pub ensemble: u32,
    /// The ensemble spread the read side passes through.
    pub ensemble_spread: f32,
    /// The three masses.
    pub masses: [f32; 3],
    /// `ReadParams.dt_macro`.
    pub dt_macro: f32,
    /// `ReadParams.delta_0`.
    pub delta_0: f32,
    /// `ReadParams.n_renorm`.
    pub n_renorm: u32,
    /// `ReadParams.horizon_steps`.
    pub horizon_steps: u32,
}

impl FrameInputs {
    /// The `StainFrame` block's words for a layer `width` pixels wide, in its declaration order.
    pub fn words(&self, width: u32) -> [u32; 12] {
        [
            self.masses[0].to_bits(),
            self.masses[1].to_bits(),
            self.masses[2].to_bits(),
            self.ensemble_spread.to_bits(),
            self.dt_macro.to_bits(),
            self.delta_0.to_bits(),
            self.n_renorm,
            self.horizon_steps,
            width,
            0,
            0,
            0,
        ]
    }

    /// The prelude's uniform block's words (R-145).
    pub fn prelude_words(&self) -> [u32; 4] {
        prelude::uniform_words(self.ensemble)
    }
}

/// `words` as little-endian bytes.
pub fn bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

// ── Node uniforms: the uniformSchema's block, by WGSL's uniform layout ────────────────────────────────────────────

/// A uniform type's alignment and size in bytes in the uniform address space (WGSL §"Alignment and Size").
fn align_size(ty: UniformType) -> (u64, u64) {
    match ty {
        UniformType::F32 | UniformType::I32 | UniformType::U32 => (4, 4),
        UniformType::Vec2 => (8, 8),
        UniformType::Vec3 => (16, 12),
        UniformType::Vec4 => (16, 16),
    }
}

/// Each member's byte offset in a block of `uniforms`, in order, and the block's size: each member at the next
/// multiple of its alignment, the size rounded up to 16, a uniform struct's alignment.
pub fn block_layout(uniforms: &[Uniform]) -> (Vec<u64>, u64) {
    let mut at: u64 = 0;
    let mut offsets = Vec::with_capacity(uniforms.len());
    for u in uniforms {
        let (align, size) = align_size(u.ty);
        at = at.next_multiple_of(align);
        offsets.push(at);
        at += size;
    }
    (offsets, at.next_multiple_of(16))
}

/// `value`, a value of type `ty`, as the block stores it: each component a little-endian `f32`, `i32` or `u32`.
pub fn encode(ty: UniformType, value: &[f64]) -> Vec<u8> {
    value
        .iter()
        .flat_map(|&v| match ty {
            UniformType::I32 => (v as i32).to_le_bytes(),
            UniformType::U32 => (v as u32).to_le_bytes(),
            _ => (v as f32).to_le_bytes(),
        })
        .collect()
}

/// One node's uniform block in a compiled stain: the node, its buffer and its schema.
#[derive(Debug)]
pub struct NodeBlock {
    /// The node's position in the stain.
    pub node: usize,
    /// The node's key.
    pub key: NodeKey,
    /// Its binding in group 0.
    pub binding: u32,
    /// The block's buffer, its defaults written at compile.
    pub buffer: wgpu::Buffer,
    /// The schema, in declaration order.
    pub uniforms: Vec<Uniform>,
    /// Each uniform's byte offset.
    pub offsets: Vec<u64>,
}

// ── Compiled stains ───────────────────────────────────────────────────────────────────────────────────────────────

/// A compiled stain: the pipeline, its bind group layouts (group 0: the prelude's block and the node blocks after it;
/// group 1: the stored `SimState` buffer, and the word buffer where the tier binds it, at R-343's bindings,
/// `ledger::payload::bindings`; group 2: the frame's block, applied per R-369), and the stain as it compiled, with each
/// node's key.
#[derive(Debug)]
pub struct CompiledStain {
    key: PipelineKey,
    stain: Stain,
    keys: Vec<NodeKey>,
    source: String,
    pipeline: wgpu::RenderPipeline,
    layouts: [wgpu::BindGroupLayout; 3],
    blocks: Vec<NodeBlock>,
}

impl CompiledStain {
    /// The key it compiled under.
    pub fn key(&self) -> PipelineKey {
        self.key
    }

    /// The stain as it compiled: a failed node at its last valid occupant.
    pub fn stain(&self) -> &Stain {
        &self.stain
    }

    /// Each node's key, by position.
    pub fn keys(&self) -> &[NodeKey] {
        &self.keys
    }

    /// The WGSL it compiled from: the assembled stain and [`STAIN_ENTRY`].
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The pipeline.
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    /// The layout of bind group `g`, 0 to 2.
    pub fn layout(&self, g: usize) -> &wgpu::BindGroupLayout {
        &self.layouts[g]
    }

    /// The node uniform blocks, in binding order.
    pub fn blocks(&self) -> &[NodeBlock] {
        &self.blocks
    }

    /// Writes `value` to the uniform `name` of the node keyed `key`: a param edit, a buffer write that never
    /// recompiles (lowering Part 5). Refused when no live node of that key declares `name`, or `value` is not a value
    /// of it ([`Uniform::admits`]).
    pub fn write_param(
        &self,
        queue: &wgpu::Queue,
        key: NodeKey,
        name: &str,
        value: &[f64],
    ) -> Result<(), String> {
        let (block, k) = self
            .blocks
            .iter()
            .find_map(|b| {
                let k = b.uniforms.iter().position(|u| u.name == name)?;
                (b.key == key).then_some((b, k))
            })
            .ok_or_else(|| format!("no live node keyed {key} declares the uniform `{name}`"))?;
        let u = &block.uniforms[k];
        if !u.admits(value) {
            return Err(format!("{value:?} is not a value of `{name}`: {u:?}"));
        }
        queue.write_buffer(&block.buffer, block.offsets[k], &encode(u.ty, value));
        Ok(())
    }

    /// The bind group 0 for this stain: `prelude` at binding 0, then each node block's buffer.
    pub fn uniform_group(&self, device: &wgpu::Device, prelude: &wgpu::Buffer) -> wgpu::BindGroup {
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: prelude::uniforms_binding().binding,
            resource: prelude.as_entire_binding(),
        }];
        entries.extend(self.blocks.iter().map(|b| wgpu::BindGroupEntry {
            binding: b.binding,
            resource: b.buffer.as_entire_binding(),
        }));
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stain uniforms"),
            layout: &self.layouts[0],
            entries: &entries,
        })
    }

    /// The bind group 1 for this stain: the stored `SimState` buffer, and the word buffer where the tier binds it.
    /// Refused when the word is given at a tier without it, or missing at a tier with it.
    pub fn sim_group(
        &self,
        device: &wgpu::Device,
        simstate: &wgpu::Buffer,
        word: Option<&wgpu::Buffer>,
    ) -> Result<wgpu::BindGroup, String> {
        if word.is_some() != self.key.has_word {
            return Err(format!(
                "the stain's tier has_word = {}; a word buffer was {}given",
                self.key.has_word,
                if word.is_some() { "" } else { "not " }
            ));
        }
        let [s, w] = ledger::payload::bindings();
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: s.binding,
            resource: simstate.as_entire_binding(),
        }];
        entries.extend(word.map(|word| wgpu::BindGroupEntry {
            binding: w.binding,
            resource: word.as_entire_binding(),
        }));
        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stain sim buffers"),
            layout: &self.layouts[1],
            entries: &entries,
        }))
    }

    /// The bind group 2: the frame's block.
    pub fn frame_group(&self, device: &wgpu::Device, frame: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stain frame"),
            layout: &self.layouts[2],
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            }],
        })
    }
}

/// A uniform-buffer layout entry at `binding`, visible to the fragment stage.
fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    buffer_entry(binding, wgpu::BufferBindingType::Uniform)
}

fn buffer_entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// `fragment`, the assembled `stain` at `key`'s tier, as a pipeline: the source with [`STAIN_ENTRY`] appended, its
/// layouts, and each node block's buffer with its defaults. A WGSL, validation or pipeline error is returned.
fn create(
    device: &wgpu::Device,
    key: PipelineKey,
    stain: Stain,
    keys: Vec<NodeKey>,
    fragment: Fragment,
) -> Result<CompiledStain, String> {
    let source = format!("{}{STAIN_ENTRY}", fragment.source);
    let order = stain.canonical().order().to_vec();
    checked(device, "the stain's pipeline", || {
        objects(device, key, &source, &fragment.uniforms, &order, &keys)
    })
    .map(|(pipeline, layouts, blocks)| CompiledStain {
        key,
        stain,
        keys,
        source,
        pipeline,
        layouts,
        blocks,
    })
}

/// The wgpu objects of [`create`]: the pipeline, its three bind group layouts, and the node blocks.
fn objects(
    device: &wgpu::Device,
    key: PipelineKey,
    source: &str,
    uniforms: &[UniformBlock],
    order: &[usize],
    keys: &[NodeKey],
) -> (
    wgpu::RenderPipeline,
    [wgpu::BindGroupLayout; 3],
    Vec<NodeBlock>,
) {
    let fs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("stain"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let vs = full_target_vertex(device);
    let mut group0 = vec![uniform_entry(prelude::uniforms_binding().binding)];
    group0.extend(uniforms.iter().map(|b| uniform_entry(b.binding)));
    let read_only = wgpu::BufferBindingType::Storage { read_only: true };
    let [simstate, word] = ledger::payload::bindings();
    let mut group1 = vec![buffer_entry(simstate.binding, read_only)];
    if key.has_word {
        group1.push(buffer_entry(word.binding, read_only));
    }
    let layout = |label: &str, entries: &[wgpu::BindGroupLayoutEntry]| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries,
        })
    };
    let layouts = [
        layout("stain uniforms", &group0),
        layout("stain sim buffers", &group1),
        layout("stain frame", &[uniform_entry(0)]),
    ];
    let refs: Vec<Option<&wgpu::BindGroupLayout>> = layouts.iter().map(Some).collect();
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("stain"),
        bind_group_layouts: &refs,
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("stain"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vs,
            entry_point: Some("full_target"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &fs,
            entry_point: Some("stain_fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: LAYER_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let blocks = uniforms
        .iter()
        .map(|b| {
            let (offsets, size) = block_layout(&b.uniforms);
            let mut contents = vec![0u8; size as usize];
            for (u, &at) in b.uniforms.iter().zip(&offsets) {
                let v = encode(u.ty, &u.default);
                contents[at as usize..at as usize + v.len()].copy_from_slice(&v);
            }
            let node = order[b.node];
            NodeBlock {
                node,
                key: keys[node],
                binding: b.binding,
                buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("node uniforms"),
                    contents: &contents,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                }),
                uniforms: b.uniforms.clone(),
                offsets,
            }
        })
        .collect();
    (pipeline, layouts, blocks)
}

/// A new prelude uniform buffer and a new frame uniform buffer, each writable, for the colour pass's groups 0 and 2.
pub fn frame_buffers(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer) {
    let buffer = |label: &str, size: u64| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    };
    (
        buffer("prelude uniforms", PRELUDE_BYTES),
        buffer("stain frame", FRAME_BYTES),
    )
}

// ── Errors ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// A compile failure, surfaced by [`PipelineCache::errors`].
#[derive(Clone, Debug, PartialEq)]
pub enum CompileError {
    /// A node whose occupant failed on its own: it keeps its last valid occupant, the other nodes their current ones.
    Node {
        /// Its position in the stain requested.
        node: usize,
        /// Its key.
        key: NodeKey,
        /// Why its occupant failed.
        error: AssembleError,
    },
    /// The stain as requested cannot render: the previous pipeline stays.
    Stain(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Node { node, key, error } => write!(
                f,
                "node {node} (key {key}) keeps its last valid source: {error}"
            ),
            CompileError::Stain(why) => write!(f, "the previous pipeline stays: {why}"),
        }
    }
}

/// What [`PipelineCache::request`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requested {
    /// The key was compiled: its pipeline is current now.
    Hit,
    /// The stain went to the compile worker; [`PipelineCache::poll`] swaps it in when it is done.
    Queued,
    /// The same request is already with the worker.
    Pending,
}

/// Why a request was refused before it reached the worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestError(pub String);

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RequestError {}

// ── The cache and its worker ──────────────────────────────────────────────────────────────────────────────────────

/// One request for the worker. Only the latest request's ticket swaps in; a prepare's never does.
struct Job {
    ticket: u64,
    stain: Stain,
    keys: Vec<NodeKey>,
    tier: Tier,
}

/// The worker's answer to a [`Job`].
struct Outcome {
    ticket: u64,
    compiled: Option<Arc<CompiledStain>>,
    errors: Vec<CompileError>,
}

/// What the frame's side and the worker share: the device, the cache, each node's last valid occupant, and the count
/// of pipelines created.
struct Shared {
    device: wgpu::Device,
    cache: Mutex<HashMap<PipelineKey, Arc<CompiledStain>>>,
    last_valid: Mutex<HashMap<NodeKey, Occupant>>,
    compiles: AtomicU64,
}

/// The lock's value, a panic in another holder notwithstanding: the maps hold no invariant a panic could break midway.
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The fragment pipeline cache: compiled stains by key, the current one, the per-node last-valid state, and the
/// compile worker.
pub struct PipelineCache {
    shared: Arc<Shared>,
    jobs: Sender<Job>,
    outcomes: Receiver<Outcome>,
    current: Option<Arc<CompiledStain>>,
    errors: Vec<CompileError>,
    next_ticket: u64,
    /// The latest request still with the worker, its ticket and key: only its outcome becomes current.
    latest: Option<(u64, PipelineKey)>,
    in_flight: usize,
}

impl PipelineCache {
    /// An empty cache on `device`, its compile worker started. The worker stops when the cache is dropped, once it has
    /// finished the job it is on.
    pub fn new(device: &wgpu::Device) -> PipelineCache {
        let shared = Arc::new(Shared {
            device: device.clone(),
            cache: Mutex::new(HashMap::new()),
            last_valid: Mutex::new(HashMap::new()),
            compiles: AtomicU64::new(0),
        });
        let (jobs, rx) = mpsc::channel::<Job>();
        let (tx, outcomes) = mpsc::channel::<Outcome>();
        let worker_shared = Arc::clone(&shared);
        // A worker that cannot start leaves `jobs` with no receiver: every request is refused, naming it.
        let _ = std::thread::Builder::new()
            .name("render-compile".into())
            .spawn(move || {
                for job in rx {
                    let (compiled, errors) = resolve(&worker_shared, &job);
                    let outcome = Outcome {
                        ticket: job.ticket,
                        compiled,
                        errors,
                    };
                    if tx.send(outcome).is_err() {
                        break;
                    }
                }
            });
        PipelineCache {
            shared,
            jobs,
            outcomes,
            current: None,
            errors: Vec::new(),
            next_ticket: 0,
            latest: None,
            in_flight: 0,
        }
    }

    /// Asks for `stain` at `tier`, its nodes keyed by `keys` (one per node, distinct), as the current pipeline: at once
    /// when its key is compiled ([`Requested::Hit`]), else through the worker ([`Requested::Queued`]); a request equal
    /// to the latest one still with the worker is [`Requested::Pending`].
    pub fn request(
        &mut self,
        stain: &Stain,
        keys: &[NodeKey],
        tier: Tier,
    ) -> Result<Requested, RequestError> {
        check_keys(stain, keys)?;
        let key = PipelineKey::new(stain, tier);
        let hit = lock(&self.shared.cache).get(&key).cloned();
        if let Some(compiled) = hit {
            let mut last_valid = lock(&self.shared.last_valid);
            for (n, &k) in stain.nodes().iter().zip(keys) {
                last_valid.insert(k, n.occupant.clone());
            }
            self.current = Some(compiled);
            self.errors.clear();
            self.latest = None;
            return Ok(Requested::Hit);
        }
        if self.latest.is_some_and(|(_, k)| k == key) {
            return Ok(Requested::Pending);
        }
        let ticket = self.send(stain, keys, tier)?;
        self.latest = Some((ticket, key));
        Ok(Requested::Queued)
    }

    /// Compiles `stain` at `tier` into the cache without making it current: the startup precompile of the render
    /// presets (lowering Part 4). A key already compiled is not compiled again.
    pub fn prepare(
        &mut self,
        stain: &Stain,
        keys: &[NodeKey],
        tier: Tier,
    ) -> Result<(), RequestError> {
        check_keys(stain, keys)?;
        if !lock(&self.shared.cache).contains_key(&PipelineKey::new(stain, tier)) {
            self.send(stain, keys, tier)?;
        }
        Ok(())
    }

    fn send(&mut self, stain: &Stain, keys: &[NodeKey], tier: Tier) -> Result<u64, RequestError> {
        let ticket = self.next_ticket;
        let job = Job {
            ticket,
            stain: stain.clone(),
            keys: keys.to_vec(),
            tier,
        };
        self.jobs
            .send(job)
            .map_err(|_| RequestError("the compile worker is not running".into()))?;
        self.next_ticket += 1;
        self.in_flight += 1;
        Ok(ticket)
    }

    /// Takes the worker's finished answers without waiting, swapping in the latest request's pipeline when it
    /// compiled; whether the current pipeline changed. The start of each frame calls it.
    pub fn poll(&mut self) -> bool {
        let mut swapped = false;
        while let Ok(outcome) = self.outcomes.try_recv() {
            swapped |= self.take(outcome);
        }
        swapped
    }

    /// Waits until the worker has answered every request, swapping in as [`poll`](Self::poll) does; whether the
    /// current pipeline changed. For the startup precompile and the tests: a frame never waits.
    pub fn wait(&mut self) -> bool {
        let mut swapped = false;
        while self.in_flight > 0 {
            let Ok(outcome) = self.outcomes.recv() else {
                break;
            };
            swapped |= self.take(outcome);
        }
        swapped
    }

    fn take(&mut self, outcome: Outcome) -> bool {
        self.in_flight -= 1;
        if self.latest.is_none_or(|(t, _)| t != outcome.ticket) {
            return false;
        }
        self.latest = None;
        self.errors = outcome.errors;
        match outcome.compiled {
            Some(compiled) => {
                self.current = Some(compiled);
                true
            }
            None => false,
        }
    }

    /// The current pipeline, the one frames draw; `None` until a request compiles.
    pub fn current(&self) -> Option<&Arc<CompiledStain>> {
        self.current.as_ref()
    }

    /// The compiled pipeline of `key`, if it is in the cache.
    pub fn get(&self, key: PipelineKey) -> Option<Arc<CompiledStain>> {
        lock(&self.shared.cache).get(&key).cloned()
    }

    /// The errors of the latest request that finished: empty when it compiled whole.
    pub fn errors(&self) -> &[CompileError] {
        &self.errors
    }

    /// How many pipelines the worker has created or tried to: a cache hit creates none.
    pub fn compiles(&self) -> u64 {
        self.shared.compiles.load(Ordering::SeqCst)
    }

    /// How many requests are still with the worker.
    pub fn in_flight(&self) -> usize {
        self.in_flight
    }

    /// The last occupant of the node keyed `key` that compiled.
    pub fn last_valid(&self, key: NodeKey) -> Option<Occupant> {
        lock(&self.shared.last_valid).get(&key).cloned()
    }
}

/// `keys` as `stain`'s node keys: one per node, distinct.
fn check_keys(stain: &Stain, keys: &[NodeKey]) -> Result<(), RequestError> {
    if keys.len() != stain.nodes().len() {
        return Err(RequestError(format!(
            "{} node keys for {} nodes",
            keys.len(),
            stain.nodes().len()
        )));
    }
    let mut seen = keys.to_vec();
    seen.sort_unstable();
    seen.dedup();
    if seen.len() != keys.len() {
        return Err(RequestError("two nodes share a key".into()));
    }
    Ok(())
}

/// The worker's work on `job`: the stain as it can render, each failing node at its last valid occupant (the module
/// docs), compiled or taken from the cache; and the errors to surface.
fn resolve(shared: &Shared, job: &Job) -> (Option<Arc<CompiledStain>>, Vec<CompileError>) {
    let last_valid = lock(&shared.last_valid).clone();
    let nodes = job.stain.nodes();
    let (stain, fragment, failed) = match assemble(&job.stain, job.tier) {
        Ok(fragment) => (job.stain.clone(), fragment, Vec::new()),
        Err(whole) => {
            // The nodes that can fall back: changed, with a last valid occupant. A node with none stays as it is in
            // every trial, and a failure only it explains is the stain's.
            let revertible: Vec<usize> = (0..nodes.len())
                .filter(|&i| {
                    last_valid
                        .get(&job.keys[i])
                        .is_some_and(|o| *o != nodes[i].occupant)
                })
                .collect();
            let failed: Vec<(usize, AssembleError)> = revertible
                .iter()
                .filter_map(|&i| {
                    let trial = with_last_valid(nodes, &job.keys, &last_valid, |j| {
                        j != i && revertible.contains(&j)
                    });
                    Stain::new(trial)
                        .and_then(|s| assemble(&s, job.tier))
                        .err()
                        .map(|e| (i, e))
                })
                .collect();
            if failed.is_empty() {
                return (None, vec![CompileError::Stain(whole.to_string())]);
            }
            let fallback = with_last_valid(nodes, &job.keys, &last_valid, |j| {
                failed.iter().any(|(i, _)| *i == j)
            });
            match Stain::new(fallback).and_then(|s| assemble(&s, job.tier).map(|f| (s, f))) {
                Ok((s, f)) => (s, f, failed),
                Err(e) => return (None, vec![CompileError::Stain(e.to_string())]),
            }
        }
    };
    let errors: Vec<CompileError> = failed
        .iter()
        .map(|(i, e)| CompileError::Node {
            node: *i,
            key: job.keys[*i],
            error: e.clone(),
        })
        .collect();
    let key = PipelineKey::new(&stain, job.tier);
    let cached = lock(&shared.cache).get(&key).cloned();
    let compiled = match cached {
        Some(c) => c,
        None => {
            shared.compiles.fetch_add(1, Ordering::SeqCst);
            match create(&shared.device, key, stain, job.keys.clone(), fragment) {
                Ok(c) => {
                    let c = Arc::new(c);
                    lock(&shared.cache).insert(key, Arc::clone(&c));
                    c
                }
                Err(e) => return (None, vec![CompileError::Stain(e)]),
            }
        }
    };
    let mut last = lock(&shared.last_valid);
    for (i, (n, &k)) in nodes.iter().zip(&job.keys).enumerate() {
        if !failed.iter().any(|(f, _)| *f == i) {
            last.insert(k, n.occupant.clone());
        }
    }
    (Some(compiled), errors)
}

/// `nodes` with each node `j` for which `swap(j)` holds at its key's last valid occupant; `swap` holds only for nodes
/// that have one.
fn with_last_valid(
    nodes: &[Node],
    keys: &[NodeKey],
    last_valid: &HashMap<NodeKey, Occupant>,
    swap: impl Fn(usize) -> bool,
) -> Vec<Node> {
    nodes
        .iter()
        .enumerate()
        .map(|(j, n)| {
            let mut n = n.clone();
            if let Some(o) = last_valid.get(&keys[j]).filter(|_| swap(j)) {
                n.occupant = o.clone();
            }
            n
        })
        .collect()
}
