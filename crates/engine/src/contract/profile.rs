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
use std::io::{self, BufRead, Write};

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

/// One profiler trace: the session header, the frame records, then the precomputed summaries (telemetry §5).
///
/// The file is JSON Lines (R-286), which [`write`] writes and [`read`] reads: the header line
/// `{"schema", "header"}`, one frame record per line, then the summary line `{"leak_flags", "hot_paths"}`. A session
/// that ended before its summary line, a crash or a session still running, is [`Session::Incomplete`] (R-298). This
/// type's own serde form, one object with the five keys (and `session` when incomplete, `dropped_bytes` when not 0),
/// is not the file.
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
    /// Whether the file has its summary line (R-298). Not a key of the file: the summary line's presence is. An
    /// incomplete session's `leak_flags` and `hot_paths` are `None`, and [`Trace::leak_flags`] and
    /// [`Trace::hot_paths`] give the reason.
    #[serde(default, skip_serializing_if = "Session::is_complete")]
    pub session: Session,
    /// The bytes of a last line cut off before its newline, not one complete JSON value, which [`read`] dropped
    /// (R-299): the part of a line the session was writing when it stopped. 0 when nothing was dropped. Not a key of
    /// the file; a trace that dropped bytes is [`Session::Incomplete`].
    #[serde(default, skip_serializing_if = "is_zero")]
    pub dropped_bytes: u64,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

/// Whether a session ended with its summary line (R-298, telemetry §5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Session {
    /// The file ends with its summary line.
    #[default]
    Complete,
    /// The file ends before its summary line: the session crashed or is still running. Its frames stand; its
    /// summaries are absent.
    Incomplete,
}

impl Session {
    /// `true` for [`Session::Complete`].
    pub fn is_complete(&self) -> bool {
        *self == Session::Complete
    }
}

/// Why a trace has no leak flags or hot-path summaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Absent {
    /// The summary line holds `null`: they were not precomputed.
    NotPrecomputed,
    /// There is no summary line: the session is incomplete (R-298).
    SessionIncomplete,
}

impl fmt::Display for Absent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Absent::NotPrecomputed => "not precomputed",
            Absent::SessionIncomplete => "session incomplete",
        })
    }
}

impl Trace {
    /// The precomputed leak flags, or why there are none: "session incomplete" when the file has no summary line
    /// (R-298).
    pub fn leak_flags(&self) -> Result<&[Summary], Absent> {
        self.summary(&self.leak_flags)
    }

    /// The precomputed hot-path summaries, or why there are none: "session incomplete" when the file has no summary
    /// line (R-298).
    pub fn hot_paths(&self) -> Result<&[Summary], Absent> {
        self.summary(&self.hot_paths)
    }

    fn summary<'a>(&self, field: &'a Option<Vec<Summary>>) -> Result<&'a [Summary], Absent> {
        match (self.session, field) {
            (Session::Incomplete, _) => Err(Absent::SessionIncomplete),
            (Session::Complete, Some(entries)) => Ok(entries),
            (Session::Complete, None) => Err(Absent::NotPrecomputed),
        }
    }
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

/// The header line, as written.
#[derive(Serialize)]
struct HeaderLineOut<'a> {
    schema: SchemaId,
    header: &'a SessionHeader,
}

/// The header line, as read: exactly `schema` and `header`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeaderLine {
    schema: SchemaId,
    header: SessionHeader,
}

/// The summary line, as written.
#[derive(Serialize)]
struct SummaryLineOut<'a> {
    leak_flags: &'a Option<Vec<Summary>>,
    hot_paths: &'a Option<Vec<Summary>>,
}

/// The summary line, as read: exactly `leak_flags` and `hot_paths`, each required.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SummaryLine {
    #[serde(deserialize_with = "nullable")]
    leak_flags: Option<Vec<Summary>>,
    #[serde(deserialize_with = "nullable")]
    hot_paths: Option<Vec<Summary>>,
}

/// Writes `trace` as schema v1: JSON Lines (R-286), the header line, one compact frame record per line, then the
/// summary line, never pretty-printed; pretty-printing is on demand, `prin profile show --pretty` or `jq` (telemetry §5). A value outside its range (a negative ms,
/// NaN or an infinity), a frame with one `present` null and the other not, a pool whose `bytes` is not the sum of its
/// `by_kind` bytes, or two entries for one type in a pool's `by_kind` or for one kind and pool in a stage's
/// `allocations` is an error, and nothing is written. An incomplete session (R-298) is written without its summary
/// line, as a session that ended before it leaves the file, and its `leak_flags` and `hot_paths` must be `None`; the
/// bytes a reader dropped (R-299) are gone, so they are not written, and a complete trace must have dropped none.
///
/// The writer is buffered here and flushed before `write` returns, so a plain `File` costs no more than a `BufWriter`.
pub fn write<W: io::Write>(trace: &Trace, writer: W) -> Result<(), serde_json::Error> {
    check_ranges(trace).map_err(<serde_json::Error as serde::ser::Error>::custom)?;
    let mut writer = io::BufWriter::new(writer);
    write_lines(trace, &mut writer)?;
    writer.flush().map_err(serde_json::Error::io)
}

/// The lines of a checked trace, each compact and ended by a newline, straight into `writer`.
pub(crate) fn write_lines<W: io::Write>(
    trace: &Trace,
    mut writer: W,
) -> Result<(), serde_json::Error> {
    let header = HeaderLineOut {
        schema: trace.schema,
        header: &trace.header,
    };
    write_line(&mut writer, &header)?;
    for frame in &trace.frames {
        write_line(&mut writer, frame)?;
    }
    if trace.session == Session::Incomplete {
        return Ok(());
    }
    let summary = SummaryLineOut {
        leak_flags: &trace.leak_flags,
        hot_paths: &trace.hot_paths,
    };
    write_line(&mut writer, &summary)
}

fn write_line<W: io::Write, T: Serialize>(
    writer: &mut W,
    line: &T,
) -> Result<(), serde_json::Error> {
    serde_json::to_writer(&mut *writer, line)?;
    writer.write_all(b"\n").map_err(serde_json::Error::io)
}

/// Reads a schema v1 file, line by line. A line that is not the object its place calls for — the header line first,
/// a frame record on each line after it, the summary line last — is an error, and so are a blank line, a key outside
/// v1 (a scope beside the five stages, say), a missing key, even one whose
/// value may be `null`, a value outside its range, a frame with one `present` null and the other not, a pool whose
/// `bytes` is not the sum of its `by_kind` bytes, and two entries for one type in a pool's `by_kind` or for one kind and
/// pool in a stage's `allocations`, so each line `read` accepts validates against [`SCHEMA_V1`]'s definition for its
/// place. An error names the line, from 1.
///
/// A file that ends before its summary line, its last line a frame record or the header line, is a session that
/// crashed or is still running (R-298). It reads: the header and frames are returned, the trace is
/// [`Session::Incomplete`], and [`Trace::leak_flags`] and [`Trace::hot_paths`] report "session incomplete".
///
/// A last line with no newline after it that is not one complete JSON value was cut off as the session stopped
/// (R-299). It is dropped: the lines before it are the trace, the line before it a frame record or the header line;
/// the trace is [`Session::Incomplete`], and [`Trace::dropped_bytes`] states how many bytes were dropped. A last line
/// with no newline that is complete JSON is read as any last line is, and a malformed line that ends in a newline is
/// an error. A file whose only line is cut off has no header line, and is an error that states the bytes.
///
/// The reader is buffered here, so a plain `File` costs no more than a `BufReader`.
pub fn read<R: io::Read>(reader: R) -> Result<Trace, serde_json::Error> {
    read_lines(io::BufReader::new(reader))
}

/// Reads the lines from `reader`. Each line after the header is held until the next arrives: a line with one after it
/// is a frame record, and the last is the summary line or, in an incomplete session, a frame record (R-298).
pub(crate) fn read_lines<R: BufRead>(mut reader: R) -> Result<Trace, serde_json::Error> {
    let mut line = Vec::new();
    let mut held = Vec::new();
    if !next_line(&mut reader, &mut line)? {
        return Err(de_error("the file is empty: line 1 is not the header line"));
    }
    if cut_off(&line) {
        return Err(de_error(format!(
            "line 1 is cut off: {} bytes with no newline that are not JSON, so the file has no header line",
            line.len()
        )));
    }
    let head: HeaderLine = parse(&line, 1, "the header line")?;
    check_header(&At::Line(1), &head.header).map_err(de_error)?;
    let mut trace = Trace {
        schema: head.schema,
        header: head.header,
        frames: Vec::new(),
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    };
    if !next_line(&mut reader, &mut held)? {
        // The header line alone: a session that recorded no frame and ended before its summary line (R-298).
        trace.session = Session::Incomplete;
        return Ok(trace);
    }
    let mut number = 2;
    while next_line(&mut reader, &mut line)? {
        let frame: FrameRecord = parse(&held, number, "a frame record")?;
        // The sets borrow this frame's kinds, so they are the frame's own; reading allocates per frame regardless.
        check_frame(&At::Line(number), &frame, &mut Seens::default()).map_err(de_error)?;
        trace.frames.push(frame);
        std::mem::swap(&mut line, &mut held);
        number += 1;
    }
    if cut_off(&held) {
        // The part of a line the session was writing when it stopped (R-299): dropped, the line before it the last.
        trace.dropped_bytes = held.len() as u64;
        trace.session = Session::Incomplete;
        return Ok(trace);
    }
    // The two are told apart by their keys, which no summary line shares with a frame record.
    match serde_json::from_slice::<SummaryLine>(&held) {
        Ok(summary) => {
            trace.leak_flags = summary.leak_flags;
            trace.hot_paths = summary.hot_paths;
        }
        Err(as_summary) => {
            let frame: FrameRecord = serde_json::from_slice(&held).map_err(|as_frame| {
                de_error(format!(
                    "line {number}, the last: neither the summary line ({as_summary}) nor, for a session that \
                     ended before its summary line, a frame record ({as_frame})"
                ))
            })?;
            check_frame(&At::Line(number), &frame, &mut Seens::default()).map_err(de_error)?;
            trace.frames.push(frame);
            trace.session = Session::Incomplete;
        }
    }
    Ok(trace)
}

/// Reads the next line into `line`, its newline kept, replacing what it held; `false` at the end of the file. The
/// bytes are read as they are, so a line cut inside a character is still a line; each parse checks its UTF-8.
fn next_line<R: BufRead>(reader: &mut R, line: &mut Vec<u8>) -> Result<bool, serde_json::Error> {
    line.clear();
    let n = reader
        .read_until(b'\n', line)
        .map_err(serde_json::Error::io)?;
    Ok(n > 0)
}

/// Whether `line` was cut off as its session stopped (R-299): it has no newline after it, so it is the file's last,
/// and it is not one complete JSON value. A compact JSON object cut anywhere before its closing brace never is.
fn cut_off(line: &[u8]) -> bool {
    line.last() != Some(&b'\n') && serde_json::from_slice::<serde_json::Value>(line).is_err()
}

/// Parses line `number` as `place`, the object its place calls for; a blank line is not one.
fn parse<'a, T: Deserialize<'a>>(
    line: &'a [u8],
    number: usize,
    place: &str,
) -> Result<T, serde_json::Error> {
    serde_json::from_slice(line).map_err(|e| de_error(format!("line {number}, {place}: {e}")))
}

fn de_error(message: impl fmt::Display) -> serde_json::Error {
    <serde_json::Error as serde::de::Error>::custom(message)
}

/// The rules of dd_telemetry_and_tiers §5's definition that the Rust types don't already hold: every number finite,
/// and ≥ 0 except `playhead_dt`; `stage_ms.present` and `stages.present` null together; each pool's `bytes` the sum
/// of its `by_kind` bytes; one `by_kind` entry per type in a pool, and one `allocations` entry per kind and pool in a
/// stage. The integers' widths are the types'. A path names the line the value is written on: the header on line 1,
/// frame `i` on line `i + 2`.
fn check_ranges(trace: &Trace) -> Result<(), String> {
    check_header(&At::Line(1), &trace.header)?;
    if trace.session == Session::Incomplete
        && (trace.leak_flags.is_some() || trace.hot_paths.is_some())
    {
        return Err("an incomplete session has no summary line, so its leak_flags and hot_paths must be null".into());
    }
    if trace.session == Session::Complete && trace.dropped_bytes != 0 {
        return Err(
            "a trace whose reader dropped a cut-off last line is an incomplete session".into(),
        );
    }
    let mut seen = Seens::default();
    for (i, frame) in trace.frames.iter().enumerate() {
        check_frame(&At::Line(i + 2), frame, &mut seen)?;
    }
    Ok(())
}

/// One set of each for the whole trace, cleared for each list, so its capacity is allocated once, not per frame.
#[derive(Default)]
struct Seens<'a> {
    kinds: Seen<&'a str>,
    kind_pools: Seen<(&'a str, Pool)>,
}

fn check_header(line: &At, header: &SessionHeader) -> Result<(), String> {
    let at = At::Key(line, "header");
    if let Some(rate) = header.precision.f64_rate {
        let precision = At::Key(&at, "precision");
        non_negative(&At::Key(&precision, "f64_rate"), rate)?;
    }
    if let Some(display) = &header.display {
        let at = At::Key(&at, "display");
        non_negative(&At::Key(&at, "refresh_hz"), display.refresh_hz)?;
        non_negative(&At::Key(&at, "dpi_scale"), display.dpi_scale)?;
    }
    Ok(())
}

fn check_frame<'a>(at: &At, frame: &'a FrameRecord, seen: &mut Seens<'a>) -> Result<(), String> {
    non_negative(&At::Key(at, "frame_ms"), frame.frame_ms)?;
    if !frame.playhead_dt.is_finite() {
        return Err(format!(
            "{} is {}, not a finite number",
            At::Key(at, "playhead_dt"),
            frame.playhead_dt
        ));
    }
    non_negative(&At::Key(at, "camera_delta"), frame.camera_delta)?;
    let ms = &frame.stage_ms;
    let stage_ms = At::Key(at, "stage_ms");
    let values = [ms.integrate, ms.reduce, ms.colour, ms.upload];
    for (stage, value) in Stage::ALL.iter().zip(values) {
        non_negative(&At::Key(&stage_ms, stage.key()), value)?;
    }
    if let Some(present) = ms.present {
        non_negative(&At::Key(&stage_ms, "present"), present)?;
    }
    let stages = At::Key(at, "stages");
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
            if let Some(j) = seen.kind_pools.first_repeat(pairs) {
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
    let live_at = At::Key(at, "live_memory");
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
        if let Some(j) = seen
            .kinds
            .first_repeat(pool.by_kind.iter().map(|k| k.kind.as_str()))
        {
            return Err(format!(
                "{} repeats kind {:?}: one entry for each type",
                At::Index(&At::Key(&pool_at, "by_kind"), j),
                pool.by_kind[j].kind
            ));
        }
    }
    Ok(())
}

/// A path into the file, `line 5: stages.reduce.scopes[0].ms` say, built on the stack as the check descends and
/// formatted only when a check fails, so checking a valid file allocates no path.
#[derive(Clone, Copy)]
enum At<'a> {
    /// One line of the file, numbered from 1.
    Line(usize),
    /// A key of the object at the parent path.
    Key(&'a At<'a>, &'static str),
    /// An index into the array at the parent path.
    Index(&'a At<'a>, usize),
}

impl fmt::Display for At<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            At::Line(n) => write!(f, "line {n}"),
            At::Key(At::Line(n), key) => write!(f, "line {n}: {key}"),
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
