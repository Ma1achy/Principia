//! QA tests for TASK-M6-46's launch switch (REQ-GUI-226; R-467 D8), written from the requirement, not from the
//! implementation: `PRIN_MOCK_INVALID="s₀,t₀,s₁,t₁"`, set at launch, flags its region as `MockCanvas::flag_invalid`
//! does, read by the app and by the capture mode; a malformed value is refused with a message naming it.
//!
//! The capture mode is run as the binary runs it (`gui::cli::mock::dispatch`, `capture`), its `capture.png` read back
//! and compared with the same steps shot through a canvas flagged by `flag_invalid`. This binary holds one test, so
//! nothing else reads the process's environment while it is set. gui takes no dependency on validation (R-187), so each
//! check runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-46_env` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gui::capture::{self, Step};
use gui::cli::mock::dispatch;
use gui::mock::canvas::MockCanvas;

const VAR: &str = "PRIN_MOCK_INVALID";

/// R-467 D8's own example: the band left of the square.
const REGION: &str = "-0.5,-0.5,0,1.5";
const REGION_F: [f32; 4] = [-0.5, -0.5, 0.0, 1.5];

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn out_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_TASK-M6-46_env/{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the output directory");
    dir
}

/// A runner for the window that must not be reached.
fn no_window(_: &str, _: eframe::NativeOptions, _: eframe::AppCreator<'static>) -> eframe::Result {
    panic!("the window opened")
}

/// Runs the capture mode with `steps` into `out`, the variable as `value` (`None`: unset).
fn capture_with(value: Option<&str>, steps: &str, out: &Path) -> Result<(), String> {
    match value {
        Some(v) => std::env::set_var(VAR, v),
        None => std::env::remove_var(VAR),
    }
    let args: Vec<String> = [
        "capture",
        "--screen",
        "01_main",
        "--steps",
        steps,
        "--out",
        out.to_str().expect("a UTF-8 path"),
    ]
    .map(str::to_owned)
    .to_vec();
    let result = dispatch(&args, no_window);
    std::env::remove_var(VAR);
    result
}

fn read_png(path: &Path) -> Vec<u8> {
    let file = std::fs::File::open(path).expect("the capture");
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("a PNG");
    let mut rgba = vec![0; reader.output_buffer_size().expect("a size")];
    reader.next_frame(&mut rgba).expect("its pixels");
    rgba
}

/// A refusal of `value`: an error whose message names the value.
fn check_refused(value: &str, result: &Result<(), String>) {
    let message = result
        .as_ref()
        .err()
        .unwrap_or_else(|| panic!("{VAR}={value:?} accepted"));
    assert!(
        message.contains(value),
        "the refusal of {value:?} does not name it: {message}"
    );
}

#[test]
fn qa_m6_46_prin_mock_invalid_at_launch() {
    let steps = "f3";
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    // What flag_invalid draws, F3 off, and the unflagged capture.
    canvas.flag_invalid(REGION_F);
    let flagged = capture::shoot(canvas.clone(), &[Step::F3]).rgba;
    canvas.flag_invalid([0.0; 4]);
    let plain = capture::shoot(canvas.clone(), &[Step::F3]).rgba;
    assert!(flagged != plain, "the region changes nothing F3 off");
    drop(canvas);

    // Set at launch, the capture mode draws what flag_invalid draws.
    let out = out_dir("set");
    capture_with(Some(REGION), steps, &out).expect("the capture with the region");
    let set = read_png(&out.join("capture.png"));
    let check = |rgba: &[u8]| assert!(rgba == flagged, "the capture is not the flagged one");
    check(&set);
    // Control: unset, the capture is the unflagged one, which the check rejects.
    let out = out_dir("unset");
    capture_with(None, steps, &out).expect("the capture with no region");
    let unset = read_png(&out.join("capture.png"));
    assert!(unset == plain, "unset, the capture is not the plain one");
    rejects("the capture with the variable unset", || check(&unset));

    // A malformed value is refused, naming it, by the capture mode and by the window, which never opens.
    for bad in ["abc", "0,0,1", "0,0,1,1,1", "0;0;1;1", "-0.5,-0.5,0,x"] {
        let out = out_dir("bad");
        let result = capture_with(Some(bad), steps, &out);
        check_refused(bad, &result);
        assert!(
            !out.join("capture.png").exists(),
            "a capture written for {bad:?}"
        );
        std::env::set_var(VAR, bad);
        let window = dispatch(&[], no_window);
        std::env::remove_var(VAR);
        check_refused(bad, &window);
    }
    rejects("a well-formed value read as refused", || {
        check_refused(REGION, &Ok(()))
    });
    rejects("a refusal that does not name the value", || {
        check_refused("abc", &Err(format!("{VAR} is malformed")))
    });
}
