//! Profiler schema v1 (R-56): a superset of telemetry §2, in JSON. At the top level, §2's frame record and its five
//! stages; beneath them, nested scopes, GPU passes, allocations and events. The keys and the nesting are
//! dd_telemetry_and_tiers §5, "Profiler schema v1: the keys and the nesting" (REQ-TOOL-120, R-72); the JSON Schema is
//! [`SCHEMA_V1`].
//!
//! [`FrameRecord`] is the measurement struct, always compiled: the reporting is toggleable, the measurement is not
//! (telemetry §5.5). The engine writes it, `prin` reads and writes it, and the dev GUI's profiler reads it.

use std::io;

use serde::{Deserialize, Serialize};

/// The JSON Schema of profiler schema v1, checked in beside this module.
pub const SCHEMA_V1: &str = include_str!("schema/profile_v1.json");

/// The value of a v1 file's `schema` key.
pub const SCHEMA_ID: &str = "principia-profile-v1";

/// The `schema` key: only v1 reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaId {
    /// `"principia-profile-v1"`, [`SCHEMA_ID`].
    #[serde(rename = "principia-profile-v1")]
    V1,
}

/// One profiler file: the session header, the frame records, then the precomputed summaries (telemetry §5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trace {
    /// Always [`SchemaId::V1`].
    pub schema: SchemaId,
    /// The session header, once.
    pub header: SessionHeader,
    /// The frame records.
    pub frames: Vec<FrameRecord>,
    /// The precomputed leak flags; `None` when not precomputed.
    pub leak_flags: Option<Vec<Summary>>,
    /// The precomputed hot-path summaries; `None` when not precomputed.
    pub hot_paths: Option<Vec<Summary>>,
}

/// One leak flag or hot-path summary (render_gui_spec § "Profiler"): a JSON object whose keys the task closing
/// REQ-TOOL-100 defines. Until then, any object.
pub type Summary = serde_json::Map<String, serde_json::Value>;

/// The session header: telemetry §2's per-session fields, and the full config §5 requires.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionHeader {
    /// The GPU and CPU, their core counts, and the memory.
    pub device: Device,
    /// The graphics API and the driver.
    pub backend: Backend,
    /// f32 and f64 support, and the reported f64 rate.
    pub precision: Precision,
    /// The build's provenance.
    pub build: Build,
    /// The display; `None` for a headless run.
    pub display: Option<Display>,
    /// The run's full configuration, a JSON object (telemetry §5, "Self-contained").
    pub config: serde_json::Map<String, serde_json::Value>,
}

/// The device (telemetry §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    /// The GPU model.
    pub gpu: String,
    /// The CPU model.
    pub cpu: String,
    /// The CPU's core count.
    pub cpu_cores: u32,
    /// The GPU's core count; `None` when not reported.
    pub gpu_cores: Option<u32>,
    /// VRAM or unified memory.
    pub memory: Memory,
}

/// The device memory: unified memory is its own field, not a VRAM size of zero (telemetry §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Memory {
    /// One pool shared by CPU and GPU (Apple silicon).
    Unified {
        /// Its size.
        bytes: u64,
    },
    /// Separate VRAM and RAM.
    Discrete {
        /// The VRAM size.
        vram_bytes: u64,
        /// The RAM size.
        ram_bytes: u64,
    },
}

/// The backend (telemetry §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backend {
    /// The graphics API.
    pub api: Api,
    /// The driver version.
    pub driver: String,
}

/// The graphics APIs telemetry §2 names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Api {
    /// Metal.
    Metal,
    /// Vulkan.
    Vulkan,
    /// DirectX 12.
    Dx12,
    /// WebGPU.
    Webgpu,
}

/// Precision support (telemetry §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Precision {
    /// Whether f32 is supported.
    pub f32: bool,
    /// Whether f64 is supported.
    pub f64: bool,
    /// The reported f64 rate as a fraction of the f32 rate; `None` when not reported.
    pub f64_rate: Option<f64>,
}

/// The build (telemetry §2; the provenance line of §5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// The commit hash.
    pub commit: String,
    /// The release profile.
    pub profile: String,
    /// The feature flags.
    pub features: Vec<String>,
}

/// The display (telemetry §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Display {
    /// The width, in pixels.
    pub width_px: u32,
    /// The height, in pixels.
    pub height_px: u32,
    /// The refresh rate, in Hz.
    pub refresh_hz: f64,
    /// The DPI scale.
    pub dpi_scale: f64,
}

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    pub detail: Option<String>,
}

/// Writes `trace` as schema v1: indented JSON, readable by the sender (telemetry §5). A value outside its range (a
/// negative ms, NaN or an infinity), a frame with one `present` null and the other not, or a pool whose `bytes` is not
/// the sum of its `by_kind` bytes is an error, and nothing is written.
pub fn write<W: io::Write>(trace: &Trace, writer: W) -> Result<(), serde_json::Error> {
    check_ranges(trace).map_err(<serde_json::Error as serde::ser::Error>::custom)?;
    serde_json::to_writer_pretty(writer, trace)
}

/// Reads a schema v1 file. A key outside v1 — a scope beside the five stages, say — is an error, and so are a value
/// outside its range, a frame with one `present` null and the other not, and a pool whose `bytes` is not the sum of its
/// `by_kind` bytes, so what `read` accepts validates against [`SCHEMA_V1`].
pub fn read<R: io::Read>(reader: R) -> Result<Trace, serde_json::Error> {
    let trace: Trace = serde_json::from_reader(reader)?;
    check_ranges(&trace).map_err(<serde_json::Error as serde::de::Error>::custom)?;
    Ok(trace)
}

/// The rules of dd_telemetry_and_tiers §5's definition that the Rust types don't already hold: every number finite,
/// and ≥ 0 except `playhead_dt`; `stage_ms.present` and `stages.present` null together; each pool's `bytes` the sum
/// of its `by_kind` bytes. The integers' widths are the types'.
fn check_ranges(trace: &Trace) -> Result<(), String> {
    let header = &trace.header;
    if let Some(rate) = header.precision.f64_rate {
        non_negative("header.precision.f64_rate", rate)?;
    }
    if let Some(display) = &header.display {
        non_negative("header.display.refresh_hz", display.refresh_hz)?;
        non_negative("header.display.dpi_scale", display.dpi_scale)?;
    }
    for (i, frame) in trace.frames.iter().enumerate() {
        let at = |key: &str| format!("frames[{i}].{key}");
        non_negative(&at("frame_ms"), frame.frame_ms)?;
        if !frame.playhead_dt.is_finite() {
            return Err(format!(
                "{} is {}, not a finite number",
                at("playhead_dt"),
                frame.playhead_dt
            ));
        }
        non_negative(&at("camera_delta"), frame.camera_delta)?;
        let ms = &frame.stage_ms;
        let stage_ms = [ms.integrate, ms.reduce, ms.colour, ms.upload];
        for (stage, value) in Stage::ALL.iter().zip(stage_ms) {
            non_negative(&at(&format!("stage_ms.{}", stage.key())), value)?;
        }
        if let Some(present) = ms.present {
            non_negative(&at("stage_ms.present"), present)?;
        }
        if ms.present.is_some() != frame.stages.present.is_some() {
            return Err(format!(
                "{} and {} are not both null or both present",
                at("stage_ms.present"),
                at("stages.present")
            ));
        }
        for stage in Stage::ALL {
            if let Some(sections) = frame.stages.get(stage) {
                check_sections(&at(&format!("stages.{}", stage.key())), sections)?;
            }
        }
        let live = &frame.live_memory;
        for (pool, name) in [
            (&live.heap, "heap"),
            (&live.gpu, "gpu"),
            (&live.tile_cache, "tile_cache"),
        ] {
            let sum: u128 = pool.by_kind.iter().map(|k| u128::from(k.bytes)).sum();
            if sum != u128::from(pool.bytes) {
                return Err(format!(
                    "{} is {}, not the sum of its by_kind bytes ({sum})",
                    at(&format!("live_memory.{name}.bytes")),
                    pool.bytes
                ));
            }
        }
    }
    Ok(())
}

fn check_sections(path: &str, sections: &StageSections) -> Result<(), String> {
    fn check_scopes(path: &str, scopes: &[Scope]) -> Result<(), String> {
        for (i, scope) in scopes.iter().enumerate() {
            let at = format!("{path}[{i}]");
            non_negative(&format!("{at}.start_ms"), scope.start_ms)?;
            non_negative(&format!("{at}.ms"), scope.ms)?;
            check_scopes(&format!("{at}.children"), &scope.children)?;
        }
        Ok(())
    }
    check_scopes(&format!("{path}.scopes"), &sections.scopes)?;
    for (i, pass) in sections.gpu_passes.iter().enumerate() {
        non_negative(&format!("{path}.gpu_passes[{i}].start_ms"), pass.start_ms)?;
        non_negative(&format!("{path}.gpu_passes[{i}].ms"), pass.ms)?;
    }
    for (i, event) in sections.events.iter().enumerate() {
        non_negative(&format!("{path}.events[{i}].at_ms"), event.at_ms)?;
    }
    Ok(())
}

fn non_negative(path: &str, value: f64) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!("{path} is {value}, not a finite number >= 0"))
    }
}
