//! The frame record (R-56, R-113; dd_telemetry_and_tiers §5.5) and the render loop that fills it.
//!
//! [`FrameRecord`] and the types beneath it are profiler schema v1's measurement struct (TASK-M0-17; the keys and the
//! nesting are dd_telemetry_and_tiers §5's): telemetry §2's frame record, key for key, then the five stages' nested
//! sections. They live here, where the first frame loop fills them (R-56: "the measurement struct lands with the first
//! frame loop"; R-113), and the engine re-exports them as `engine::contract::profile`'s, which writes and reads the
//! file (render never depends on engine, systems_architecture §7.1).
//!
//! **Always measured, reporting toggleable** (telemetry §5.5). Every frame [`RenderLoop::frame`] renders fills a
//! record through a [`FrameClock`]: the stages it has at M1, colour and present, timed, and every other stage and
//! counter zero. Reporting only decides whether the record is also kept for the host to take
//! ([`FrameClock::take_reports`]); the measurement is the same either way, and nothing here is compiled out.
//!
//! **The render loop** ([`RenderLoop`]): the colour stage swaps in a finished pipeline ([`PipelineCache::poll`]),
//! writes the frame's uniforms and draws the current stain into the fresh layer; the present stage composites the
//! fresh layer over the backdrop into the target ([`Compositor::composite`]). A stage's time is the CPU's, from its
//! start to its submit.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Deserializer, Serialize};

use crate::compositor::{checked, Compositor, Layers};
use crate::pipeline_cache::{
    bytes, frame_buffers, CompiledStain, FrameInputs, NodeKey, PipelineCache,
};

/// One frame: telemetry §2's frame record, key for key, then the five stages' nested sections.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRecord {
    /// The frame's index in the session, from 0.
    pub frame: u64,
    /// Wall clock, ms.
    pub frame_ms: f64,
    /// New quads this frame.
    pub quads_computed: u64,
    /// Cache hits.
    pub quads_reused: u64,
    /// quads × N² × (E+1): the actual integration work.
    pub samples: u64,
    /// The honest cost measure.
    pub substeps_total: u64,
    /// How far time moved (signed): the change in the playhead's simulation time `t`, in the unit of `T_horizon`;
    /// negative when the playhead moves back. Not wall-clock time.
    pub playhead_dt: f64,
    /// Pan/zoom magnitude: > 0 when the camera moved this frame, 0 when it did not (a static frame, a batch render).
    /// v1 makes only that sign normative; the magnitude's metric is defined by the task that first consumes it.
    pub camera_delta: f64,
    /// The quad tree's maximum depth.
    pub tree_depth_max: u32,
    /// The quad tree's leaf count.
    pub leaf_count: u64,
    /// How many `d_min` values the packer received as NaN and stored as unset this frame (R-288).
    pub dmin_nan_unset: u32,
    /// How many `d_min` values the packer received negative and clamped to the floor this frame (R-288).
    pub dmin_negative_floored: u32,
    /// Each stage's ms.
    pub stage_ms: StageMs,
    /// Each stage's nested sections.
    pub stages: Stages,
    /// Each pool's live memory at the frame's end.
    pub live_memory: LiveMemory,
}

/// The memory live at a frame's end, per pool (render_gui_spec § "Profiler": memory over time, live allocations by
/// type). A snapshot, not a change, so a downsampled file still shows each kept frame's memory. The pools are disjoint:
/// a tracked allocation counts in exactly one, and the tile cache's bytes count in `tile_cache` only, never also in
/// `heap` or `gpu`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveMemory {
    /// The CPU heap.
    pub heap: PoolLive,
    /// GPU memory.
    pub gpu: PoolLive,
    /// The tile cache.
    pub tile_cache: PoolLive,
}

/// One pool's live memory: `bytes` is the sum of `by_kind`'s bytes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoolLive {
    /// The pool's total live bytes, the sum of `by_kind`'s bytes.
    pub bytes: u64,
    /// One entry for each type with live allocations in the pool.
    pub by_kind: Vec<LiveKind>,
}

/// The live allocations of one type in one pool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveKind {
    /// The type.
    pub kind: String,
    /// How many are live.
    pub count: u64,
    /// Their total size.
    pub bytes: u64,
}

/// The five stages of telemetry §2's `stage_ms`, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Integrate.
    Integrate,
    /// Reduce.
    Reduce,
    /// Colour.
    Colour,
    /// Upload.
    Upload,
    /// Present; absent from a batch render (telemetry §5.5).
    Present,
}

impl Stage {
    /// The five, in `stage_ms`'s order.
    pub const ALL: [Stage; 5] = [
        Stage::Integrate,
        Stage::Reduce,
        Stage::Colour,
        Stage::Upload,
        Stage::Present,
    ];

    /// The stage's key.
    pub fn key(self) -> &'static str {
        match self {
            Stage::Integrate => "integrate",
            Stage::Reduce => "reduce",
            Stage::Colour => "colour",
            Stage::Upload => "upload",
            Stage::Present => "present",
        }
    }
}

/// `stage_ms`: each stage's ms. `present` is `None` in a batch render (telemetry §5.5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageMs {
    /// Integrate.
    pub integrate: f64,
    /// Reduce.
    pub reduce: f64,
    /// Colour.
    pub colour: f64,
    /// Upload.
    pub upload: f64,
    /// Present.
    #[serde(deserialize_with = "nullable")]
    pub present: Option<f64>,
}

/// `stages`: each stage's nested sections, and nothing beside the five. `present` is `None` in a batch render.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stages {
    /// Integrate.
    pub integrate: StageSections,
    /// Reduce.
    pub reduce: StageSections,
    /// Colour.
    pub colour: StageSections,
    /// Upload.
    pub upload: StageSections,
    /// Present.
    #[serde(deserialize_with = "nullable")]
    pub present: Option<StageSections>,
}

impl Stages {
    /// The stage's sections; `None` for an absent present stage.
    pub fn get(&self, stage: Stage) -> Option<&StageSections> {
        match stage {
            Stage::Integrate => Some(&self.integrate),
            Stage::Reduce => Some(&self.reduce),
            Stage::Colour => Some(&self.colour),
            Stage::Upload => Some(&self.upload),
            Stage::Present => self.present.as_ref(),
        }
    }
}

/// What sits beneath one stage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageSections {
    /// The CPU scopes, nested; the finer categories (quadtree, stain + style, IC decode, readback, egui) are among them.
    pub scopes: Vec<Scope>,
    /// The GPU passes.
    pub gpu_passes: Vec<GpuPass>,
    /// The allocations the stage made, one entry per kind and pool.
    pub allocations: Vec<Allocation>,
    /// The events.
    pub events: Vec<Event>,
}

/// A CPU scope, beneath a stage or another scope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    /// Its name.
    pub name: String,
    /// Its start, ms from the start of the frame.
    pub start_ms: f64,
    /// Its duration, ms.
    pub ms: f64,
    /// The scopes inside it.
    pub children: Vec<Scope>,
}

/// A GPU pass, from the GPU timestamps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GpuPass {
    /// Its name.
    pub name: String,
    /// Its start, ms from the start of the frame.
    pub start_ms: f64,
    /// Its duration, ms.
    pub ms: f64,
}

/// The allocations of one kind in one pool that a stage made.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allocation {
    /// The allocated type.
    pub kind: String,
    /// The pool.
    pub pool: Pool,
    /// How many.
    pub count: u64,
    /// Their total size.
    pub bytes: u64,
}

/// The memory pools the profiler tracks: heap, GPU and tile cache (render_gui_spec § "Profiler").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pool {
    /// The CPU heap.
    Heap,
    /// GPU memory.
    Gpu,
    /// The tile cache.
    TileCache,
}

/// An instant event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// Its name.
    pub name: String,
    /// When, ms from the start of the frame.
    pub at_ms: f64,
    /// What happened, as text; `None` when the name says it all.
    #[serde(deserialize_with = "nullable")]
    pub detail: Option<String>,
}

/// Reads a key §5 lets be `null`: the key must be there (its absence is not `null`, and not v1), and `null` is `None`.
/// Without it, serde reads a missing `Option` key as `None`.
fn nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

// ── The frame clock ───────────────────────────────────────────────────────────────────────────────────────────────

/// Frame `frame`'s record before anything is measured: every time and counter 0, no present stage, each stage's
/// sections empty, each pool empty.
pub fn blank(frame: u64) -> FrameRecord {
    let sections = empty_sections;
    let pool = || PoolLive {
        bytes: 0,
        by_kind: Vec::new(),
    };
    FrameRecord {
        frame,
        frame_ms: 0.0,
        quads_computed: 0,
        quads_reused: 0,
        samples: 0,
        substeps_total: 0,
        playhead_dt: 0.0,
        camera_delta: 0.0,
        tree_depth_max: 0,
        leaf_count: 0,
        dmin_nan_unset: 0,
        dmin_negative_floored: 0,
        stage_ms: StageMs {
            integrate: 0.0,
            reduce: 0.0,
            colour: 0.0,
            upload: 0.0,
            present: None,
        },
        stages: Stages {
            integrate: sections(),
            reduce: sections(),
            colour: sections(),
            upload: sections(),
            present: None,
        },
        live_memory: LiveMemory {
            heap: pool(),
            gpu: pool(),
            tile_cache: pool(),
        },
    }
}

/// A stage's sections with nothing in them.
fn empty_sections() -> StageSections {
    StageSections {
        scopes: Vec::new(),
        gpu_passes: Vec::new(),
        allocations: Vec::new(),
        events: Vec::new(),
    }
}

/// Milliseconds from `from` to `to`.
fn ms(from: Instant, to: Instant) -> f64 {
    to.duration_since(from).as_secs_f64() * 1e3
}

/// One frame being measured, from [`FrameClock::begin`] to [`FrameClock::finish`].
#[derive(Debug)]
pub struct FrameTimer {
    start: Instant,
    record: FrameRecord,
}

impl FrameTimer {
    /// Runs `f` as the scope `name` of `stage`, timed: its duration added to the stage's `stage_ms` and the scope
    /// added to the stage's scopes, its start from the frame's. A present stage timed is a present stage there.
    pub fn time<R>(&mut self, stage: Stage, name: &str, f: impl FnOnce() -> R) -> R {
        let begin = Instant::now();
        let out = f();
        let end = Instant::now();
        self.add(stage, name, ms(self.start, begin), ms(begin, end));
        out
    }

    /// Adds the scope `name` of `stage`, `start_ms` from the frame's start and `ms` long: `ms` added to the stage's
    /// `stage_ms`, the scope to its scopes.
    pub fn add(&mut self, stage: Stage, name: &str, start_ms: f64, ms: f64) {
        let r = &mut self.record;
        let scope = Scope {
            name: name.to_owned(),
            start_ms,
            ms,
            children: Vec::new(),
        };
        let (total, sections) = match stage {
            Stage::Integrate => (&mut r.stage_ms.integrate, &mut r.stages.integrate),
            Stage::Reduce => (&mut r.stage_ms.reduce, &mut r.stages.reduce),
            Stage::Colour => (&mut r.stage_ms.colour, &mut r.stages.colour),
            Stage::Upload => (&mut r.stage_ms.upload, &mut r.stages.upload),
            Stage::Present => (
                r.stage_ms.present.get_or_insert(0.0),
                r.stages.present.get_or_insert_with(empty_sections),
            ),
        };
        *total += ms;
        sections.scopes.push(scope);
    }

    /// Records the event `name` in `stage`, now, with `detail`.
    pub fn event(&mut self, stage: Stage, name: &str, detail: Option<String>) {
        let at_ms = ms(self.start, Instant::now());
        let r = &mut self.record;
        let sections = match stage {
            Stage::Integrate => &mut r.stages.integrate,
            Stage::Reduce => &mut r.stages.reduce,
            Stage::Colour => &mut r.stages.colour,
            Stage::Upload => &mut r.stages.upload,
            Stage::Present => r.stages.present.get_or_insert_with(empty_sections),
        };
        if stage == Stage::Present {
            r.stage_ms.present.get_or_insert(0.0);
        }
        sections.events.push(Event {
            name: name.to_owned(),
            at_ms,
            detail,
        });
    }

    /// The record so far.
    pub fn record(&self) -> &FrameRecord {
        &self.record
    }
}

/// The frame clock: numbers frames from 0, measures every one, and keeps the records for the host while reporting is
/// on (telemetry §5.5: the reporting is toggleable, the measurement is not).
#[derive(Debug, Default)]
pub struct FrameClock {
    next: u64,
    reporting: bool,
    reports: Vec<FrameRecord>,
    last: Option<FrameRecord>,
}

impl FrameClock {
    /// A clock at frame 0, reporting or not.
    pub fn new(reporting: bool) -> FrameClock {
        FrameClock {
            reporting,
            ..FrameClock::default()
        }
    }

    /// Turns reporting on or off; the measurement goes on either way.
    pub fn set_reporting(&mut self, on: bool) {
        self.reporting = on;
    }

    /// Whether reporting is on.
    pub fn reporting(&self) -> bool {
        self.reporting
    }

    /// Starts the next frame's measurement.
    pub fn begin(&self) -> FrameTimer {
        FrameTimer {
            start: Instant::now(),
            record: blank(self.next),
        }
    }

    /// Ends `timer`'s frame: its `frame_ms` the wall clock since [`begin`](Self::begin). The record is the last one,
    /// and is kept for [`take_reports`](Self::take_reports) while reporting is on.
    pub fn finish(&mut self, timer: FrameTimer) -> &FrameRecord {
        let mut record = timer.record;
        record.frame_ms = ms(timer.start, Instant::now());
        self.next = record.frame + 1;
        if self.reporting {
            self.reports.push(record.clone());
        }
        self.last.insert(record)
    }

    /// The records kept since the last call, in frame order; the host takes them each frame.
    pub fn take_reports(&mut self) -> Vec<FrameRecord> {
        std::mem::take(&mut self.reports)
    }

    /// The last frame's record, whether reported or not.
    pub fn last(&self) -> Option<&FrameRecord> {
        self.last.as_ref()
    }

    /// The next frame's index.
    pub fn next_frame(&self) -> u64 {
        self.next
    }
}

// ── The render loop ───────────────────────────────────────────────────────────────────────────────────────────────

/// The bind groups of the pipeline they were made for, and the request whose params its node blocks hold.
struct Bound {
    stain: Arc<CompiledStain>,
    /// The [`PipelineCache::generation`] of the request whose params were last written to the node blocks: `None`
    /// until they are. Every stain of the key shares the blocks, and a request made current changes the generation, so
    /// a frame after any other request on the pipeline, a param edit made on it included, writes them again.
    written: Option<u64>,
    groups: [wgpu::BindGroup; 3],
}

/// Where [`RenderLoop::set_param`] put a param.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    /// The node is live in the current stain: its block holds the value now.
    Now,
    /// The node is not live in the current stain (no stain has compiled, its stain is still with the worker, or it is
    /// off the live graph): the value is kept and written when a stain in which it is live is bound.
    Later,
}

/// The render loop: the pipeline cache, the compositor and its layers, the frame clock, the sim buffers the colour
/// pass reads, and the node params, each by its node's key and the uniform's name.
pub struct RenderLoop {
    device: wgpu::Device,
    queue: wgpu::Queue,
    cache: PipelineCache,
    compositor: Compositor,
    layers: Layers,
    clock: FrameClock,
    prelude: wgpu::Buffer,
    frame: wgpu::Buffer,
    sim: Option<(wgpu::Buffer, Option<wgpu::Buffer>)>,
    bound: Option<Bound>,
    binds: u64,
    params: BTreeMap<(NodeKey, String), Vec<f64>>,
}

impl RenderLoop {
    /// A loop on `device` drawing `width` × `height` layers and presenting into `target`-format views: the
    /// compositor's three pipelines created now, at startup (lowering contract Part 4), reporting off.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Result<RenderLoop, String> {
        let compositor = Compositor::new(device, target)?;
        let layers = compositor.layers(device, width, height);
        let (prelude, frame) = frame_buffers(device);
        Ok(RenderLoop {
            device: device.clone(),
            queue: queue.clone(),
            cache: PipelineCache::new(device),
            compositor,
            layers,
            clock: FrameClock::new(false),
            prelude,
            frame,
            sim: None,
            bound: None,
            binds: 0,
            params: BTreeMap::new(),
        })
    }

    /// The pipeline cache: requests go through it.
    pub fn cache(&mut self) -> &mut PipelineCache {
        &mut self.cache
    }

    /// The compositor.
    pub fn compositor(&self) -> &Compositor {
        &self.compositor
    }

    /// The layers.
    pub fn layers(&self) -> &Layers {
        &self.layers
    }

    /// The frame clock.
    pub fn clock(&mut self) -> &mut FrameClock {
        &mut self.clock
    }

    /// The sim buffers the colour pass reads: the stored `SimState` buffer and, at a tier with the word, the word
    /// buffer. They are bound read-only; nothing here writes them.
    pub fn set_sim(&mut self, simstate: wgpu::Buffer, word: Option<wgpu::Buffer>) {
        self.sim = Some((simstate, word));
        self.bound = None;
    }

    /// How many times the loop has made a pipeline's bind groups: once per pipeline swapped in and again after the sim
    /// buffers change, never per frame. Another stain of the same pipeline becoming current makes none: its params are
    /// written to the shared node blocks, and the groups stay.
    pub fn binds(&self) -> u64 {
        self.binds
    }

    /// Sets the param `name` of the node keyed `key` to `value`: written to the current pipeline's block at that node's
    /// canonical position now, when it is live in the current stain ([`Applied::Now`]), and to every later pipeline's
    /// in which it is live ([`Applied::Later`] when that is the first). Refused, and not kept, when the node is live in
    /// the current stain and does not declare `name` or does not admit `value`.
    pub fn set_param(
        &mut self,
        key: NodeKey,
        name: &str,
        value: &[f64],
    ) -> Result<Applied, String> {
        let applied = match (self.cache.current(), self.cache.position(key)) {
            (Some(c), Some(position)) => {
                c.write_param(&self.queue, position, name, value)?;
                Applied::Now
            }
            _ => Applied::Later,
        };
        self.params.insert((key, name.to_owned()), value.to_vec());
        Ok(applied)
    }

    /// Renders one frame into `target`, a view of the layers' size in the compositor's target format, and returns its
    /// record: the colour stage (swap, uniforms, the stain into the fresh layer) and the present stage (the composite),
    /// each timed. With no pipeline current, or none that can bind the sim buffers, the fresh layer is cleared and
    /// the colour stage records why as the event `skipped`.
    pub fn frame(&mut self, inputs: &FrameInputs, target: &wgpu::TextureView) -> &FrameRecord {
        let mut timer = self.clock.begin();
        let skipped = timer.time(Stage::Colour, "stain", || self.colour(inputs).err());
        if let Some(why) = skipped {
            timer.event(Stage::Colour, "skipped", Some(why));
        }
        timer.time(Stage::Present, "composite", || self.present(target));
        self.clock.finish(timer)
    }

    /// The colour stage: the stain into the fresh layer, or the layer cleared and why.
    fn colour(&mut self, inputs: &FrameInputs) -> Result<(), String> {
        self.cache.poll();
        let drawn = self.bind();
        let (width, _) = self.layers.size();
        self.queue
            .write_buffer(&self.prelude, 0, &bytes(&inputs.prelude_words()));
        self.queue
            .write_buffer(&self.frame, 0, &bytes(&inputs.words(width)));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("colour"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.layers.fresh(),
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
            if let (Ok(()), Some(b)) = (&drawn, &self.bound) {
                pass.set_pipeline(b.stain.pipeline());
                for (g, group) in (0u32..).zip(&b.groups) {
                    pass.set_bind_group(g, group, &[]);
                }
                pass.draw(0..3, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);
        drawn
    }

    /// Binds the current pipeline's groups, if they are not bound already, and writes the current request's params to
    /// its node blocks, if they do not hold them already. A wgpu error, sim buffers past the device's binding limit
    /// say, is returned, and the frame skips. A frame with nothing changed allocates nothing and creates nothing.
    fn bind(&mut self) -> Result<(), String> {
        let Some(current) = self.cache.current() else {
            self.bound = None;
            return Err("no pipeline has compiled".into());
        };
        if !self
            .bound
            .as_ref()
            .is_some_and(|b| Arc::ptr_eq(&b.stain, current))
        {
            self.bound = None;
            let (simstate, word) = self
                .sim
                .as_ref()
                .ok_or_else(|| "no sim buffers are set".to_owned())?;
            let device = &self.device;
            let groups = checked(device, "the stain's bind groups", || {
                Ok::<_, String>([
                    current.uniform_group(device, &self.prelude),
                    current.sim_group(device, simstate, word.as_ref())?,
                    current.frame_group(device, &self.frame),
                ])
            })??;
            self.binds += 1;
            self.bound = Some(Bound {
                stain: Arc::clone(current),
                written: None,
                groups,
            });
        }
        let generation = Some(self.cache.generation());
        let Some(bound) = self.bound.as_mut().filter(|b| b.written != generation) else {
            return Ok(());
        };
        // Another request on the pipeline, another stain of the key or this one under other node keys, may have
        // written its own nodes' params into the shared blocks, by a bind or by a param edit made while it was current:
        // back to the defaults, then this request's params, each at its node's canonical position.
        current.write_defaults(&self.queue);
        for ((key, name), value) in &self.params {
            if let Some(position) = self.cache.position(*key) {
                // A param the new stain's node no longer declares, or no longer admits, keeps its default there.
                let _ = current.write_param(&self.queue, position, name, value);
            }
        }
        bound.written = generation;
        Ok(())
    }

    /// The present stage: the fresh layer over the backdrop, into `target`.
    fn present(&mut self, target: &wgpu::TextureView) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.compositor
            .composite(&mut encoder, &self.layers, target);
        self.queue.submit([encoder.finish()]);
    }
}
