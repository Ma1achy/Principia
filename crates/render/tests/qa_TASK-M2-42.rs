//! QA's tests for TASK-M2-42, written from the requirement it closes, not from the implementation:
//! - REQ-VAL-192: a render test's process must exit by status, never by a signal after its assertions pass; the found
//!   teardown sequence, run in a child process, exits by status, and the check reports a child that dies by a signal
//!   as a failure naming the signal.
//!
//! Two kinds of check, each with its negative control (R-176):
//! - in process: whatever the render side compiled is released by the time its owner's drop returns, whichever owner
//!   (a `PipelineCache`, or the `RenderLoop` holding one) and whatever the worker still has queued, with the GPU
//!   harness dropped first as `pipeline_runtime`'s `Rig` drops it. Nothing is then left for another thread to tear
//!   down while the process exits. The drop is also bounded: it must return.
//! - in children: teardown variants the found sequence does not cover (a compile still in flight, superseded requests
//!   queued, two caches on one device, the render loop's own cache, the normal drop order), eight children at once as
//!   the parallel suite runs them, each this binary re-run on one test; a child that dies by a signal fails the check
//!   and the signal is named.

#![cfg(unix)]

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Output};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use render::assemble::{Kind, Node, Occupant, Stain, Tier};
use render::compositor::LAYER_FORMAT;
use render::frame_record::RenderLoop;
use render::pipeline_cache::{CompiledStain, PipelineCache};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::spawn::Spawn;

/// Set in a child: the teardown variant it runs.
const CHILD: &str = "QA_TASK_M2_42_CHILD";

/// The test the children run.
const CHILD_TEST: &str = "qa_teardown_children_exit_by_status";

/// How long a drop may take before the check calls it hung: far above a lavapipe compile of these stains.
const DROP_BOUND: Duration = Duration::from_secs(60);

const KEYS: [u64; 4] = [0, 1, 2, 3];

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// `ftle` into the flat colour `(r, 0.25, 0.125)`, the pass-through combiner, and OUT: a distinct stain per `r`.
fn stain(r: f32) -> Stain {
    let colour =
        format!("fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>({r:?}, 0.25, 0.125); }}");
    Stain::new(vec![
        node(Kind::Source, Occupant::Field("ftle".into()), &[]),
        node(Kind::Colour, Occupant::Custom(colour), &[Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ])
    .unwrap_or_else(|e| panic!("{e}"))
}

/// Compiles `stain(r)` on `cache` and waits for it: a weak handle on the compiled pipeline.
fn compiled(cache: &mut PipelineCache, r: f32) -> Weak<CompiledStain> {
    cache
        .request(&stain(r), &KEYS, Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    cache.wait();
    Arc::downgrade(cache.current().expect("the stain compiled"))
}

/// Queues `n` more distinct stains on `cache` without waiting: the worker is still busy with them.
fn queue(cache: &mut PipelineCache, n: usize) {
    for i in 0..n {
        cache
            .request(&stain(0.01 * (i as f32 + 1.0)), &KEYS, Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"));
    }
}

/// Drops `owner` on another thread and waits up to [`DROP_BOUND`] for it to return.
fn bounded_drop<T: Send + 'static>(owner: T) {
    let dropping = std::thread::spawn(move || drop(owner));
    let start = Instant::now();
    while !dropping.is_finished() && start.elapsed() < DROP_BOUND {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        dropping.is_finished(),
        "the drop did not return within {DROP_BOUND:?}"
    );
    dropping.join().expect("the drop's thread");
}

// ── In process ────────────────────────────────────────────────────────────────────────────────────────────────────

/// A cache with a compiled stain and `queued` more requests still with its worker, the harness dropped first: once the
/// cache's drop returns, the compiled pipeline is gone. With `keep`, the control, the test keeps a clone past the drop,
/// as a thread still holding the cache's state would.
fn check_cache_releases(queued: usize, keep: bool) {
    let h = GpuHarness::new().expect("a GPU device");
    let mut cache = PipelineCache::new(h.device());
    let first = compiled(&mut cache, 0.5);
    queue(&mut cache, queued);
    let kept = if keep { first.upgrade() } else { None };
    drop(h);
    bounded_drop(cache);
    assert!(
        first.upgrade().is_none(),
        "a compiled pipeline outlived its cache's drop"
    );
    drop(kept);
}

#[test]
fn qa_teardown_cache_drop_releases_with_work_queued() {
    check_cache_releases(6, false);
}

negative_control!(
    qa_teardown_cache_drop_releases_with_work_queued,
    "a pipeline still held past the drop fails the check",
    expected = "a compiled pipeline outlived its cache's drop",
    check_cache_releases(6, true)
);

#[test]
fn qa_teardown_cache_drop_releases_when_idle() {
    check_cache_releases(0, false);
}

negative_control!(
    qa_teardown_cache_drop_releases_when_idle,
    "a pipeline still held past the drop fails the check",
    expected = "a compiled pipeline outlived its cache's drop",
    check_cache_releases(0, true)
);

/// The render loop, as `pipeline_runtime`'s rig holds it, the harness dropped before it, with a request still with
/// the worker: once the loop's drop returns, what its cache compiled is gone. With `keep`, the control, a clone is kept.
fn check_loop_releases(keep: bool) {
    let h = GpuHarness::new().expect("a GPU device");
    let mut rl = RenderLoop::new(h.device(), h.queue(), LAYER_FORMAT, 4, 2)
        .unwrap_or_else(|e| panic!("{e}"));
    let first = compiled(rl.cache(), 0.5);
    queue(rl.cache(), 3);
    let kept = if keep { first.upgrade() } else { None };
    drop(h);
    bounded_drop(rl);
    assert!(
        first.upgrade().is_none(),
        "a compiled pipeline outlived the render loop's drop"
    );
    drop(kept);
}

#[test]
fn qa_teardown_render_loop_drop_releases() {
    check_loop_releases(false);
}

negative_control!(
    qa_teardown_render_loop_drop_releases,
    "a pipeline still held past the render loop's drop fails the check",
    expected = "a compiled pipeline outlived the render loop's drop",
    check_loop_releases(true)
);

/// The bound itself can fire: a drop that never returns within it fails the check.
struct Stuck(Duration);

impl Drop for Stuck {
    fn drop(&mut self) {
        std::thread::sleep(self.0);
    }
}

#[test]
fn qa_teardown_bounded_drop_passes_a_quick_drop() {
    bounded_drop(Stuck(Duration::from_millis(1)));
}

#[cfg(feature = "controls")]
mod qa_bounded_drop_control {
    // The control's own bound: the stuck drop outlasts a shortened copy of the check.
    use super::*;

    fn bounded_drop_within(owner: Stuck, bound: Duration) {
        let dropping = std::thread::spawn(move || drop(owner));
        let start = Instant::now();
        while !dropping.is_finished() && start.elapsed() < bound {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            dropping.is_finished(),
            "the drop did not return within {bound:?}"
        );
    }

    negative_control!(
        qa_teardown_bounded_drop_passes_a_quick_drop,
        "a drop that outlasts the bound fails the check",
        expected = "the drop did not return within",
        bounded_drop_within(Stuck(Duration::from_secs(2)), Duration::from_millis(200))
    );
}

// ── In children ───────────────────────────────────────────────────────────────────────────────────────────────────

/// The teardown variants a child runs, each ending by returning to libtest, which exits.
fn variant(mode: &str) {
    match mode {
        // A compile still in flight when everything drops, the harness first.
        "inflight" => {
            let h = GpuHarness::new().expect("a GPU device");
            let mut cache = PipelineCache::new(h.device());
            queue(&mut cache, 1);
            drop(h);
            drop(cache);
        }
        // A compiled stain, then superseded requests and a prepare queued, the harness first.
        "queued" => {
            let h = GpuHarness::new().expect("a GPU device");
            let mut cache = PipelineCache::new(h.device());
            compiled(&mut cache, 0.5);
            queue(&mut cache, 5);
            cache
                .prepare(&stain(0.75), &KEYS, Tier::FULL)
                .unwrap_or_else(|e| panic!("{e}"));
            drop(h);
            drop(cache);
        }
        // Two caches on one device, each with a compile, dropped after the harness, the first cache first.
        "two_caches" => {
            let h = GpuHarness::new().expect("a GPU device");
            let mut a = PipelineCache::new(h.device());
            let mut b = PipelineCache::new(h.device());
            compiled(&mut a, 0.5);
            compiled(&mut b, 0.25);
            queue(&mut b, 2);
            drop(h);
            drop(a);
            drop(b);
        }
        // The render loop's cache, a request in flight, dropped after the harness, as the rig drops.
        "render_loop" => {
            let h = GpuHarness::new().expect("a GPU device");
            let mut rl = RenderLoop::new(h.device(), h.queue(), LAYER_FORMAT, 4, 2)
                .unwrap_or_else(|e| panic!("{e}"));
            compiled(rl.cache(), 0.5);
            queue(rl.cache(), 2);
            drop(h);
            drop(rl);
        }
        // The normal order: the cache before the harness.
        "settled" => {
            let h = GpuHarness::new().expect("a GPU device");
            let mut cache = PipelineCache::new(h.device());
            compiled(&mut cache, 0.5);
            drop(cache);
            drop(h);
        }
        // The control: a real fault, as the crash was, after the work is done.
        "fault" => {
            let h = GpuHarness::new().expect("a GPU device");
            let mut cache = PipelineCache::new(h.device());
            compiled(&mut cache, 0.5);
            // SAFETY: none; this writes to an unmapped address on purpose, so that the child dies by a signal. Rust's
            // handler finds no guard page there, restores the default action and returns, and the write faults again.
            unsafe { std::ptr::write_volatile(16usize as *mut u8, 1) };
            unreachable!("the fault did not end the child");
        }
        other => panic!("no child mode {other}"),
    }
}

/// The signal's number, and its name where it is one the crash could be.
fn signal_label(signal: i32) -> String {
    let name = match signal {
        11 => " (SIGSEGV)",
        6 => " (SIGABRT)",
        _ if signal == bus() => " (SIGBUS)",
        _ => "",
    };
    format!("{signal}{name}")
}

/// SIGBUS's number on this platform.
fn bus() -> i32 {
    if cfg!(target_os = "linux") {
        7
    } else {
        10
    }
}

/// What failed in a child's run: a signal, named; a failing status; or no test run.
fn failure(mode: &str, out: &Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if let Some(signal) = out.status.signal() {
        return Some(format!(
            "teardown child {mode} died by signal {}:\n{stdout}\n{stderr}",
            signal_label(signal)
        ));
    }
    if !out.status.success() {
        return Some(format!(
            "teardown child {mode} failed, {}:\n{stdout}\n{stderr}",
            out.status
        ));
    }
    if !stdout.contains("test result: ok. 1 passed") {
        return Some(format!(
            "teardown child {mode} ran no test:\n{stdout}\n{stderr}"
        ));
    }
    None
}

/// Runs every mode in `modes` as a child at once, each this binary re-run on [`CHILD_TEST`], each bounded by the
/// confirmed spawn timeout; panics naming every child that failed.
fn run_children(modes: &[&str]) {
    let exe = std::env::current_exe().expect("the test binary");
    let outs: Vec<(String, Output)> = std::thread::scope(|scope| {
        let runs: Vec<_> = modes
            .iter()
            .map(|&mode| {
                let exe = &exe;
                scope.spawn(move || {
                    let out = Command::new(exe)
                        .args([CHILD_TEST, "--exact", "--nocapture", "--test-threads", "1"])
                        .env(CHILD, mode)
                        .timed_output()
                        .unwrap_or_else(|e| panic!("teardown child {mode} did not finish: {e}"));
                    (mode.to_string(), out)
                })
            })
            .collect();
        runs.into_iter()
            .map(|r| r.join().expect("the child's thread"))
            .collect()
    });
    let failures: Vec<String> = outs
        .iter()
        .filter_map(|(mode, out)| failure(mode, out))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n----\n"));
}

#[test]
fn qa_teardown_children_exit_by_status() {
    match std::env::var(CHILD) {
        Ok(mode) => variant(&mode),
        Err(_) => run_children(&[
            "inflight",
            "queued",
            "two_caches",
            "render_loop",
            "settled",
            "inflight",
            "queued",
            "render_loop",
        ]),
    }
}

negative_control!(
    qa_teardown_children_exit_by_status,
    "a child that faults among children that exit cleanly fails the check, naming SIGSEGV",
    expected = "teardown child fault died by signal 11 (SIGSEGV)",
    run_children(&["settled", "fault", "inflight"])
);
