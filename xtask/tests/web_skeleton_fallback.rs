//! The skeleton's fallback (TASK-M8-86, R-627): a worker layout that fails before its first frame has returned gives way
//! to the main thread, leaving none of its forwards behind. `web_skeleton_fallback.mjs` runs the real `worker.js` and
//! `loader.js` on Node, with a stub `gui.js` and a stub page; each control runs it on the file with the fix undone.

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
#[cfg(feature = "controls")]
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

/// One edit of `loader.js`.
#[cfg(feature = "controls")]
const fn loader<'a>(from: &'a str, to: &'a str) -> [(&'a str, &'a str, &'a str); 1] {
    [("loader.js", from, to)]
}

const FAILED: &str = "loader_failed_worker_gives_up_its_forwards";

#[test]
fn loader_failed_worker_gives_up_its_forwards() {
    run(FAILED, &[]);
}

mod failed_input_stays {
    use super::*;
    negative_control!(
        loader_failed_worker_gives_up_its_forwards,
        "the hidden input not removed",
        expected = "the hidden input stays",
        run(FAILED, &loader("    ime.remove();\n", ""))
    );
}

mod failed_observer_stays {
    use super::*;
    negative_control!(
        loader_failed_worker_gives_up_its_forwards,
        "the ResizeObserver not disconnected",
        expected = "the ResizeObserver stays",
        run(FAILED, &loader("    resized.disconnect();\n", ""))
    );
}

mod failed_keys_stay {
    use super::*;
    negative_control!(
        loader_failed_worker_gives_up_its_forwards,
        "the window's keydown listener left in place",
        expected = "the window's keydown is still forwarded",
        run(
            FAILED,
            &loader(
                "addEventListener(\"keydown\", key(true), { signal });",
                "addEventListener(\"keydown\", key(true));"
            )
        )
    );
}

mod failed_focus_stays {
    use super::*;
    negative_control!(
        loader_failed_worker_gives_up_its_forwards,
        "the window's focus listener left in place",
        expected = "the window's focus is still forwarded",
        run(
            FAILED,
            &loader(
                "addEventListener(\"focus\", () => { ime.focus(); post(\"focus\", true); }, { signal });",
                "addEventListener(\"focus\", () => { ime.focus(); post(\"focus\", true); });"
            )
        )
    );
}

mod failed_canvas_stays {
    use super::*;
    negative_control!(
        loader_failed_worker_gives_up_its_forwards,
        "the old canvas's press listener left in place",
        expected = "the canvas's pointerdown is still forwarded",
        run(
            FAILED,
            &loader(
                "canvas.addEventListener(\"pointerdown\", button(true), { signal });",
                "canvas.addEventListener(\"pointerdown\", button(true));"
            )
        )
    );
}

mod failed_no_focus {
    use super::*;
    negative_control!(
        loader_failed_worker_gives_up_its_forwards,
        "the fallback's canvas not focused",
        expected = "the fallback's canvas does not hold the keyboard",
        run(FAILED, &loader("  canvas.focus();\n", ""))
    );
}

#[test]
fn loader_broken_worker_gives_up_its_forwards() {
    run("loader_broken_worker_gives_up_its_forwards", &[]);
}

negative_control!(
    loader_broken_worker_gives_up_its_forwards,
    "failed stops the worker but keeps the forwards",
    expected = "the hidden input stays",
    run(
        "loader_broken_worker_gives_up_its_forwards",
        &loader(
            "      clearTimeout(timer);\n      stop();\n",
            "      clearTimeout(timer);\n      worker.terminate();\n"
        )
    )
);

#[test]
fn loader_timed_out_worker_gives_up_its_forwards() {
    run("loader_timed_out_worker_gives_up_its_forwards", &[]);
}

negative_control!(
    loader_timed_out_worker_gives_up_its_forwards,
    "the timer stops the worker but keeps the forwards",
    expected = "the hidden input stays",
    run(
        "loader_timed_out_worker_gives_up_its_forwards",
        &loader(
            "      stop();\n      reject(",
            "      worker.terminate();\n      reject("
        )
    )
);

#[test]
fn loader_started_worker_keeps_its_forwards() {
    run("loader_started_worker_keeps_its_forwards", &[]);
}

negative_control!(
    loader_started_worker_keeps_its_forwards,
    "the forwards undone on the start",
    expected = "a key is not forwarded",
    run(
        "loader_started_worker_keeps_its_forwards",
        &loader(
            "          started = true;\n",
            "          started = true;\n          unforward();\n"
        )
    )
);

#[test]
fn loader_fatal_worker_gives_up_its_forwards() {
    run("loader_fatal_worker_gives_up_its_forwards", &[]);
}

negative_control!(
    loader_fatal_worker_gives_up_its_forwards,
    "a fatal failure keeps the forwards",
    expected = "the hidden input stays",
    run(
        "loader_fatal_worker_gives_up_its_forwards",
        &loader(
            "      clearTimeout(timer);\n      stop();\n",
            "      clearTimeout(timer);\n      worker.terminate();\n"
        )
    )
);

const ALTGRAPH: &str = "loader_altgraph_character_reaches_the_input";
#[cfg(feature = "controls")]
const RELEASE: &str = "const name = pressed ? e.key : down.get(e.code);";
#[cfg(feature = "controls")]
const ALT_GR: &str = r#"const altGr = e.ctrlKey && !e.metaKey && e.getModifierState("AltGraph");"#;
#[cfg(feature = "controls")]
const CHARACTER: &str =
    r#"if (pressed && altGr && (e.key.length === 1 || e.key === "Dead")) return;"#;

#[test]
fn loader_altgraph_character_reaches_the_input() {
    run(ALTGRAPH, &[]);
}

mod altgraph_a_chord {
    use super::*;
    negative_control!(
        loader_altgraph_character_reaches_the_input,
        "AltGr's characters not told from chords, as Codex's finding traced",
        expected = "AltGr's `@` was cancelled",
        run(ALTGRAPH, &loader(CHARACTER, ""))
    );
}

mod altgraph_forwards_the_key {
    use super::*;
    negative_control!(
        loader_altgraph_character_reaches_the_input,
        "the input keeps AltGr's character, and its key goes on too: Ctrl+Alt+A, egui's select-all",
        expected = "for egui to read as Ctrl+Alt",
        run(
            ALTGRAPH,
            &loader(
                CHARACTER,
                r#"if (pressed && altGr && (e.key.length === 1 || e.key === "Dead")) { post("key", e.key, e.code, pressed, e.repeat, ...mods(e)); return; }"#
            )
        )
    );
}

mod ctrl_never_a_chord {
    use super::*;
    negative_control!(
        loader_altgraph_character_reaches_the_input,
        "any Ctrl taken for AltGr, not only under AltGraph",
        expected = "Ctrl+S was left to the page",
        run(
            ALTGRAPH,
            &loader(ALT_GR, "const altGr = e.ctrlKey && !e.metaKey;")
        )
    );
}

mod altgraph_excuses_meta {
    use super::*;
    negative_control!(
        loader_altgraph_character_reaches_the_input,
        "AltGraph excusing Cmd as well as Ctrl",
        expected = "Cmd+S under AltGraph was left to the page",
        run(
            ALTGRAPH,
            &loader(
                ALT_GR,
                r#"const altGr = e.ctrlKey && e.getModifierState("AltGraph");"#
            )
        )
    );
}

mod altgraph_takes_every_key {
    use super::*;
    negative_control!(
        loader_altgraph_character_reaches_the_input,
        "every key under AltGraph taken for a character",
        expected = "AltGr's Enter was left to the page",
        run(
            ALTGRAPH,
            &loader(CHARACTER, "if (pressed && altGr) return;")
        )
    );
}

mod altgraph_dead_key_a_chord {
    use super::*;
    negative_control!(
        loader_altgraph_character_reaches_the_input,
        "a dead key under AltGraph not taken for a character",
        expected = "AltGr's dead key was cancelled",
        run(
            ALTGRAPH,
            &loader(
                CHARACTER,
                "if (pressed && altGr && e.key.length === 1) return;"
            )
        )
    );
}

mod altgraph_swallows_a_release {
    use super::*;
    negative_control!(
        loader_altgraph_character_reaches_the_input,
        "the guard on keyup too, as code review 5777907216 F1 traced: a key held as AltGr goes down stays down in egui",
        expected = "Space's keyup under AltGraph did not reach the worker",
        run(
            ALTGRAPH,
            &loader(
                CHARACTER,
                r#"if (altGr && (e.key.length === 1 || e.key === "Dead")) return;"#
            )
        )
    );
}

mod altgraph_forwards_every_release {
    use super::*;
    negative_control!(
        loader_altgraph_character_reaches_the_input,
        "every keyup forwarded, not only those of a press that went on",
        expected = "for egui to read as Ctrl+Alt",
        run(
            ALTGRAPH,
            &loader(
                RELEASE,
                "const name = pressed ? e.key : down.get(e.code) ?? e.key;"
            )
        )
    );
}

mod release_under_the_layer_name {
    use super::*;
    negative_control!(
        loader_altgraph_character_reaches_the_input,
        "a release named by its own event: a known code alone, with the layer's name, lets another key up in egui",
        expected = "KeyQ came up as @",
        run(
            ALTGRAPH,
            &loader(
                RELEASE,
                "const name = pressed || down.has(e.code) ? e.key : undefined;"
            )
        )
    );
}
