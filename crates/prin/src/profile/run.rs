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
    self, Event, FrameRecord, LiveMemory, PoolLive, SchemaId, Scope, Session, SessionHeader,
    StageMs, StageSections, Stages, Trace,
};
use engine::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, Links, Lock, Plane, Quality, SimConfig, Slice,
};
use engine::telemetry::session;
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

/// The session header (telemetry §5), from the probe `prin profile` shares with the benchmark runner
/// (`engine::telemetry::session`). The GPU's fields come only from `adapter`, the adapter the run already opened for its
/// own work; with none, the header is the no-GPU form (R-308). M0's `prin` can open none ([`OpenAdapter`] has no value).
pub(crate) fn session_header(
    adapter: Option<&OpenAdapter>,
    config: Map<String, Value>,
) -> Result<SessionHeader, String> {
    if let Some(adapter) = adapter {
        match *adapter {}
    }
    let host = session::host().map_err(|e| format!("prin profile: {e}"))?;
    let build = session::build(
        env!("PRIN_BUILD_COMMIT"),
        env!("PRIN_BUILD_PROFILE"),
        env!("PRIN_BUILD_FEATURES"),
    );
    session::header(None, &host, build, config)
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
        assert_eq!(trace.header.backend.api, profile::Api::None);
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
