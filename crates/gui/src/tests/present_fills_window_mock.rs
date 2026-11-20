//! `present_fills_window_mock` (REQ-GUI-179; R-406, R-407; render_gui_spec § "Export & share"): present mode, opened
//! from Export & share by the keys, hides the chrome as F3 does, so its figure fills the window the same way: its
//! capture equals the F3-off one, and Esc restores the shown capture pixel for pixel. Present sends no `SetField`.
//! Each check also runs on an input it must reject (R-176, through `rejects`). These run on the GPU, on
//! `PRIN_GPU_BACKEND`'s backend (R-206).

use eframe::egui::{Key, Modifiers};

use super::f3_fills_window_mock::{canvas, check_covers, left_in, shown_px};
use super::support::{focus, headless, mock_app, press, rejects};
use crate::app::App;
use crate::capture::{shoot, Step};
use crate::headless::Headless;
use crate::side::MockSide;
use crate::windows::export;

/// The keys to Export & share's Present, focused, not yet pressed: Tab to the top bar, Enter into it, → six times to
/// Export…, Enter to open the window, then Tab twice to its Present section. The screenshot case `mock_present` plays
/// these and Enter.
const TO_PRESENT: [(Key, Step); 11] = [
    (Key::Tab, Step::Tab),
    (Key::Enter, Step::Enter),
    (Key::ArrowRight, Step::ArrowRight),
    (Key::ArrowRight, Step::ArrowRight),
    (Key::ArrowRight, Step::ArrowRight),
    (Key::ArrowRight, Step::ArrowRight),
    (Key::ArrowRight, Step::ArrowRight),
    (Key::ArrowRight, Step::ArrowRight),
    (Key::Enter, Step::Enter),
    (Key::Tab, Step::Tab),
    (Key::Tab, Step::Tab),
];

/// [`TO_PRESENT`]'s steps, then `more`.
fn steps(more: &[Step]) -> Vec<Step> {
    TO_PRESENT
        .iter()
        .map(|(_, s)| *s)
        .chain(more.iter().copied())
        .collect()
}

/// The screenshot case `01_main/mock_present` plays [`TO_PRESENT`] and Enter.
#[test]
fn present_fills_window_mock_case_plays_the_route() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/screenshot/01_main/cases.json"
    );
    let cases: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("01_main's cases"))
            .expect("JSON");
    let case = cases["cases"]
        .as_array()
        .expect("a list")
        .iter()
        .find(|c| c["name"] == "mock_present")
        .expect("a mock_present case");
    let names: Vec<&str> = case["surface"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| s.as_str().expect("a step name"))
        .collect();
    let parsed: Vec<Step> = names
        .iter()
        .map(|n| Step::parse(n).expect("a step"))
        .collect();
    assert_eq!(parsed, steps(&[Step::Enter]));
    rejects("the route without its Enter", || {
        assert_eq!(parsed, steps(&[]))
    });
}

#[test]
fn present_fills_window_mock_as_f3_off() {
    let canvas = canvas();
    let f3_off = shoot(canvas.clone(), &[Step::F3]);
    let before = shoot(canvas.clone(), &steps(&[]));
    let present = shoot(canvas.clone(), &steps(&[Step::Enter]));
    let back = shoot(canvas, &steps(&[Step::Enter, Step::Escape]));
    check_covers(&present.rgba, "present mode");
    assert!(
        present.rgba == f3_off.rgba,
        "present mode is not the F3-off capture"
    );
    assert!(present.names.is_empty(), "chrome named in present mode");
    assert!(
        back.rgba == before.rgba,
        "Esc did not restore the shown capture"
    );
    // A present mode left in the shown rect fails the coverage check.
    rejects("a present mode left in the shown rect", || {
        check_covers(&left_in(&present.rgba, shown_px()), "left in its rect")
    });
    rejects("the shown capture taken as present mode", || {
        assert!(before.rgba == f3_off.rgba, "the shown capture")
    });
}

/// Frames enough for a snapshot to be read after an edit.
fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..3 {
        let _ = h.frame(app, Vec::new());
    }
}

/// The undo depth, the `SetField`s the mock logged and the chart.
fn logged(app: &App<MockSide>) -> (u32, usize, engine::contract::sim_config::Plane) {
    let n = app
        .console()
        .iter()
        .filter(|e| e.message.starts_with("SetField"))
        .count();
    (
        app.snapshot().history.undo_depth,
        n,
        app.snapshot().sim.plane.clone(),
    )
}

#[test]
fn present_fills_window_mock_sends_no_set_field() {
    let mut app = mock_app();
    let mut h = headless();
    settle(&mut h, &mut app);
    for (key, _) in TO_PRESENT {
        press(&mut h, &mut app, key, Modifiers::NONE);
    }
    settle(&mut h, &mut app);
    assert_eq!(
        focus(&app).last(),
        Some(&export::PRESENT),
        "the route ends on Present"
    );
    let before = logged(&app);
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    settle(&mut h, &mut app);
    assert!(app.present, "Enter on Present entered present mode");
    assert_eq!(logged(&app), before, "present mode sent an edit");
    press(&mut h, &mut app, Key::Escape, Modifiers::NONE);
    settle(&mut h, &mut app);
    assert!(!app.present, "Esc left present mode");
    assert_eq!(logged(&app), before, "Esc sent an edit");
    rejects("an undo depth one deeper", || {
        let (depth, n, plane) = logged(&app);
        assert_eq!((depth + 1, n, plane), before)
    });
}
