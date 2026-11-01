//! The skeleton's fallback (TASK-M8-86, R-627): a worker layout that fails before its first frame has returned gives way
//! to the main thread. `web_skeleton_fallback.mjs` runs the real `worker.js` on Node, with a stub `gui.js`; each control
//! runs it on the file with the fix undone.

use std::sync::atomic::{AtomicU32, Ordering};

use validation::negative_control;

static RUNS: AtomicU32 = AtomicU32::new(0);

/// Runs `scenario` on the skeleton's scripts with each `(file, from, to)` edit applied, panicking with what it reports.
fn run(scenario: &str, edits: &[(&str, &str, &str)]) {
    let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let skeleton = tests.join("../../web/skeleton");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "web-fallback-{}-{}",
        std::process::id(),
        RUNS.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("the scratch directory");
    for file in ["worker.js", "loader.js"] {
        let mut text = std::fs::read_to_string(skeleton.join(file)).expect("a skeleton script");
        for (_, from, to) in edits.iter().filter(|(name, ..)| *name == file) {
            assert!(
                text.contains(from),
                "the control's `{from}` is not in {file}"
            );
            text = text.replace(from, to);
        }
        std::fs::write(dir.join(file), text).expect("a scratch script");
    }
    let output = std::process::Command::new("node")
        .arg(tests.join("web_skeleton_fallback.mjs"))
        .arg(&dir)
        .arg(scenario)
        .output()
        .expect("node, which the skeleton's runner needs too");
    std::fs::remove_dir_all(&dir).expect("the scratch directory");
    assert!(
        output.status.success(),
        "{scenario}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `started` posted as the app is made, before any frame: the old order.
const STARTED_EARLY: (&str, &str, &str) = (
    "worker.js",
    "  early.splice(0).forEach(call);\n",
    "  early.splice(0).forEach(call);\n  self.postMessage({ type: \"started\" });\n",
);

#[test]
fn worker_started_after_first_frame() {
    run("worker_started_after_first_frame", &[]);
}

negative_control!(
    worker_started_after_first_frame,
    "started posted before any frame",
    expected = "started was posted before any frame ran",
    run("worker_started_after_first_frame", &[STARTED_EARLY])
);

#[test]
fn worker_first_frame_failure_is_not_started() {
    run("worker_first_frame_failure_is_not_started", &[]);
}

negative_control!(
    worker_first_frame_failure_is_not_started,
    "started posted before a first frame that fails",
    expected = "a first frame that failed was reported as started",
    run(
        "worker_first_frame_failure_is_not_started",
        &[STARTED_EARLY]
    )
);

#[test]
fn worker_later_frame_failure_follows_started() {
    run("worker_later_frame_failure_follows_started", &[]);
}

negative_control!(
    worker_later_frame_failure_follows_started,
    "started never posted",
    expected = "the second frame failed, and it posted: failed",
    run(
        "worker_later_frame_failure_follows_started",
        &[(
            "worker.js",
            "self.postMessage({ type: \"started\" });",
            "void 0;"
        )]
    )
);

#[test]
fn worker_start_failure_is_not_started() {
    run("worker_start_failure_is_not_started", &[]);
}

negative_control!(
    worker_start_failure_is_not_started,
    "started posted on a start that failed",
    expected = "the start failed, and it posted: started,failed",
    run(
        "worker_start_failure_is_not_started",
        &[(
            "worker.js",
            "async function start({",
            "async function start(args) { self.postMessage({ type: \"started\" }); return begin(args); }\nasync function begin({"
        )]
    )
);
