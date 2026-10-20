//! The render teardown, in a child process (TASK-M2-42, REQ-VAL-192). A render-crate test used to pass its assertions
//! and then die by SIGSEGV on lavapipe, after libtest's `test result: ok`: the pipeline cache's compile worker, a
//! thread left running, still held a clone of the device and every compiled pipeline when the cache was dropped, and
//! released them on its own thread while the process went on to exit. The cache's drop now joins the worker.
//!
//! `pipeline_teardown_exits_by_status` runs the sequence `pipeline_runtime`'s rig runs, `GpuHarness`, a `RenderLoop`
//! with its pipeline cache, a stain compiled by the worker, a frame, and their drop, as this test binary re-run on
//! that one test (as `scratch_guard_keeps_a_failing_tests_scratch` runs its child), four children at once as a CI
//! shard runs four tests. The check fails when a child dies by a signal, naming it; when a child fails, as it does
//! when a compiled pipeline outlives the render loop's drop; and when a child ran no test. Its negative controls (R-176)
//! show each.

#![cfg(unix)]

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Output};
use std::sync::Arc;
use std::time::Duration;

use ledger::gen::rust;
use render::assemble::{Kind, Node, Occupant, Stain, Tier};
use render::compositor::LAYER_FORMAT;
use render::frame_record::RenderLoop;
use render::pipeline_cache::FrameInputs;
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::spawn::Spawn;

/// Set in a child: what it runs ([`child`]).
const CHILD: &str = "PIPELINE_TEARDOWN_CHILD";

/// The test the children run.
const TEST: &str = "pipeline_teardown_exits_by_status";

const W: u32 = 4;
const H: u32 = 2;

const INPUTS: FrameInputs = FrameInputs {
    ensemble: 0,
    ensemble_spread: 0.25,
    masses: [1.0, 1.0, 1.0],
    dt_macro: 0.01,
    delta_0: 1e-6,
    n_renorm: 16,
    horizon_steps: 1000,
};

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// `ftle` into a flat colour, the pass-through combiner, and OUT.
fn stain() -> Stain {
    let colour = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5, 0.25, 0.125); }";
    Stain::new(vec![
        node(Kind::Source, Occupant::Field("ftle".into()), &[]),
        node(Kind::Colour, Occupant::Custom(colour.into()), &[Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ])
    .unwrap_or_else(|e| panic!("{e}"))
}

/// One stored `SimStateFTLE`'s size in bytes.
fn simstate_size() -> u64 {
    let s = ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == "SimStateFTLE")
        .expect("the FTLE stored variant");
    u64::from(rust::offsets(&s).1)
}

/// The teardown sequence: the harness, a render loop presenting into a target, the sim buffers set on it, a stain
/// compiled by the worker and drawn, then the render loop dropped, which must release every pipeline the worker
/// compiled before it returns, then the rest, in the order `pipeline_runtime`'s `Rig` drops them. With `leak`, the
/// control, a clone of the compiled pipeline is kept past the drop, as a worker still running keeps one.
fn sequence(leak: bool) {
    let h = GpuHarness::new().expect("a GPU device");
    let mut rl = RenderLoop::new(h.device(), h.queue(), LAYER_FORMAT, W, H)
        .unwrap_or_else(|e| panic!("{e}"));
    let target = h.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: LAYER_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let buffer = |label: &str, size: u64| {
        h.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    };
    let samples = u64::from(W * H);
    let simstate = buffer("simstate", samples * simstate_size());
    let word = buffer("word", samples * 16);
    rl.set_sim(simstate.clone(), Some(word.clone()));
    rl.cache()
        .request(&stain(), &[0, 1, 2, 3], Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    rl.cache().wait();
    let compiled = Arc::clone(rl.cache().current().expect("the stain compiled"));
    rl.frame(&INPUTS, &view);
    h.device()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll failed");
    let pipeline = Arc::downgrade(&compiled);
    let kept = leak.then_some(compiled);
    drop(rl);
    assert!(
        pipeline.upgrade().is_none(),
        "a compiled pipeline outlived the render loop's drop: another thread still holds it"
    );
    drop(kept);
    drop(h);
    drop((target, view, simstate, word));
}

/// Ends this process by SIGSEGV, and waits up to 10 s for it to land. The Rust runtime's own SIGSEGV handler, its
/// stack-overflow check, takes the first one sent and restores the default action, so a second one ends the process.
fn die_by_sigsegv() {
    let pid = std::process::id().to_string();
    for _ in 0..2 {
        let out = Command::new("kill")
            .args(["-s", "SEGV", &pid])
            .timed_output()
            .expect("kill ran");
        assert!(out.status.success(), "kill -s SEGV {pid} failed");
        std::thread::sleep(Duration::from_secs(1));
    }
    std::thread::sleep(Duration::from_secs(8));
    panic!("SIGSEGV never ended the child");
}

/// What a child runs, by `mode`: the teardown sequence, or, for the controls, the sequence keeping a pipeline past the
/// drop, a death by SIGSEGV or one by SIGABRT.
fn child(mode: &str) {
    match mode {
        "sequence" => sequence(false),
        "leak" => sequence(true),
        "segv" => die_by_sigsegv(),
        "abort" => std::process::abort(),
        other => panic!("no child mode {other}"),
    }
}

/// The signal's name where the check names it: SIGSEGV, the crash this check is for.
fn signal_name(signal: i32) -> &'static str {
    match signal {
        11 => " (SIGSEGV)",
        _ => "",
    }
}

/// What failed in a child's run, if anything: a signal, named; a failing status; or no test run.
fn verdict(out: &Output) -> Result<(), String> {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if let Some(signal) = out.status.signal() {
        return Err(format!(
            "the teardown child died by signal {signal}{}:\n{stdout}\n{stderr}",
            signal_name(signal)
        ));
    }
    if !out.status.success() {
        return Err(format!(
            "the teardown child failed, {}:\n{stdout}\n{stderr}",
            out.status
        ));
    }
    if !stdout.contains("test result: ok. 1 passed") {
        return Err(format!(
            "the teardown child ran no test:\n{stdout}\n{stderr}"
        ));
    }
    Ok(())
}

/// Runs `children` copies of `test` at once, each as this binary re-run on that one test with [`CHILD`] set to `mode`,
/// and panics with the first one's failure.
fn check(mode: &str, test: &str, children: usize) {
    let exe = std::env::current_exe().expect("the test binary");
    let outs: Vec<Output> = std::thread::scope(|scope| {
        let runs: Vec<_> = (0..children)
            .map(|_| {
                scope.spawn(|| {
                    Command::new(&exe)
                        .args([test, "--exact", "--nocapture", "--test-threads", "1"])
                        .env(CHILD, mode)
                        .timed_output()
                        .expect("the teardown child ran")
                })
            })
            .collect();
        runs.into_iter()
            .map(|r| r.join().expect("the child's thread"))
            .collect()
    });
    for out in &outs {
        if let Err(failure) = verdict(out) {
            panic!("{failure}");
        }
    }
}

#[test]
fn pipeline_teardown_exits_by_status() {
    match std::env::var(CHILD) {
        Ok(mode) => child(&mode),
        Err(_) => check("sequence", TEST, 4),
    }
}

negative_control!(
    pipeline_teardown_exits_by_status,
    "a child that dies by SIGSEGV fails the check, naming the signal",
    expected = "the teardown child died by signal 11 (SIGSEGV):",
    check("segv", TEST, 1)
);

#[cfg(feature = "controls")]
mod pipeline_teardown_other_signal {
    use super::*;
    negative_control!(
        pipeline_teardown_exits_by_status,
        "a child that dies by another signal, SIGABRT, fails the check, naming its number",
        expected = "the teardown child died by signal 6:",
        check("abort", TEST, 1)
    );
}

#[cfg(feature = "controls")]
mod pipeline_teardown_leak {
    use super::*;
    negative_control!(
        pipeline_teardown_exits_by_status,
        "a pipeline still held after the render loop's drop, as by a worker left running, fails the check",
        expected = "a compiled pipeline outlived the render loop's drop",
        check("leak", TEST, 1)
    );
}

#[cfg(feature = "controls")]
mod pipeline_teardown_no_test {
    use super::*;
    negative_control!(
        pipeline_teardown_exits_by_status,
        "a child whose filter matches no test fails the check",
        expected = "the teardown child ran no test",
        check("sequence", "pipeline_teardown_no_such_test", 1)
    );
}
