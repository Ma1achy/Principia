//! The headless run: the scenario registry, the frame loop and the JSON Lines write (telemetry §5, R-286).
//!
//! The run streams its trace (R-341): the header line first, flushed at once, then each frame record as the scenario
//! produces it, flushed at the 60th frame since the last flush or once 1 s has passed since it, whichever comes first,
//! then the summary line when the session ends. It keeps no frame record once the record is written, so its memory
//! does not grow with `--frames`; a run killed mid-session leaves an incomplete session, which the reader reports
//! "session incomplete" (R-298, R-299).
//!
//! A scenario is fixed and deterministic (render_gui_spec § "Profiler"): the same name and frame count give the same
//! frame count and the same scope and event sequence. M0 registers `synthetic_frames` (R-113); `deep_zoom_03` is
//! defined and registered in M5.
//!
//! No M0 scenario does GPU work, so no run opens a GPU adapter, and the header is the no-GPU form: `backend.api`
//! "none" and the GPU's fields null (telemetry §5, R-308). A run never opens an adapter only to fill the header: the
//! header probe, [`session_header`], takes the adapter the run already opened, if any.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use engine::contract::canonical;
use engine::contract::profile::{
    Api, Backend, Build, Device, Event, Flush, FrameRecord, LiveMemory, PoolLive, Scope,
    SessionHeader, StageMs, StageSections, Stages, Stream,
};
use engine::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, Links, Lock, Plane, Quality, SimConfig, Slice,
};
use serde_json::{Map, Value};

/// A registered scenario: a name and the run that produces its frames.
pub(crate) struct Scenario {
    /// The name `--scenario` takes.
    pub(crate) name: &'static str,
    /// Runs `frames` frames into `out`: [`Out::begin`] once, with the adapter it opened, if any, then [`Out::frame`]
    /// for each frame as it completes. `gpu` is the only way the run can reach a GPU adapter; a scenario with no GPU
    /// work never asks it.
    run: ScenarioRun,
}

/// A scenario's run: its frame count, its way to a GPU adapter, and where its frames go.
pub(crate) type ScenarioRun =
    fn(frames: u32, gpu: &mut AdapterRequest<'_>, out: &mut Out<'_>) -> Result<(), String>;

/// The registered scenarios.
pub(crate) const SCENARIOS: &[Scenario] = &[Scenario {
    name: "synthetic_frames",
    run: synthetic_frames,
}];

/// The registered scenario called `name`.
pub(crate) fn find(name: &str) -> Option<&'static Scenario> {
    SCENARIOS.iter().find(|s| s.name == name)
}

/// How a run reaches the GPU: calling it opens an adapter. A run calls it only when its scenario does GPU work, never
/// to fill the header (R-308).
pub(crate) type AdapterRequest<'a> = dyn FnMut() -> Result<OpenAdapter, String> + 'a;

/// A GPU adapter a run opened. M0's `prin` links no GPU API, so no adapter can be opened and this type has no value;
/// the first scenario that does GPU work (M5) gives it one, and [`session_header`] fills the GPU's fields from it.
pub(crate) enum OpenAdapter {}

/// `prin`'s GPU at M0: it links no GPU API, so a request is refused.
fn no_gpu_api() -> Result<OpenAdapter, String> {
    Err("prin links no GPU API at M0, so it cannot open a GPU adapter".to_owned())
}

/// R-341's flush: the frame lines are flushed at the 60th frame since the last flush, or once 1 s has passed since it,
/// whichever comes first. The header line is flushed as soon as it is written (R-341, applied per R-204, accepted by
/// R-346), and the summary line when the session ends.
pub(crate) const FLUSH: Flush = Flush {
    frames: 60,
    interval: Duration::from_secs(1),
};

/// Where a run's trace goes, as the run produces it (R-341): the header line when the scenario begins, each frame
/// record as it completes, and the summary line when the session ends. It holds the writer and the flush state, never
/// a frame record.
pub(crate) struct Out<'a> {
    scenario: &'a str,
    frames: u32,
    /// The trace's name in an error: its path.
    name: String,
    /// The clock the flush policy reads, the caller's (R-341).
    clock: &'a mut dyn FnMut() -> Instant,
    /// When the frame lines are flushed: [`FLUSH`].
    flush: Flush,
    state: OutState<'a>,
}

enum OutState<'a> {
    /// The header line is not written yet.
    Waiting(Box<dyn Write + 'a>),
    /// The header line is written; frames follow.
    Streaming(Stream<Box<dyn Write + 'a>>),
    /// A write failed, or the session ended.
    Done,
}

impl<'a> Out<'a> {
    /// A trace for `frames` frames of `scenario`, written to `writer` (buffered: [`Stream`] flushes it), the flushes
    /// timed by `clock`. `name` names the trace in an error.
    pub(crate) fn new(
        scenario: &'a str,
        frames: u32,
        writer: impl Write + 'a,
        name: String,
        clock: &'a mut dyn FnMut() -> Instant,
    ) -> Self {
        Out {
            scenario,
            frames,
            name,
            clock,
            flush: FLUSH,
            state: OutState::Waiting(Box::new(writer)),
        }
    }

    /// Writes the header line and flushes it. The GPU's fields come only from `adapter`, the one the run opened for
    /// its own work, if any (R-308). Called once, before any frame.
    pub(crate) fn begin(&mut self, adapter: Option<&OpenAdapter>) -> Result<(), String> {
        let OutState::Waiting(writer) = std::mem::replace(&mut self.state, OutState::Done) else {
            return Err(format!(
                "prin profile: {}: the header line is written once, before the frames",
                self.name
            ));
        };
        let header = session_header(adapter, config(self.scenario, self.frames)?)?;
        let stream = Stream::start(writer, &header, self.flush, (self.clock)())
            .map_err(|e| self.cannot_write(e))?;
        self.state = OutState::Streaming(stream);
        Ok(())
    }

    /// Writes `record` as the next frame line, flushing when R-341's policy says a flush is due. Nothing of the record
    /// is kept.
    pub(crate) fn frame(&mut self, record: &FrameRecord) -> Result<(), String> {
        let OutState::Streaming(stream) = &mut self.state else {
            return Err(format!(
                "prin profile: {}: a frame line comes after the header line",
                self.name
            ));
        };
        let now = (self.clock)();
        stream.frame(record, now).map_err(|e| self.cannot_write(e))
    }

    /// Ends the session: the summary line, last, and a flush. The summaries are `null` until the task closing
    /// REQ-TOOL-100 defines them (telemetry §5).
    pub(crate) fn finish(mut self) -> Result<(), String> {
        let OutState::Streaming(stream) = std::mem::replace(&mut self.state, OutState::Done) else {
            return Err(format!(
                "prin profile: {}: the session ended before its header line",
                self.name
            ));
        };
        stream
            .finish(&None, &None)
            .map(drop)
            .map_err(|e| self.cannot_write(e))
    }

    fn cannot_write(&self, e: serde_json::Error) -> String {
        format!("prin profile: cannot write {}: {e}", self.name)
    }
}

/// `prin profile --scenario NAME --frames N --json PATH`: runs the scenario, streaming its trace to PATH (R-341).
pub(crate) fn main(name: &str, frames: u32, path: &Path) -> Result<ExitCode, String> {
    let scenario = find(name).ok_or_else(|| {
        let names: Vec<&str> = SCENARIOS.iter().map(|s| s.name).collect();
        format!(
            "prin profile: {name:?} is not a registered scenario; registered: {}",
            names.join(", ")
        )
    })?;
    let file = File::create(path)
        .map_err(|e| format!("prin profile: cannot create {}: {e}", path.display()))?;
    let mut clock = Instant::now;
    let mut out = Out::new(
        scenario.name,
        frames,
        BufWriter::new(file),
        path.display().to_string(),
        &mut clock,
    );
    (scenario.run)(frames, &mut no_gpu_api, &mut out)?;
    out.finish()?;
    Ok(ExitCode::SUCCESS)
}

/// The run's full configuration (telemetry §5, R-309): `{"scenario": NAME, "frames": N, "sim": SimConfig, "render":
/// RenderState}`, the two structs as the M0 contract skeleton holds them, each in the canonical serialisation. `frames`
/// is a u32, so it is a JSON number (R-327, R-322).
fn config(scenario: &str, frames: u32) -> Result<Map<String, Value>, String> {
    let (sim, render) = skeleton();
    let mut config = Map::new();
    config.insert("scenario".to_owned(), Value::from(scenario));
    config.insert("frames".to_owned(), Value::from(frames));
    config.insert("sim".to_owned(), canonical_value(&sim)?);
    config.insert("render".to_owned(), canonical_value(&render)?);
    Ok(config)
}

/// `SimConfig` and `RenderState` at M0: each group is named and empty (TASK-M0-16), so a scenario sets nothing in
/// them.
pub(crate) fn skeleton() -> (SimConfig, RenderState) {
    let sim = SimConfig {
        chart: Chart {},
        plane: Plane {},
        slice: Slice {},
        lock: Lock {},
        links: Links {},
        integrator: Integrator {},
        horizon: Horizon {},
        collision: Collision {},
        quality: Quality {},
    };
    let render = RenderState {
        stain_graph: StainGraph {},
        overlays: Overlays {},
        palette: Palette {},
        playhead: Playhead {},
    };
    (sim, render)
}

/// A value's canonical text, read back as JSON; the header writes it back out canonically.
fn canonical_value<T: serde::Serialize>(value: &T) -> Result<Value, String> {
    let text = canonical::to_string(value).map_err(|e| format!("prin profile: config: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("prin profile: config: {e}"))
}

/// The session header (telemetry §5). The GPU's fields come only from `adapter`, the adapter the run already opened
/// for its own work; with none, the header is the no-GPU form (R-308): `backend.api` "none", and `backend.driver`,
/// `device.gpu`, `device.gpu_cores`, `device.memory` and `precision` null. The CPU is written as always. A headless
/// run has no display.
pub(crate) fn session_header(
    adapter: Option<&OpenAdapter>,
    config: Map<String, Value>,
) -> Result<SessionHeader, String> {
    if let Some(adapter) = adapter {
        match *adapter {}
    }
    Ok(SessionHeader {
        device: Device {
            gpu: None,
            cpu: cpu_model(),
            cpu_cores_available: cpu_cores_available()?,
            cpu_cores_total: cpu_cores_total(),
            gpu_cores: None,
            memory: None,
        },
        backend: Backend {
            api: Api::None,
            driver: None,
        },
        precision: None,
        build: build(),
        display: None,
        config,
    })
}

/// The build's provenance, stamped by `build.rs`: the commit hash, the cargo profile and the enabled features.
fn build() -> Build {
    let features = env!("PRIN_BUILD_FEATURES");
    Build {
        commit: env!("PRIN_BUILD_COMMIT").to_owned(),
        profile: env!("PRIN_BUILD_PROFILE").to_owned(),
        features: features
            .split(',')
            .filter(|f| !f.is_empty())
            .map(str::to_owned)
            .collect(),
    }
}

/// macOS's `sysctl`, by its full path: the header's CPU fields never depend on the run's PATH, which may lack
/// /usr/sbin (telemetry §5, R-329). Elsewhere the path doesn't exist, and the probes fall through to /proc/cpuinfo.
const SYSCTL: &str = "/usr/sbin/sysctl";

/// The CPU model, as the operating system names it: macOS's `sysctl machdep.cpu.brand_string`, or Linux's
/// `model name` in /proc/cpuinfo. Both are asked on every system, and a system without one gives nothing from it.
fn cpu_model() -> String {
    let sysctl = std::process::Command::new(SYSCTL)
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .ok()
        .map(|out| out.stdout);
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").ok();
    cpu_named(sysctl.as_deref(), cpuinfo.as_deref())
}

/// The CPU model from `sysctl`'s output, else from /proc/cpuinfo's first `model name`; `unknown` where neither names
/// one.
fn cpu_named(sysctl: Option<&[u8]>, cpuinfo: Option<&str>) -> String {
    let named = |name: &str| Some(name.trim().to_owned()).filter(|n| !n.is_empty());
    let brand = sysctl.and_then(|out| named(&String::from_utf8_lossy(out)));
    let model = || {
        cpuinfo?
            .lines()
            .find_map(|l| l.strip_prefix("model name")?.split_once(':'))
            .and_then(|(_, name)| named(name))
    };
    brand.or_else(model).unwrap_or_else(|| "unknown".to_owned())
}

/// The CPU cores this process may use, as `std::thread::available_parallelism` reports them (R-329).
fn cpu_cores_available() -> Result<u32, String> {
    let n = std::thread::available_parallelism()
        .map_err(|e| format!("prin profile: the CPU core count is not reported: {e}"))?;
    Ok(u32::try_from(n.get()).unwrap_or(u32::MAX))
}

/// The machine's own CPU core count (R-329), where the platform reports it cheaply: macOS's `sysctl hw.ncpu`, or the
/// `processor` entries in Linux's /proc/cpuinfo, the same two sources [`cpu_model`] asks. Both are asked on every
/// system; `None` where neither gives a count. No new dependency and no unsafe code: a platform these don't cover
/// writes `null` (telemetry §5).
fn cpu_cores_total() -> Option<u32> {
    let sysctl = std::process::Command::new(SYSCTL)
        .args(["-n", "hw.ncpu"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| out.stdout);
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").ok();
    cpu_total_from(sysctl.as_deref(), cpuinfo.as_deref())
}

/// The core count from `sysctl hw.ncpu`'s output, else the number of `processor` entries in /proc/cpuinfo; `None`
/// where neither gives a count of at least one that fits a u32.
fn cpu_total_from(sysctl: Option<&[u8]>, cpuinfo: Option<&str>) -> Option<u32> {
    let counted = |n: u32| Some(n).filter(|n| *n > 0);
    let ncpu = sysctl.and_then(|out| String::from_utf8_lossy(out).trim().parse::<u32>().ok());
    let processors = || {
        let n = cpuinfo?
            .lines()
            .filter(|l| {
                l.split_once(':')
                    .is_some_and(|(key, _)| key.trim() == "processor")
            })
            .count();
        u32::try_from(n).ok()
    };
    ncpu.and_then(counted)
        .or_else(|| processors().and_then(counted))
}

/// The four stages a headless run has; a batch render has no present stage (telemetry §5.5).
const BATCH_STAGES: [&str; 4] = ["integrate", "reduce", "colour", "upload"];

/// The scope each synthetic stage runs, and the child scope inside it.
const SCOPE: &str = "synthetic";
const CHILD: &str = "synthetic_step";
/// The event the integrate stage emits at the start of each frame.
const EVENT: &str = "synthetic_frame";

/// `synthetic_frames` (R-113, REQ-TOOL-006): no physics. Each frame runs the four batch stages; each stage one scope
/// with one child scope, and the integrate stage one event, so every frame has the same scopes and events in the same
/// order. The scopes do no work of their own: their times, measured wall clock, are the instrumentation's own cost,
/// which a profile carries in every scenario (telemetry §5.5: the overhead must be small enough to leave on). Nothing is integrated,
/// reduced or uploaded, so the counts are 0; the camera and the playhead do not move; and no memory is tracked. A
/// headless run is a batch render: no present stage (telemetry §5.5). It does no GPU work, so it never asks `gpu`.
/// Each frame goes to `out` as it completes, and is not kept (R-341).
fn synthetic_frames(
    frames: u32,
    _gpu: &mut AdapterRequest<'_>,
    out: &mut Out<'_>,
) -> Result<(), String> {
    out.begin(None)?;
    for index in 0..u64::from(frames) {
        out.frame(&synthetic_frame(index))?;
    }
    Ok(())
}

fn ms_since(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn synthetic_frame(index: u64) -> FrameRecord {
    let frame_start = Instant::now();
    let mut stage_ms = [0.0; 4];
    let mut sections: Vec<StageSections> = Vec::with_capacity(4);
    for (i, stage) in BATCH_STAGES.iter().enumerate() {
        let stage_start = Instant::now();
        let mut events = Vec::new();
        if *stage == "integrate" {
            events.push(Event {
                name: EVENT.to_owned(),
                at_ms: ms_since(frame_start),
                detail: None,
            });
        }
        let scope_start_ms = ms_since(frame_start);
        let scope_start = Instant::now();
        let child_start_ms = ms_since(frame_start);
        let child_start = Instant::now();
        let child = Scope {
            name: CHILD.to_owned(),
            start_ms: child_start_ms,
            ms: ms_since(child_start),
            children: Vec::new(),
        };
        let scope = Scope {
            name: SCOPE.to_owned(),
            start_ms: scope_start_ms,
            ms: ms_since(scope_start),
            children: vec![child],
        };
        sections.push(StageSections {
            scopes: vec![scope],
            gpu_passes: Vec::new(),
            allocations: Vec::new(),
            events,
        });
        stage_ms[i] = ms_since(stage_start);
    }
    let frame_ms = ms_since(frame_start);
    let mut sections = sections.into_iter();
    let mut next = || sections.next().expect("one section per batch stage");
    let empty = || PoolLive {
        bytes: 0,
        by_kind: Vec::new(),
    };
    FrameRecord {
        frame: index,
        frame_ms,
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
            integrate: stage_ms[0],
            reduce: stage_ms[1],
            colour: stage_ms[2],
            upload: stage_ms[3],
            present: None,
        },
        stages: Stages {
            integrate: next(),
            reduce: next(),
            colour: next(),
            upload: next(),
            present: None,
        },
        live_memory: LiveMemory {
            heap: empty(),
            gpu: empty(),
            tile_cache: empty(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `out`, flushing by `flush` in place of [`FLUSH`]: the controls' run.
    #[cfg(feature = "controls")]
    fn flushing(out: Out<'_>, flush: Flush) -> Out<'_> {
        Out { flush, ..out }
    }

    /// A scenario that asks for an adapter, as one doing GPU work would; the control's run.
    #[cfg(feature = "controls")]
    fn asks_for_an_adapter(
        frames: u32,
        gpu: &mut AdapterRequest<'_>,
        out: &mut Out<'_>,
    ) -> Result<(), String> {
        let adapter = gpu().ok();
        out.begin(adapter.as_ref())?;
        for index in 0..u64::from(frames) {
            out.frame(&synthetic_frame(index))?;
        }
        Ok(())
    }

    /// Runs `run` for three frames against a spy: it asks for no GPU adapter, and the header it writes is the no-GPU
    /// form (R-308).
    fn check_requests_no_adapter(run: ScenarioRun) {
        // A spy for the GPU: it counts the requests, and opens nothing.
        let mut requests = 0u32;
        let mut bytes = Vec::new();
        {
            let mut spy = || -> Result<OpenAdapter, String> {
                requests += 1;
                Err("the spy opens no adapter".to_owned())
            };
            let mut clock = Instant::now;
            let mut out = Out::new(
                "synthetic_frames",
                3,
                &mut bytes,
                "the test's trace".to_owned(),
                &mut clock,
            );
            run(3, &mut spy, &mut out).expect("the run fails");
            out.finish().expect("the run's trace does not finish");
        }
        assert_eq!(
            requests, 0,
            "the run requested a GPU adapter {requests} time(s)"
        );
        let trace = engine::contract::profile::read(bytes.as_slice()).expect("no trace");
        assert_eq!(trace.header.backend.api, Api::None);
        assert_eq!(trace.frames.len(), 3);
    }

    /// `ms_since` gives wall-clock milliseconds: a sleep of `ms` reads as at least `ms`, and well under a second more.
    fn check_milliseconds(ms_since: fn(Instant) -> f64, ms: u64) {
        let start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(ms));
        let got = ms_since(start);
        assert!(
            got >= ms as f64 && got < ms as f64 + 1000.0,
            "a {ms} ms sleep reads as {got} ms"
        );
    }

    #[test]
    fn profile_file_times_are_milliseconds() {
        check_milliseconds(ms_since, 5);
    }

    validation::negative_control!(
        profile_file_times_are_milliseconds,
        "seconds must fail the milliseconds check",
        expected = "ms sleep reads as",
        check_milliseconds(|start| start.elapsed().as_secs_f64(), 5)
    );

    /// M0's GPU refuses every request, saying why: `prin` links no GPU API.
    fn check_refuses(request: fn() -> Result<OpenAdapter, String>) {
        match request() {
            Err(why) => assert!(
                why.contains("links no GPU API"),
                "the refusal does not say prin links no GPU API: {why:?}"
            ),
            Ok(adapter) => match adapter {},
        }
    }

    #[test]
    fn profile_no_gpu_api_refuses_a_request() {
        check_refuses(no_gpu_api);
    }

    validation::negative_control!(
        profile_no_gpu_api_refuses_a_request,
        "a refusal that gives no reason must fail the check",
        expected = "does not say prin links no GPU API",
        check_refuses(|| Err(String::new()))
    );

    /// `sysctl`'s output, /proc/cpuinfo, and the CPU model they name.
    type CpuCase<'a> = (Option<&'a [u8]>, Option<&'a str>, &'a str);

    /// `named` reads the CPU model: `sysctl`'s output first, then /proc/cpuinfo's `model name`, else `unknown`.
    fn check_cpu_named(named: fn(Option<&[u8]>, Option<&str>) -> String) {
        let cpuinfo =
            "processor\t: 0\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Xeon(R) CPU\n";
        let cases: [CpuCase<'_>; 7] = [
            (Some(b"Apple M3 Pro\n"), None, "Apple M3 Pro"),
            (Some(b"Apple M3 Pro\n"), Some(cpuinfo), "Apple M3 Pro"),
            (Some(b""), Some(cpuinfo), "Intel(R) Xeon(R) CPU"),
            (None, Some(cpuinfo), "Intel(R) Xeon(R) CPU"),
            (Some(b"  \n"), Some("model name\t: \n"), "unknown"),
            (None, Some("processor\t: 0\n"), "unknown"),
            (None, None, "unknown"),
        ];
        for (sysctl, info, want) in cases {
            assert_eq!(
                named(sysctl, info),
                want,
                "the CPU is misnamed from {sysctl:?} and {info:?}"
            );
        }
    }

    #[test]
    fn profile_no_gpu_header_names_the_cpu() {
        check_cpu_named(cpu_named);
    }

    validation::negative_control!(
        profile_no_gpu_header_names_the_cpu,
        "a probe that names no CPU must fail the check",
        expected = "the CPU is misnamed",
        check_cpu_named(|_, _| "unknown".to_owned())
    );

    /// `sysctl hw.ncpu`'s output, /proc/cpuinfo, and the core count they give.
    type TotalCase<'a> = (Option<&'a [u8]>, Option<&'a str>, Option<u32>);

    /// `total` reads the machine's core count (R-329): `sysctl hw.ncpu` first, then /proc/cpuinfo's `processor`
    /// entries, else `None`.
    fn check_cpu_total(total: fn(Option<&[u8]>, Option<&str>) -> Option<u32>) {
        let cpuinfo = "processor\t: 0\nmodel name\t: X\n\nprocessor\t: 1\nmodel name\t: X\n";
        let cases: [TotalCase<'_>; 7] = [
            (Some(b"10\n"), None, Some(10)),
            (Some(b"10\n"), Some(cpuinfo), Some(10)),
            (Some(b""), Some(cpuinfo), Some(2)),
            (None, Some(cpuinfo), Some(2)),
            (Some(b"0\n"), Some("model name\t: X\n"), None),
            (Some(b"4294967296\n"), None, None),
            (None, None, None),
        ];
        for (sysctl, info, want) in cases {
            assert_eq!(
                total(sysctl, info),
                want,
                "the core total is wrong from {sysctl:?} and {info:?}"
            );
        }
    }

    #[test]
    fn profile_file_header_cpu_cores_total() {
        check_cpu_total(cpu_total_from);
    }

    validation::negative_control!(
        profile_file_header_cpu_cores_total,
        "a probe that never reports the total must fail the check",
        expected = "the core total is wrong",
        check_cpu_total(|_, _| None)
    );

    /// A writer that keeps, at each flush, the number of whole lines written by then.
    #[derive(Clone, Default)]
    struct Recorder(std::rc::Rc<std::cell::RefCell<(usize, Vec<usize>)>>);

    impl Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().0 += buf.iter().filter(|b| **b == b'\n').count();
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            let mut recorded = self.0.borrow_mut();
            let lines = recorded.0;
            recorded.1.push(lines);
            Ok(())
        }
    }

    /// Runs `synthetic_frames` for `frames` frames into `out`'s trace, by a clock that moves on `tick` at each
    /// reading; the whole lines written at each flush. `out` builds the run's [`Out`].
    fn run_flushes(
        frames: u32,
        tick: Duration,
        out: impl for<'a> Fn(Out<'a>) -> Out<'a>,
    ) -> Vec<usize> {
        let recorder = Recorder::default();
        let mut now = Instant::now();
        let mut clock = || {
            now += tick;
            now
        };
        let base = Out::new(
            "synthetic_frames",
            frames,
            recorder.clone(),
            "the test's trace".to_owned(),
            &mut clock,
        );
        let mut out = out(base);
        let scenario = find("synthetic_frames").expect("synthetic_frames is not registered");
        (scenario.run)(frames, &mut no_gpu_api, &mut out).expect("the run fails");
        out.finish().expect("the run's trace does not finish");
        let flushes = recorder.0.borrow().1.clone();
        flushes
    }

    /// The run flushes its frame lines at the 60th frame since the last flush (R-341): 150 frames 1 ms apart flush at
    /// lines 61 and 121, after the header's flush on line 1; the summary line, line 152, is flushed last.
    fn check_run_sixty(out: impl for<'a> Fn(Out<'a>) -> Out<'a>) {
        let got = run_flushes(150, Duration::from_millis(1), out);
        assert_eq!(got, [1, 61, 121, 152], "the run flushed at lines {got:?}");
    }

    #[test]
    fn profile_stream_flush_run_every_60_frames() {
        check_run_sixty(|out| out);
    }

    validation::negative_control!(
        profile_stream_flush_run_every_60_frames,
        "a run that flushes only at 61 frames must fail the check",
        expected = "the run flushed at lines",
        check_run_sixty(|out| flushing(
            out,
            Flush {
                frames: 61,
                ..FLUSH
            }
        ))
    );

    /// The run flushes its frame lines once 1 s has passed since the last flush (R-341): 10 frames 300 ms apart flush
    /// at the 4th and 8th frames (lines 5 and 9); the summary line, line 12, is flushed last.
    fn check_run_one_second(out: impl for<'a> Fn(Out<'a>) -> Out<'a>) {
        let got = run_flushes(10, Duration::from_millis(300), out);
        assert_eq!(got, [1, 5, 9, 12], "the run flushed at lines {got:?}");
    }

    #[test]
    fn profile_stream_flush_run_every_1_s() {
        check_run_one_second(|out| out);
    }

    validation::negative_control!(
        profile_stream_flush_run_every_1_s,
        "a run whose flushes ignore the clock must fail the check",
        expected = "the run flushed at lines",
        check_run_one_second(|out| flushing(
            out,
            Flush {
                interval: Duration::MAX,
                ..FLUSH
            }
        ))
    );

    #[test]
    fn profile_no_gpu_synthetic_run_requests_no_adapter() {
        let scenario = find("synthetic_frames").expect("synthetic_frames is not registered");
        check_requests_no_adapter(scenario.run);
    }

    validation::negative_control!(
        profile_no_gpu_synthetic_run_requests_no_adapter,
        "a run that asks for an adapter must fail the no-request check",
        expected = "the run requested a GPU adapter",
        check_requests_no_adapter(asks_for_an_adapter)
    );
}
