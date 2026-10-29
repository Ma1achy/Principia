//! `mock_layout_remembered` (REQ-GUI-214; R-462 J3, J5; render_gui_spec §G2's Layout; gui_state_contract §2): on the
//! mock, a layout changed by the splitters, saved and restored in a new GUI comes back the same; View › reset layout
//! returns every splitter to its default, and the defaults are saved; the layout is the GUI's own state, never the
//! contract's. Each check also runs on an outcome it must reject (R-176, through `rejects`; gui takes no dependency on
//! validation, R-187): a GUI that forgets the layout across a restart, and a reset that leaves a splitter moved.

use std::path::{Path, PathBuf};

use engine::contract::log::Source;

use super::mock_layout_splitters::{current, drag_splitter};
use super::support::{click, headless, mock_app, rejects, FORMAT};
use crate::app::App;
use crate::headless::Headless;
use crate::layout::{Layout, Splits, Splitter};
use crate::mock::MockEngine;
use crate::settings::{Settings, Store, FILE};
use crate::side::MockSide;

/// A directory of its own for test `name`, empty, under the system's temporary directory.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "principia-gui-layout-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// A GUI on the mock, its settings kept at `path`, settled.
fn gui(path: &Path) -> (App<MockSide>, Headless) {
    let mut app = App::with_settings(
        MockSide::new(MockEngine::new(), None),
        FORMAT,
        Store::at(path),
    );
    let mut h = headless();
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    (app, h)
}

/// Moves every splitter.
fn change(h: &mut Headless, app: &mut App<MockSide>) {
    for (s, by) in [
        (Splitter::ManifoldFigure, -16.0),
        (Splitter::FigureTrajectory, 40.0),
        (Splitter::TopBottom, 24.0),
        (Splitter::CompassTime, -60.0),
        (Splitter::TimeLegend, 90.0),
    ] {
        drag_splitter(h, app, s, by);
    }
}

#[test]
fn mock_layout_remembered_a_new_gui_comes_back_the_same() {
    let dir = scratch("same");
    let path = dir.join(FILE);
    let (left, splits) = {
        let (mut app, mut h) = gui(&path);
        change(&mut h, &mut app);
        let defaults = Splits::default();
        for s in Splitter::ALL {
            assert_ne!(app.splits.get(s), defaults.get(s), "{s:?} moved");
        }
        (current(&h, &app), app.splits)
    };
    assert!(path.exists(), "saved as the drags ended");
    let (again, h) = gui(&path);
    assert_eq!(again.splits, splits);
    let back = current(&h, &again);
    assert_eq!(back, left);
    // Drawn there, not only held: the panels' scopes placed at the layout's rects.
    for (id, rect) in [
        (crate::explore::MANIFOLD_VIEW, back.manifold_view),
        (crate::explore::FIGURE, back.figure),
        (crate::explore::trajectory::TRAJECTORY, back.trajectory),
        (crate::explore::COMPASS, back.compass),
        (crate::explore::time::TIME, back.time),
        (crate::explore::legend::LEGEND, back.legend),
    ] {
        assert_eq!(
            again.keyboard.place_of(id).map(|p| p.rect),
            Some(rect),
            "{id}"
        );
    }
    // A GUI that forgets: the same layout, started without the settings.
    let mut forgot = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut forgot, Vec::new());
    let fresh = current(&h, &forgot);
    rejects("a GUI that forgets the layout across a restart", || {
        assert_eq!(fresh, left)
    });
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_layout_remembered_view_reset_layout_restores_the_defaults() {
    let dir = scratch("reset");
    let path = dir.join(FILE);
    let (mut app, mut h) = gui(&path);
    change(&mut h, &mut app);
    let moved = app.splits;
    click(&mut h, &mut app, "View");
    click(&mut h, &mut app, "reset layout");
    let _ = h.frame(&mut app, Vec::new());
    let check = |splits: Splits| assert_eq!(splits, Splits::default());
    check(app.splits);
    assert_eq!(
        current(&h, &app),
        Layout::new(h.screen(), h.pixels_per_point())
    );
    // The defaults saved: a new GUI starts at them.
    let saved = Store::at(&path).load().settings;
    check(saved.layout);
    rejects("a reset that leaves a splitter moved", || check(moved));
    // From the keyboard: Enter on the View menu's entry acts as its click.
    change(&mut h, &mut app);
    let mut actions = crate::app::Actions::default();
    crate::explore::top_bar::activate(crate::explore::top_bar::RESET_LAYOUT.0, &mut actions);
    assert!(actions.reset_layout);
    let tree = app.keyboard.tree(app.view.mode);
    assert_eq!(
        tree.children(Some("view")),
        [
            crate::explore::top_bar::HIDE.0,
            crate::explore::top_bar::DEBUG_VIEW.0,
            crate::explore::top_bar::RESET_LAYOUT.0
        ]
    );
    assert!(
        tree.get(crate::explore::top_bar::RESET_LAYOUT.0)
            .unwrap()
            .activates
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_layout_remembered_the_file_reads_back_the_layout() {
    let mut layout = Splits::default();
    layout.set(Splitter::TopBottom, 0.7);
    layout.set(Splitter::CompassTime, 0.2);
    let settings = Settings {
        layout,
        ..Settings::default()
    };
    let text = settings.to_json();
    for s in Splitter::ALL {
        assert!(text.contains(&format!("\"{}\"", s.name())), "{text}");
    }
    assert_eq!(Settings::from_json(&text), Ok(settings.clone()));
    // A file with no layout, or a splitter it does not name, has the defaults there; a name the GUI does not have is
    // skipped.
    assert_eq!(
        Settings::from_json(r#"{"windows": {}}"#),
        Ok(Settings::default())
    );
    let one = Settings::from_json(r#"{"layout": {"top_bottom": 0.7, "gone": 0.5}}"#).expect("read");
    assert_eq!(one.layout.get(Splitter::TopBottom), 0.7);
    for s in Splitter::ALL
        .into_iter()
        .filter(|s| *s != Splitter::TopBottom)
    {
        assert_eq!(one.layout.get(s), Splits::default().get(s), "{s:?}");
    }
    // The ends of [0, 1] read; past them, a string or a layout that is not an object cannot be read.
    for edge in ["0", "1"] {
        let text = format!(r#"{{"layout": {{"time_legend": {edge}}}}}"#);
        assert!(Settings::from_json(&text).is_ok(), "{text}");
    }
    for bad in [
        r#"{"layout": []}"#,
        r#"{"layout": {"time_legend": -0.01}}"#,
        r#"{"layout": {"time_legend": 1.01}}"#,
        r#"{"layout": {"time_legend": "0.5"}}"#,
    ] {
        assert!(Settings::from_json(bad).is_err(), "{bad}");
    }
    rejects("an unreadable layout read", || {
        Settings::from_json(r#"{"layout": {"time_legend": 2}}"#).unwrap();
    });
}

#[test]
fn mock_layout_remembered_no_layout_reaches_the_contract() {
    let dir = scratch("contract");
    let path = dir.join(FILE);
    let (mut app, mut h) = gui(&path);
    let before = app.snapshot().clone();
    let lines = |app: &App<MockSide>| {
        app.console()
            .iter()
            .filter(|e| e.source == Source::Contract)
            .count()
    };
    let n = lines(&app);
    change(&mut h, &mut app);
    click(&mut h, &mut app, "View");
    click(&mut h, &mut app, "reset layout");
    let _ = h.frame(&mut app, Vec::new());
    let after = app.snapshot().clone();
    let reached = |a: &engine::contract::snapshot::Snapshot| {
        assert_eq!(a.sim, before.sim);
        assert_eq!(a.render, before.render);
        assert_eq!(a.history, before.history);
    };
    reached(&after);
    assert_eq!(lines(&app), n, "no SetField logged");
    app.set_field(super::support::playhead(1.5, false));
    let _ = h.frame(&mut app, Vec::new());
    let edited = app.snapshot().clone();
    rejects("an edit that entered the contract", || reached(&edited));
    let _ = std::fs::remove_dir_all(&dir);
}
