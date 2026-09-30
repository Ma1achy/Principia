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
use std::hint::black_box;
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
    run: fn(frames: u64, gpu: &mut AdapterRequest<'_>) -> Result<Run, String>,
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
pub(crate) fn main(name: &str, frames: u64, path: &Path) -> Result<ExitCode, String> {
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
pub(crate) fn trace_of(scenario: &str, frames: u64, run: Run) -> Result<Trace, String> {
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
/// RenderState}`, the two structs as the M0 contract skeleton holds them, each in the canonical serialisation.
fn config(scenario: &str, frames: u64) -> Result<Map<String, Value>, String> {
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
            cpu_cores: cpu_cores()?,
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

/// The CPU model, as the operating system names it; `unknown` where it names none.
fn cpu_model() -> String {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output();
        if let Ok(out) = out {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            if out.status.success() && !name.is_empty() {
                return name;
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo") {
            let name = info
                .lines()
                .find_map(|l| l.strip_prefix("model name")?.split_once(':'))
                .map(|(_, name)| name.trim().to_owned());
            if let Some(name) = name.filter(|n| !n.is_empty()) {
                return name;
            }
        }
    }
    "unknown".to_owned()
}

/// The CPU cores the process may run on, as the operating system reports them.
fn cpu_cores() -> Result<u32, String> {
    let n = std::thread::available_parallelism()
        .map_err(|e| format!("prin profile: the CPU core count is not reported: {e}"))?;
    Ok(u32::try_from(n.get()).unwrap_or(u32::MAX))
}

/// The four stages a headless run has; a batch render has no present stage (telemetry §5.5).
const BATCH_STAGES: [&str; 4] = ["integrate", "reduce", "colour", "upload"];

/// The scope each synthetic stage runs, and the child scope inside it.
const SCOPE: &str = "synthetic";
const CHILD: &str = "synthetic_step";
/// The event the integrate stage emits at the start of each frame.
const EVENT: &str = "synthetic_frame";

/// `synthetic_frames` (R-113, REQ-TOOL-006): no physics. Each frame runs the four batch stages; each stage one scope
/// with one child scope around a fixed, small integer computation, and the integrate stage one event, so every frame
/// has the same scopes and events in the same order. The times are measured, wall clock. Nothing is integrated,
/// reduced or uploaded, so the counts are 0; the camera and the playhead do not move; and no memory is tracked. A
/// headless run is a batch render: no present stage (telemetry §5.5). It does no GPU work, so it never asks `gpu`.
fn synthetic_frames(frames: u64, _gpu: &mut AdapterRequest<'_>) -> Result<Run, String> {
    let records = (0..frames).map(synthetic_frame).collect();
    Ok(Run {
        frames: records,
        adapter: None,
    })
}

fn ms_since(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

/// A fixed amount of integer work, so a scope's time is the time of something.
fn work(seed: u64) -> u64 {
    (0..1_000u64).fold(seed, |acc, k| {
        black_box(acc.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(k))
    })
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
        black_box(work(index));
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
    fn asks_for_an_adapter(frames: u64, gpu: &mut AdapterRequest<'_>) -> Result<Run, String> {
        let adapter = gpu().ok();
        Ok(Run {
            frames: (0..frames).map(synthetic_frame).collect(),
            adapter,
        })
    }

    /// Runs `run` for three frames against a spy: it asks for no GPU adapter, and the header it gets is the no-GPU
    /// form (R-308).
    fn check_requests_no_adapter(run: fn(u64, &mut AdapterRequest<'_>) -> Result<Run, String>) {
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
