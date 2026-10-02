//! The fixed benchmarks (telemetry §1.1: the work held constant, so devices compare), which `cargo xtask bench` runs
//! through the `bench` binary (TASK-M0-19). Each runs headless on the harness's GPU and returns the same profiler
//! schema v1 trace `prin profile` writes (telemetry §5), its session header filled once from the live system by the
//! probe `prin profile` shares (`engine::telemetry::session`), the GPU's fields from the adapter the bench opened for its
//! own work. Benchmarks run on the human's Mac, never on a hosted runner (R-186).

use std::path::Path;
use std::time::Instant;

use engine::contract::profile::{
    Api, FrameRecord, LiveMemory, PoolLive, SchemaId, Scope, Session, SessionHeader, StageMs,
    StageSections, Stages, Trace,
};
use engine::telemetry::session;
use serde_json::{Map, Value};

use crate::gpu::{first_mismatch, identity_fixture, GpuHarness};

/// A registered benchmark: its name, and the run that writes its trace from the workspace at the given root.
pub struct Bench {
    /// The name `cargo xtask bench` takes, and its baseline's directory under `fixtures/bench/`.
    pub name: &'static str,
    /// Runs it.
    pub run: fn(&Path) -> Result<Trace, String>,
}

/// The registered benchmarks.
pub const BENCHES: &[Bench] = &[Bench {
    name: "trivial-kernel",
    run: trivial_kernel,
}];

/// The registered benchmark called `name`.
pub fn find(name: &str) -> Option<&'static Bench> {
    BENCHES.iter().find(|b| b.name == name)
}

/// `trivial-kernel`'s frame count: each frame is one dispatch of the trivial kernel over the harness's 2^16-word
/// fixture. Applied per R-204 (RQ-201) — veto?
pub const TRIVIAL_KERNEL_FRAMES: u32 = 1000;

/// The WGSL entry point naga names for `kernel::toolchain::pack_unpack`.
pub const TRIVIAL_KERNEL_ENTRY: &str = "toolchain_pack_unpack";

/// The trivial kernel of TASK-M0-14 (`kernel::toolchain`), from the WGSL `cargo xtask build-kernel` wrote: compiled
/// once, then dispatched and read back once per frame, each frame's integrate stage timing the round trip. The first
/// frame's words are checked against the kernel run natively, so the bench times a correct kernel.
fn trivial_kernel(root: &Path) -> Result<Trace, String> {
    let path = root.join("target/spirv/kernel.wgsl");
    let wgsl = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "{}: {e}; run `cargo xtask build-kernel` first",
            path.display()
        )
    })?;
    let harness = GpuHarness::new().map_err(|e| e.to_string())?;
    eprintln!("{}", harness.adapter_info());
    let input = identity_fixture();
    let prepared = harness.prepare(&wgsl, TRIVIAL_KERNEL_ENTRY, &[&input]);
    let mut frames = Vec::with_capacity(TRIVIAL_KERNEL_FRAMES as usize);
    for index in 0..TRIVIAL_KERNEL_FRAMES {
        let start = Instant::now();
        let words = prepared.run();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if index == 0 {
            let native: Vec<u32> = (0..input.len() as u32)
                .zip(&input)
                .map(|(i, &w)| kernel::toolchain::pack_unpack_word(i, w))
                .collect();
            if let Some(i) = first_mismatch(&native, &words) {
                return Err(format!(
                    "the trivial kernel's word {i} differs from the native one"
                ));
            }
        }
        frames.push(frame(u64::from(index), TRIVIAL_KERNEL_ENTRY, ms));
    }
    let adapter = harness.session_adapter().map_err(|e| e.to_string())?;
    let mut config = Map::new();
    config.insert("bench".to_owned(), Value::from("trivial-kernel"));
    config.insert("entry".to_owned(), Value::from(TRIVIAL_KERNEL_ENTRY));
    config.insert("frames".to_owned(), Value::from(TRIVIAL_KERNEL_FRAMES));
    config.insert("words".to_owned(), Value::from(input.len() as u64));
    let header = session::header(Some(adapter), &session::host()?, build(root), config)?;
    check_complete(&header)?;
    Ok(Trace {
        schema: SchemaId::V1,
        header,
        frames,
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    })
}

/// This build's provenance: the commit `git` names at `root`, the cargo profile, and this crate's enabled features.
fn build(root: &Path) -> engine::contract::profile::Build {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let features = if cfg!(feature = "controls") {
        "controls"
    } else {
        ""
    };
    session::build(&session::git_commit(root), profile, features)
}

/// A bench frame: a batch render (no present stage, telemetry §5.5) whose integrate stage is one scope, `scope`, of
/// `ms`; nothing else is timed, counted or held.
pub fn frame(index: u64, scope: &str, ms: f64) -> FrameRecord {
    let empty = || StageSections {
        scopes: Vec::new(),
        gpu_passes: Vec::new(),
        allocations: Vec::new(),
        events: Vec::new(),
    };
    let pool = || PoolLive {
        bytes: 0,
        by_kind: Vec::new(),
    };
    let integrate = StageSections {
        scopes: vec![Scope {
            name: scope.to_owned(),
            start_ms: 0.0,
            ms,
            children: Vec::new(),
        }],
        ..empty()
    };
    FrameRecord {
        frame: index,
        frame_ms: ms,
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
            integrate: ms,
            reduce: 0.0,
            colour: 0.0,
            upload: 0.0,
            present: None,
        },
        stages: Stages {
            integrate,
            reduce: empty(),
            colour: empty(),
            upload: empty(),
            present: None,
        },
        live_memory: LiveMemory {
            heap: pool(),
            gpu: pool(),
            tile_cache: pool(),
        },
    }
}

/// A bench opened a GPU, so its header is complete: an API, and the GPU's model, driver, memory and precision filled.
/// The fields telemetry §5 makes null when not reported (`gpu_cores`, `f64_rate`) and a headless run's `display` may be
/// null.
pub fn check_complete(header: &SessionHeader) -> Result<(), String> {
    let missing: Vec<&str> = [
        ("backend.api", header.backend.api == Api::None),
        (
            "backend.driver",
            header.backend.driver.as_deref().is_none_or(str::is_empty),
        ),
        ("device.gpu", header.device.gpu.is_none()),
        ("device.memory", header.device.memory.is_none()),
        ("precision", header.precision.is_none()),
    ]
    .into_iter()
    .filter_map(|(field, absent)| absent.then_some(field))
    .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "the session header is incomplete: {} not filled",
            missing.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::telemetry::session::{Adapter, AdapterMemory, Host};

    fn header(adapter: Option<&Adapter>) -> SessionHeader {
        let host = Host {
            cpu: "cpu".to_owned(),
            cpu_cores_available: 1,
            cpu_cores_total: None,
            ram_bytes: Some(1 << 30),
        };
        let build = session::build("c", "release", "");
        session::header(adapter, &host, build, Map::new()).expect("no header")
    }

    /// `check` passes an opened adapter's header and refuses the no-GPU one, naming the fields.
    fn check_completeness(check: fn(&SessionHeader) -> Result<(), String>) {
        let adapter = Adapter {
            name: "gpu".to_owned(),
            api: Api::Metal,
            driver: "d".to_owned(),
            memory: AdapterMemory::Unified,
            f64: false,
        };
        assert_eq!(check(&header(Some(&adapter))), Ok(()));
        let refused = check(&header(None)).expect_err("a no-GPU header passed as complete");
        assert!(
            refused.contains("backend.api, backend.driver, device.gpu, device.memory, precision"),
            "the refusal names the wrong fields: {refused}"
        );
        let no_driver = Adapter {
            driver: String::new(),
            ..adapter
        };
        let refused = check(&header(Some(&no_driver))).expect_err("an empty driver passed");
        assert!(refused.ends_with("backend.driver not filled"), "{refused}");
    }

    #[test]
    fn bench_header_complete() {
        check_completeness(check_complete);
    }

    crate::negative_control!(
        bench_header_complete,
        "a check that passes every header must fail",
        expected = "a no-GPU header passed as complete",
        check_completeness(|_| Ok(()))
    );

    /// A bench frame is a batch render whose integrate stage holds the one timed scope, and the frame's time is it.
    fn check_frame(frame: fn(u64, &str, f64) -> FrameRecord) {
        let f = frame(7, "k", 2.5);
        let scope = &f.stages.integrate.scopes;
        assert!(
            f.frame == 7 && f.frame_ms == 2.5 && f.stage_ms.integrate == 2.5,
            "the frame's times are not the scope's"
        );
        assert!(
            scope.len() == 1 && scope[0].name == "k" && scope[0].ms == 2.5,
            "the integrate stage does not hold the timed scope"
        );
        assert!(f.stage_ms.present.is_none() && f.stages.present.is_none());
    }

    #[test]
    fn bench_frame_times_the_integrate_scope() {
        check_frame(frame);
    }

    crate::negative_control!(
        bench_frame_times_the_integrate_scope,
        "a frame whose scope is not the timed one must fail",
        expected = "does not hold the timed scope",
        check_frame(|i, _, ms| frame(i, "other", ms))
    );
}
