//! Profiler schema v1 (R-56): a superset of telemetry §2, in JSON. At the top level, §2's frame record and its five
//! stages; beneath them, nested scopes, GPU passes, allocations and events. The keys and the nesting are
//! dd_telemetry_and_tiers §5, "Profiler schema v1: the keys and the nesting" (REQ-TOOL-120, R-72); the JSON Schema is
//! [`SCHEMA_V1`].
//!
//! [`FrameRecord`] is the measurement struct, always compiled: the reporting is toggleable, the measurement is not
//! (telemetry §5.5). The engine writes it, `prin` reads and writes it, and the dev GUI's profiler reads it.

use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::io::{self, Write};

use serde::{Deserialize, Deserializer, Serialize};

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
    #[serde(deserialize_with = "nullable")]
    pub leak_flags: Option<Vec<Summary>>,
    /// The precomputed hot-path summaries; `None` when not precomputed.
    #[serde(deserialize_with = "nullable")]
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
    #[serde(deserialize_with = "nullable")]
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
    #[serde(deserialize_with = "nullable")]
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
    #[serde(deserialize_with = "nullable")]
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

/// Writes `trace` as schema v1: indented JSON, readable by the sender (telemetry §5). A value outside its range (a
/// negative ms, NaN or an infinity), a frame with one `present` null and the other not, a pool whose `bytes` is not the
/// sum of its `by_kind` bytes, or two entries for one type in a pool's `by_kind` or for one kind and pool in a stage's
/// `allocations` is an error, and nothing is written.
///
/// The writer is buffered here and flushed before `write` returns, so a plain `File` costs no more than a `BufWriter`.
pub fn write<W: io::Write>(trace: &Trace, writer: W) -> Result<(), serde_json::Error> {
    check_ranges(trace).map_err(<serde_json::Error as serde::ser::Error>::custom)?;
    let mut writer = io::BufWriter::new(writer);
    serde_json::to_writer_pretty(&mut writer, trace)?;
    writer.flush().map_err(serde_json::Error::io)
}

/// Reads a schema v1 file. A key outside v1 — a scope beside the five stages, say — is an error, and so are a missing
/// key, even one whose value may be `null`, a value outside its range, a frame with one `present` null and the other
/// not, a pool whose `bytes` is not the sum of its `by_kind` bytes, and two entries for one type in a pool's `by_kind`
/// or for one kind and pool in a stage's `allocations`, so what `read` accepts validates against [`SCHEMA_V1`].
///
/// The reader is buffered here, so a plain `File` costs no more than a `BufReader`.
pub fn read<R: io::Read>(reader: R) -> Result<Trace, serde_json::Error> {
    let trace: Trace = serde_json::from_reader(io::BufReader::new(reader))?;
    check_ranges(&trace).map_err(<serde_json::Error as serde::de::Error>::custom)?;
    Ok(trace)
}

/// The rules of dd_telemetry_and_tiers §5's definition that the Rust types don't already hold: every number finite,
/// and ≥ 0 except `playhead_dt`; `stage_ms.present` and `stages.present` null together; each pool's `bytes` the sum
/// of its `by_kind` bytes; one `by_kind` entry per type in a pool, and one `allocations` entry per kind and pool in a
/// stage. The integers' widths are the types'.
fn check_ranges(trace: &Trace) -> Result<(), String> {
    // One set of each for the whole trace, cleared for each list, so its capacity is allocated once, not per frame.
    let mut kinds = Seen::default();
    let mut kind_pools = Seen::default();
    let header = At::Key(&At::Root, "header");
    if let Some(rate) = trace.header.precision.f64_rate {
        let precision = At::Key(&header, "precision");
        non_negative(&At::Key(&precision, "f64_rate"), rate)?;
    }
    if let Some(display) = &trace.header.display {
        let at = At::Key(&header, "display");
        non_negative(&At::Key(&at, "refresh_hz"), display.refresh_hz)?;
        non_negative(&At::Key(&at, "dpi_scale"), display.dpi_scale)?;
    }
    let frames = At::Key(&At::Root, "frames");
    for (i, frame) in trace.frames.iter().enumerate() {
        let at = At::Index(&frames, i);
        non_negative(&At::Key(&at, "frame_ms"), frame.frame_ms)?;
        if !frame.playhead_dt.is_finite() {
            return Err(format!(
                "{} is {}, not a finite number",
                At::Key(&at, "playhead_dt"),
                frame.playhead_dt
            ));
        }
        non_negative(&At::Key(&at, "camera_delta"), frame.camera_delta)?;
        let ms = &frame.stage_ms;
        let stage_ms = At::Key(&at, "stage_ms");
        let values = [ms.integrate, ms.reduce, ms.colour, ms.upload];
        for (stage, value) in Stage::ALL.iter().zip(values) {
            non_negative(&At::Key(&stage_ms, stage.key()), value)?;
        }
        if let Some(present) = ms.present {
            non_negative(&At::Key(&stage_ms, "present"), present)?;
        }
        let stages = At::Key(&at, "stages");
        if ms.present.is_some() != frame.stages.present.is_some() {
            return Err(format!(
                "{} and {} are not both null or both present",
                At::Key(&stage_ms, "present"),
                At::Key(&stages, "present")
            ));
        }
        for stage in Stage::ALL {
            if let Some(sections) = frame.stages.get(stage) {
                let path = At::Key(&stages, stage.key());
                check_sections(&path, sections)?;
                let pairs = sections
                    .allocations
                    .iter()
                    .map(|a| (a.kind.as_str(), a.pool));
                if let Some(j) = kind_pools.first_repeat(pairs) {
                    let a = &sections.allocations[j];
                    return Err(format!(
                        "{} repeats kind {:?} in pool {}: one entry per kind and pool",
                        At::Index(&At::Key(&path, "allocations"), j),
                        a.kind,
                        serde_json::to_string(&a.pool).unwrap_or_default()
                    ));
                }
            }
        }
        let live = &frame.live_memory;
        let live_at = At::Key(&at, "live_memory");
        for (pool, name) in [
            (&live.heap, "heap"),
            (&live.gpu, "gpu"),
            (&live.tile_cache, "tile_cache"),
        ] {
            let pool_at = At::Key(&live_at, name);
            let sum: u128 = pool.by_kind.iter().map(|k| u128::from(k.bytes)).sum();
            if sum != u128::from(pool.bytes) {
                return Err(format!(
                    "{} is {}, not the sum of its by_kind bytes ({sum})",
                    At::Key(&pool_at, "bytes"),
                    pool.bytes
                ));
            }
            if let Some(j) = kinds.first_repeat(pool.by_kind.iter().map(|k| k.kind.as_str())) {
                return Err(format!(
                    "{} repeats kind {:?}: one entry for each type",
                    At::Index(&At::Key(&pool_at, "by_kind"), j),
                    pool.by_kind[j].kind
                ));
            }
        }
    }
    Ok(())
}

/// A path into the file, `frames[3].stages.reduce.scopes[0].ms` say, built on the stack as the check descends and
/// formatted only when a check fails, so checking a valid file allocates no path.
#[derive(Clone, Copy)]
enum At<'a> {
    /// The file.
    Root,
    /// A key of the object at the parent path.
    Key(&'a At<'a>, &'static str),
    /// An index into the array at the parent path.
    Index(&'a At<'a>, usize),
}

impl fmt::Display for At<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            At::Root => Ok(()),
            At::Key(At::Root, key) => f.write_str(key),
            At::Key(parent, key) => write!(f, "{parent}.{key}"),
            At::Index(parent, i) => write!(f, "{parent}[{i}]"),
        }
    }
}

/// The keys seen so far in one list, to find a repeat (dd_telemetry_and_tiers §5: one entry per type, or per kind and
/// pool). JSON Schema can't express uniqueness by a key, so only the reader checks it.
struct Seen<K>(HashSet<K>);

impl<K> Default for Seen<K> {
    fn default() -> Self {
        Seen(HashSet::new())
    }
}

impl<K: Eq + Hash> Seen<K> {
    /// The index of the first key in `keys` that repeats an earlier one, if any.
    fn first_repeat(&mut self, keys: impl IntoIterator<Item = K>) -> Option<usize> {
        self.0.clear();
        keys.into_iter()
            .enumerate()
            .find_map(|(j, key)| (!self.0.insert(key)).then_some(j))
    }
}

fn check_sections(path: &At, sections: &StageSections) -> Result<(), String> {
    fn check_scopes(path: &At, scopes: &[Scope]) -> Result<(), String> {
        for (i, scope) in scopes.iter().enumerate() {
            let at = At::Index(path, i);
            non_negative(&At::Key(&at, "start_ms"), scope.start_ms)?;
            non_negative(&At::Key(&at, "ms"), scope.ms)?;
            check_scopes(&At::Key(&at, "children"), &scope.children)?;
        }
        Ok(())
    }
    check_scopes(&At::Key(path, "scopes"), &sections.scopes)?;
    let passes = At::Key(path, "gpu_passes");
    for (i, pass) in sections.gpu_passes.iter().enumerate() {
        let at = At::Index(&passes, i);
        non_negative(&At::Key(&at, "start_ms"), pass.start_ms)?;
        non_negative(&At::Key(&at, "ms"), pass.ms)?;
    }
    let events = At::Key(path, "events");
    for (i, event) in sections.events.iter().enumerate() {
        non_negative(&At::Key(&At::Index(&events, i), "at_ms"), event.at_ms)?;
    }
    Ok(())
}

fn non_negative(path: &At, value: f64) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!("{path} is {value}, not a finite number >= 0"))
    }
}
