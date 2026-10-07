//! `mock_status_line` (REQ-GUI-168, REQ-GUI-176; RQ-255): the top bar's status line shows the mock snapshot's `t`,
//! fps, frame ms and quad count in 01_main.png's format, its undo depth 2 after two edits on the mock and 1 after an
//! undo, with the redo depth 1; "—" for an absent value; the footer shows the mock's memory readout and the "? keys"
//! hint. The top bar's controls are drawn and the menus open (RQ-249).

use engine::contract::snapshot::LiveMemory;

use super::support::{click, frame_texts, headless, mock_app, playhead, real_app, rejects};
use crate::app::Counts;
use crate::console::time_of_day;
use crate::explore::footer::{counts_label, megabytes, memory_readout};
use crate::explore::top_bar::{grouped, status_line, WINDOWS};

/// `texts` holds `line`.
fn check_has(texts: &[String], line: &str) {
    assert!(
        texts.iter().any(|t| t == line),
        "no `{line}` among the names: {texts:?}"
    );
}

#[test]
fn mock_status_line_after_two_edits_and_an_undo() {
    let mut app = mock_app();
    let mut headless = headless();
    app.set_field(playhead(1.0, false));
    app.set_field(playhead(12.4, false));
    let texts = frame_texts(&mut headless, &mut app);
    check_has(
        &texts,
        "t 12.40 · 60 fps · 4.1 ms · 1 842 quads · undo 2 / redo 0 · F3 hide",
    );
    check_has(&texts, "GPU 464 MB · heap 148 MB");
    check_has(&texts, "? keys");
    check_has(&texts, "principia · dev");
    app.undo();
    let texts = frame_texts(&mut headless, &mut app);
    check_has(
        &texts,
        "t 1.00 · 60 fps · 4.1 ms · 1 842 quads · undo 1 / redo 1 · F3 hide",
    );
    rejects("the status line before the undo", || {
        check_has(
            &texts,
            "t 12.40 · 60 fps · 4.1 ms · 1 842 quads · undo 2 / redo 0 · F3 hide",
        )
    });
}

#[test]
fn mock_status_line_absent_values_read_a_dash() {
    let mut app = real_app();
    let texts = frame_texts(&mut headless(), &mut app);
    check_has(
        &texts,
        "t 0.00 · — fps · — ms · — quads · undo 0 / redo 0 · F3 hide",
    );
    check_has(&texts, "GPU — · heap —");
    assert_eq!(memory_readout(None), "GPU — · heap —");
    assert_eq!(
        memory_readout(Some(LiveMemory {
            heap_bytes: 2_499_999,
            gpu_bytes: 2_500_000
        })),
        "GPU 3 MB · heap 2 MB"
    );
    let snapshot = app.snapshot().clone();
    assert!(status_line(&snapshot).starts_with("t 0.00 · — fps"));
}

#[test]
fn mock_status_line_number_formats() {
    let cases = [
        (0, "0"),
        (999, "999"),
        (1000, "1 000"),
        (1842, "1 842"),
        (123_456, "123 456"),
        (1_234_567, "1 234 567"),
    ];
    for (n, text) in cases {
        assert_eq!(grouped(n), text);
    }
    assert_eq!(megabytes(0), "0 MB");
    assert_eq!(megabytes(499_999), "0 MB");
    assert_eq!(megabytes(500_000), "1 MB");
    assert_eq!(megabytes(u64::MAX), format!("{} MB", u64::MAX / 1_000_000));
    assert_eq!(time_of_day(0), "00:00:00.000");
    assert_eq!(time_of_day(crate::mock::MOCK_START_MS), "12:03:59.410");
    assert_eq!(time_of_day(86_399_999), "23:59:59.999");
    assert_eq!(time_of_day(86_400_000 + 3_723_004), "01:02:03.004");
    let counts = Counts {
        warnings: 1,
        errors: 0,
    };
    assert_eq!(counts_label(counts, false), "⚠ 1 · × 0 · console ▴");
    assert_eq!(counts_label(counts, true), "⚠ 1 · × 0 · console ▾");
}

/// Every control 01_main.png's top bar shows is drawn, the menus open with their items, and the mode switch shows the
/// Stain page's empty frame and back (RQ-249).
#[test]
fn mock_status_line_top_bar_controls() {
    let mut app = mock_app();
    let mut headless = headless();
    let texts = frame_texts(&mut headless, &mut app);
    for name in [
        "principia · dev",
        "File",
        "View",
        "Windows",
        "Overlays ▾ 0",
        "Run…",
        "Profiler…",
        "Export…",
        "Help",
        "Explore",
        "Stain",
        "Manifold view",
        "Trajectory",
        "Compass",
        "Time",
        "Legend",
    ] {
        check_has(&texts, name);
    }
    for (menu, items) in [
        ("File", &["Quit"][..]),
        ("View", &["F3 hide"][..]),
        ("Windows", &WINDOWS[..]),
        ("Help", &["Keys (?)"][..]),
    ] {
        click(&mut headless, &mut app, menu);
        let texts = frame_texts(&mut headless, &mut app);
        for item in items {
            check_has(&texts, item);
        }
        // Close it again, with a click on an empty part of the page.
        click(&mut headless, &mut app, "Legend");
    }
    click(&mut headless, &mut app, "Stain");
    let texts = frame_texts(&mut headless, &mut app);
    assert!(
        !texts.iter().any(|t| t == "Manifold view"),
        "Explore's regions on the Stain page"
    );
    assert_eq!(app.view.mode, engine::contract::view_ui::Mode::Stain);
    click(&mut headless, &mut app, "Explore");
    check_has(&frame_texts(&mut headless, &mut app), "Manifold view");
    // File's Quit closes the window.
    click(&mut headless, &mut app, "File");
    let names = super::support::names(&mut headless, &mut app);
    let quit = names
        .iter()
        .find(|n| n.name == "Quit")
        .and_then(|n| n.rect)
        .expect("Quit");
    let centre = eframe::egui::pos2(
        ((quit[0] + quit[2]) / 3.0) as f32,
        ((quit[1] + quit[3]) / 3.0) as f32,
    );
    let mut closed = false;
    for events in crate::capture::click(centre) {
        let output = headless.frame(&mut app, events);
        closed |= output.viewport_output[&eframe::egui::ViewportId::ROOT]
            .commands
            .contains(&eframe::egui::ViewportCommand::Close);
    }
    assert!(closed, "Quit did not close the window");
    // View's "F3 hide" hides the layer, as F3 does.
    click(&mut headless, &mut app, "View");
    click(&mut headless, &mut app, "F3 hide");
    assert!(!app.shown);
    let hidden = frame_texts(&mut headless, &mut app);
    assert!(
        !hidden.iter().any(|t| t == "principia · dev"),
        "the layer still shows: {hidden:?}"
    );
}

/// `point`, in points, lies inside `rect`, in pixels at `scale` pixels per point.
fn check_on(rect: [f64; 4], point: eframe::egui::Pos2, scale: f64) {
    let (x, y) = (f64::from(point.x) * scale, f64::from(point.y) * scale);
    assert!(
        rect[0] <= x && x <= rect[2] && rect[1] <= y && y <= rect[3],
        "({x}, {y}) is not on the counts at {rect:?}"
    );
}

#[test]
fn mock_status_line_footer_click_lands_on_the_counts() {
    use crate::capture::footer_point;
    use crate::layout::Layout;
    let mut app = mock_app();
    let mut headless = headless();
    let names = super::support::names(&mut headless, &mut app);
    let counts = names
        .iter()
        .find(|n| n.name == "⚠ 0 · × 0 · console ▴")
        .and_then(|n| n.rect)
        .expect("the counts are drawn");
    let footer = Layout::new(headless.screen(), headless.pixels_per_point()).footer;
    let scale = f64::from(headless.pixels_per_point());
    let point = footer_point(footer);
    check_on(counts, point, scale);
    rejects("the footer's left edge", || {
        check_on(counts, eframe::egui::pos2(footer.min.x, point.y), scale)
    });
}

/// The console window is open exactly when `open`, and the counts' arrow says so.
fn check_console(texts: &[String], open: bool) {
    let arrow = if open { "▾" } else { "▴" };
    check_has(texts, &format!("⚠ 0 · × 0 · console {arrow}"));
    assert_eq!(
        texts.iter().any(|t| t == crate::console::TITLE),
        open,
        "the console window's presence: {texts:?}"
    );
}

#[test]
fn mock_status_line_console_window_closes_from_its_title_bar() {
    let mut app = mock_app();
    let mut headless = headless();
    check_console(&frame_texts(&mut headless, &mut app), false);
    click(&mut headless, &mut app, "⚠ 0 · × 0 · console ▴");
    let opened = frame_texts(&mut headless, &mut app);
    check_console(&opened, true);
    click(&mut headless, &mut app, "Close window");
    let closed = frame_texts(&mut headless, &mut app);
    check_console(&closed, false);
    rejects("the closed console read as open", || {
        check_console(&closed, true)
    });
    rejects("the open console read as closed", || {
        check_console(&opened, false)
    });
}

/// The AccessKit toggled state of the node labelled `label` in `output`.
fn toggled(
    output: &eframe::egui::FullOutput,
    label: &str,
) -> Option<eframe::egui::accesskit::Toggled> {
    let update = output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("AccessKit is on");
    update
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label))
        .and_then(|(_, node)| node.toggled())
}

/// The mode switch marks `on` selected and `off` not.
fn check_mode(output: &eframe::egui::FullOutput, on: &str, off: &str) {
    use eframe::egui::accesskit::Toggled;
    assert_eq!(
        toggled(output, on),
        Some(Toggled::True),
        "{on} not selected"
    );
    assert_eq!(toggled(output, off), Some(Toggled::False), "{off} selected");
}

#[test]
fn mock_status_line_mode_switch_marks_the_mode_and_draws_its_page() {
    use crate::layout::Layout;
    let mut app = mock_app();
    let mut headless = headless();
    let explore = headless.frame(&mut app, Vec::new());
    check_mode(&explore, "Explore", "Stain");
    click(&mut headless, &mut app, "Stain");
    let stain = headless.frame(&mut app, Vec::new());
    check_mode(&stain, "Stain", "Explore");
    rejects("the Stain page read as Explore", || {
        check_mode(&stain, "Explore", "Stain")
    });
    // The Stain page's title is drawn on the page, below the top bar's switch.
    let page = Layout::new(headless.screen(), headless.pixels_per_point()).page;
    let s = f64::from(headless.pixels_per_point());
    let on_page = |names: &[crate::headless::Name]| {
        names.iter().any(|n| {
            n.name == "Stain"
                && n.rect.is_some_and(|r| {
                    f64::from(page.min.x) * s <= r[0] && f64::from(page.min.y) * s <= r[1]
                })
        })
    };
    assert!(on_page(&headless.names(&stain)), "no Stain page title");
    assert!(
        !on_page(&headless.names(&explore)),
        "a Stain page title on Explore"
    );
}

#[test]
fn mock_status_line_severity_colours() {
    use crate::explore::footer::severity_colour;
    use engine::contract::log::Severity;
    let v = eframe::egui::Visuals::dark();
    let colours = [Severity::Error, Severity::Warn, Severity::Info].map(|s| severity_colour(&v, s));
    let check = |c: [eframe::egui::Color32; 3]| {
        assert_eq!(c, [v.error_fg_color, v.warn_fg_color, v.weak_text_color()]);
    };
    check(colours);
    rejects("errors in the warning colour", || {
        check([v.warn_fg_color, v.warn_fg_color, v.weak_text_color()])
    });
}
