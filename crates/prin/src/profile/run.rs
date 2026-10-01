//! The headless run: the scenario registry, the frame loop and the JSON Lines write (telemetry §5, R-286).
//!
//! A scenario is fixed and deterministic (render_gui_spec § "Profiler"): the same name and frame count give the same
//! frame count and the same scope and event sequence. M0 registers `synthetic_frames` (R-113); `deep_zoom_03` is
//! defined and registered in M5.
//!
//! No M0 scenario does GPU work, so no run opens a GPU adapter, and the header is the no-GPU form: `backend.api`
//! "none" and the GPU's fields null (telemetry §5, R-308). A run never opens an adapter only to fill the header: the
//! header probe, [`session_header`], takes the adapter the run already opened, if any.

use std::fs::File;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use engine::contract::canonical;
use engine::contract::profile::{
    self, Api, Backend, Build, Device, Event, FrameRecord, LiveMemory, PoolLive, SchemaId, Scope,
    Session, SessionHeader, StageMs, StageSections, Stages, Trace,
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
    /// Runs `frames` frames. `gpu` is the only way the run can reach a GPU adapter; a scenario with no GPU work never
    /// asks it.
    run: fn(frames: u32, gpu: &mut AdapterRequest<'_>) -> Result<Run, String>,
}

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

/// What a run produced: its frames, and the adapter it opened, if any.
pub(crate) struct Run {
    /// One record per frame, in order.
    pub(crate) frames: Vec<FrameRecord>,
    /// The GPU adapter the run opened for its own work; `None` when it did no GPU work.
    pub(crate) adapter: Option<OpenAdapter>,
}

/// `prin profile --scenario NAME --frames N --json PATH`: runs the scenario and writes its trace.
pub(crate) fn main(name: &str, frames: u32, path: &Path) -> Result<ExitCode, String> {
    let scenario = find(name).ok_or_else(|| {
        let names: Vec<&str> = SCENARIOS.iter().map(|s| s.name).collect();
        format!(
            "prin profile: {name:?} is not a registered scenario; registered: {}",
            names.join(", ")
        )
    })?;
    let run = (scenario.run)(frames, &mut no_gpu_api)?;
    let trace = trace_of(scenario.name, frames, run)?;
    let file = File::create(path)
        .map_err(|e| format!("prin profile: cannot create {}: {e}", path.display()))?;
    profile::write(&trace, file)
        .map_err(|e| format!("prin profile: cannot write {}: {e}", path.display()))?;
    Ok(ExitCode::SUCCESS)
}

/// The trace of a run: the header, the frames and the summary line. The summaries are `null` until the task closing
/// REQ-TOOL-100 defines them (telemetry §5).
pub(crate) fn trace_of(scenario: &str, frames: u32, run: Run) -> Result<Trace, String> {
    let header = session_header(run.adapter.as_ref(), config(scenario, frames)?)?;
    Ok(Trace {
        schema: SchemaId::V1,
        header,
        frames: run.frames,
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    })
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
fn synthetic_frames(frames: u32, _gpu: &mut AdapterRequest<'_>) -> Result<Run, String> {
    let records = (0..u64::from(frames)).map(synthetic_frame).collect();
    Ok(Run {
        frames: records,
        adapter: None,
    })
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

    /// A scenario that asks for an adapter, as one doing GPU work would; the control's run.
    #[cfg(feature = "controls")]
    fn asks_for_an_adapter(frames: u32, gpu: &mut AdapterRequest<'_>) -> Result<Run, String> {
        let adapter = gpu().ok();
        Ok(Run {
            frames: (0..u64::from(frames)).map(synthetic_frame).collect(),
            adapter,
        })
    }

    /// Runs `run` for three frames against a spy: it asks for no GPU adapter, and the header it gets is the no-GPU
    /// form (R-308).
    fn check_requests_no_adapter(run: fn(u32, &mut AdapterRequest<'_>) -> Result<Run, String>) {
        // A spy for the GPU: it counts the requests, and opens nothing.
        let mut requests = 0u32;
        let done = {
            let mut spy = || -> Result<OpenAdapter, String> {
                requests += 1;
                Err("the spy opens no adapter".to_owned())
            };
            run(3, &mut spy).expect("the run fails")
        };
        assert_eq!(
            requests, 0,
            "the run requested a GPU adapter {requests} time(s)"
        );
        let trace = trace_of("synthetic_frames", 3, done).expect("no trace");
        assert_eq!(trace.header.backend.api, Api::None);
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
