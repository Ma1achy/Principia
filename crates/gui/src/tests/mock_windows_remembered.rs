//! REQ-GUI-212 (R-461 I2, I4; render_gui_spec §G5; gui_state_contract §2): on the mock, a window moved and resized
//! opens, in a new GUI started from the settings the first saved, at the same place and size; a remembered place off a
//! smaller screen is kept on it; with no settings each window opens at the artboards' place; a settings file that
//! cannot be read is ignored with one console line, and replaced on the next save; no window's place reaches the
//! contract's snapshot or its history. Each check also runs on an outcome it must reject (R-176, through `rejects`):
//! a GUI that forgets the place across a restart, and a snapshot an edit did reach.

use std::path::{Path, PathBuf};

use eframe::egui::{pos2, vec2, Rect};
use engine::contract::log::{Severity, Source};
use engine::contract::view_ui::Window;

use super::mock_windows_move_resize::{
    assert_on_screen, drag, drag_title, open, placed, settle, MOVABLE,
};
use super::support::{headless, mock_app, playhead, rejects, FORMAT};
use crate::app::App;
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::MockEngine;
use crate::settings::{platform_path, Settings, Store, DIR, FILE};
use crate::side::MockSide;
use crate::windows::place::{Places, KEEP_W, TITLE_BAR_H};
use crate::windows::{self as wins, entry, place};

/// A directory of its own for test `name`, empty, under the system's temporary directory.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("principia-gui-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// A GUI on the mock, its settings kept at `path`.
fn gui(path: &Path) -> App<MockSide> {
    App::with_settings(
        MockSide::new(MockEngine::new(), None),
        FORMAT,
        Store::at(path),
    )
}

/// The app's console lines from the app itself, warnings, about the settings.
fn settings_lines(app: &App<MockSide>) -> Vec<String> {
    app.console()
        .iter()
        .filter(|e| e.source == Source::App && e.severity == Severity::Warn)
        .map(|e| e.message.clone())
        .collect()
}

/// Moves `window` by its title bar and resizes it from its bottom-right corner.
fn move_and_resize(h: &mut Headless, app: &mut App<MockSide>, window: Window) -> Rect {
    drag_title(h, app, window, vec2(-60.0, 50.0));
    let r = placed(h, app, window);
    let corner = r.max - vec2(2.0, 2.0);
    drag(h, app, corner, corner + vec2(-20.0, -40.0));
    placed(h, app, window)
}

#[test]
fn mock_windows_remembered_a_new_gui_opens_each_window_where_it_was_left() {
    let dir = scratch("same-place");
    let path = dir.join(FILE);
    let mut left = Vec::new();
    {
        let mut app = gui(&path);
        let mut h = headless();
        for window in MOVABLE {
            open(&mut h, &mut app, window);
            let before = placed(&h, &app, window);
            let after = move_and_resize(&mut h, &mut app, window);
            assert_ne!(after.min, before.min, "{window:?} moved");
            assert_ne!(after.size(), before.size(), "{window:?} resized");
            left.push((window, after));
        }
    }
    assert!(path.exists(), "the settings were saved");
    let mut again = gui(&path);
    let mut h = headless();
    for &(window, rect) in &left {
        open(&mut h, &mut again, window);
        assert_eq!(placed(&h, &again, window), rect, "{window:?}");
    }
    assert!(settings_lines(&again).is_empty());
    // A GUI that forgets: the same windows, opened without the settings.
    let mut forgot = mock_app();
    let mut h = headless();
    for &(window, rect) in &left {
        open(&mut h, &mut forgot, window);
        let at = placed(&h, &forgot, window);
        rejects("a GUI that forgets the place across a restart", || {
            assert_eq!(at, rect, "{window:?}")
        });
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_windows_remembered_a_place_off_a_smaller_screen_is_clamped() {
    let dir = scratch("smaller");
    let path = dir.join(FILE);
    // Left on a 1440 × 900 point screen, in its bottom-right corner.
    let far = Rect::from_min_size(pos2(1300.0, 820.0), vec2(320.0, 300.0));
    let settings = Settings {
        places: Places::remembered([(Window::Display, far)]),
        ..Settings::default()
    };
    Store::at(&path).save(&settings).expect("saved");
    let mut app = gui(&path);
    let mut h = headless();
    h.set_size([1200, 900]);
    open(&mut h, &mut app, Window::Display);
    let screen = h.screen();
    let at = placed(&h, &app, Window::Display);
    assert_on_screen(at, screen);
    assert_eq!(
        at.min,
        pos2(screen.max.x - KEEP_W, screen.max.y - TITLE_BAR_H)
    );
    assert_eq!(at.size(), far.size());
    rejects("a remembered place opened as it was", || {
        assert_on_screen(far, screen)
    });
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_windows_remembered_with_no_settings_each_window_opens_at_the_artboards_place() {
    let dir = scratch("none");
    let path = dir.join(FILE);
    let mut app = gui(&path);
    let mut h = headless();
    let layout = Layout::new(h.screen(), h.pixels_per_point());
    for window in MOVABLE {
        open(&mut h, &mut app, window);
        assert_eq!(
            placed(&h, &app, window),
            wins::rect(window, &layout),
            "{window:?}"
        );
    }
    assert!(settings_lines(&app).is_empty(), "no file is not an error");
    assert!(!path.exists(), "nothing saved without a move");
    let moved = Places::remembered([(
        Window::Run,
        wins::rect(Window::Run, &layout).translate(vec2(9.0, 9.0)),
    )]);
    rejects("a window opened elsewhere", || {
        assert_eq!(
            moved.shown(Window::Run, wins::rect(Window::Run, &layout), h.screen()),
            wins::rect(Window::Run, &layout)
        )
    });
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_windows_remembered_a_corrupt_file_is_ignored_with_one_console_line() {
    let dir = scratch("corrupt");
    let path = dir.join(FILE);
    std::fs::write(&path, "{\"windows\": {\"Run\": [1, 2").expect("written");
    let mut app = gui(&path);
    let mut h = headless();
    let layout = Layout::new(h.screen(), h.pixels_per_point());
    open(&mut h, &mut app, Window::Run);
    assert_eq!(
        placed(&h, &app, Window::Run),
        wins::rect(Window::Run, &layout)
    );
    settle(&mut h, &mut app);
    let lines = settings_lines(&app);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("could not be read"), "{}", lines[0]);
    assert_eq!(app.counts().warnings, 1);
    // Replaced on the next save, which reads back.
    let rect = move_and_resize(&mut h, &mut app, Window::Run);
    let text = std::fs::read_to_string(&path).expect("saved");
    let read = Settings::from_json(&text).expect("the file replaced");
    assert_eq!(read.places.get(Window::Run), Some(rect));
    assert_eq!(settings_lines(&app).len(), 1, "logged once");
    rejects("a corrupt file read as settings", || {
        Settings::from_json("{\"windows\": {\"Run\": [1, 2").unwrap();
    });
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_windows_remembered_no_place_reaches_the_contract() {
    let dir = scratch("contract");
    let path = dir.join(FILE);
    let mut app = gui(&path);
    let mut h = headless();
    open(&mut h, &mut app, Window::Profiler);
    settle(&mut h, &mut app);
    let before = app.snapshot().clone();
    let lines = app.console().len();
    let _ = move_and_resize(&mut h, &mut app, Window::Profiler);
    settle(&mut h, &mut app);
    let after = app.snapshot().clone();
    let reached = |a: &engine::contract::snapshot::Snapshot| {
        assert_eq!(a.sim, before.sim);
        assert_eq!(a.render, before.render);
        assert_eq!(a.history, before.history);
    };
    reached(&after);
    assert_eq!(app.console().len(), lines, "no SetField logged");
    assert!(path.exists());
    // A snapshot an edit did reach.
    app.set_field(playhead(1.5, false));
    settle(&mut h, &mut app);
    let edited = app.snapshot().clone();
    rejects("an edit that entered the contract", || reached(&edited));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_windows_remembered_the_file_reads_back_what_it_wrote() {
    let a = Rect::from_min_size(pos2(10.5, -20.0), vec2(300.0, 240.0));
    let b = Rect::from_min_size(pos2(900.0, 80.25), vec2(376.0, 500.0));
    let settings = Settings {
        places: Places::remembered([(Window::Inspector, a), (Window::Export, b)]),
        ..Settings::default()
    };
    let text = settings.to_json();
    assert!(
        text.contains("\"Inspector\"") && text.contains("\"Export & share\""),
        "{text}"
    );
    let read = Settings::from_json(&text).expect("read back");
    assert_eq!(read.places.rects().len(), 2);
    for window in [Window::Inspector, Window::Export] {
        assert_eq!(
            read.places.get(window),
            settings.places.get(window),
            "{window:?}"
        );
    }
    // A name the GUI does not have is skipped; no `windows` is no place.
    let other = r#"{"windows": {"Gone": {"x": 1, "y": 2, "w": 3, "h": 4}}, "layout": {}}"#;
    assert_eq!(Settings::from_json(other), Ok(Settings::default()));
    assert_eq!(Settings::from_json("{}"), Ok(Settings::default()));
    // What cannot be read.
    for bad in [
        "",
        "[]",
        r#"{"windows": []}"#,
        r#"{"windows": {"Run": {"x": 1, "y": 2, "w": 3}}}"#,
        r#"{"windows": {"Run": {"x": "1", "y": 2, "w": 3, "h": 4}}}"#,
        r#"{"windows": {"Run": {"x": 1, "y": 2, "w": 0, "h": 4}}}"#,
        r#"{"windows": {"Run": {"x": 1, "y": 2, "w": 3, "h": 0}}}"#,
        r#"{"windows": {"Run": {"x": 1, "y": 2, "w": 3, "h": -1}}}"#,
        r#"{"windows": {"Run": {"x": 1e300, "y": 2, "w": 3, "h": 4}}}"#,
        r#"{"windows": {"Gone": {"x": 1, "y": 2}}}"#,
    ] {
        assert!(Settings::from_json(bad).is_err(), "{bad}");
    }
    // A far edge past f32's range, across or down: refused, as any unreadable place.
    for bad in [
        r#"{"windows": {"Run": {"x": 3e38, "y": 0, "w": 3e38, "h": 4}}}"#,
        r#"{"windows": {"Run": {"x": 0, "y": 3e38, "w": 4, "h": 3e38}}}"#,
    ] {
        assert!(Settings::from_json(bad).is_err(), "{bad}");
    }
    let near = r#"{"windows": {"Run": {"x": 1e38, "y": 1e38, "w": 1e38, "h": 1e38}}}"#;
    let read = Settings::from_json(near).expect("a far edge inside f32's range");
    assert!(read.places.get(Window::Run).is_some_and(|r| r.is_finite()));
    rejects("a pair whose sum stays in range refused", || {
        assert!(Settings::from_json(near).is_err())
    });
    // A file with an overflowing place: the defaults, and the one console line.
    let dir = scratch("overflow");
    let path = dir.join(FILE);
    std::fs::write(
        &path,
        r#"{"windows": {"Run": {"x": 3e38, "y": 0, "w": 3e38, "h": 4}}}"#,
    )
    .expect("written");
    let app = gui(&path);
    assert_eq!(app.places, Places::default());
    assert_eq!(settings_lines(&app).len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
    // The least size read.
    let tiny = r#"{"windows": {"Run": {"x": 0, "y": 0, "w": 0.5, "h": 0.5}}}"#;
    let read = Settings::from_json(tiny).expect("read");
    assert_eq!(
        read.places.get(Window::Run).map(|r| r.size()),
        Some(vec2(0.5, 0.5))
    );
    rejects("a place with no size", || {
        Settings::from_json(r#"{"windows": {"Run": {"x": 1, "y": 2, "w": 0, "h": 0}}}"#).unwrap();
    });
}

#[test]
fn mock_windows_remembered_the_store_loads_and_saves() {
    let dir = scratch("store");
    // No store: the defaults, and a save that does nothing.
    let none = Store::default();
    assert_eq!(none.path(), None);
    assert_eq!(none.load().settings, Settings::default());
    assert_eq!(none.save(&Settings::default()), Ok(()));
    // A file in a directory not yet made: made on save.
    let path = dir.join("deeper").join(FILE);
    let store = Store::at(&path);
    assert_eq!(store.path(), Some(path.as_path()));
    let settings = Settings {
        places: Places::remembered([(
            Window::Run,
            Rect::from_min_size(pos2(1.0, 2.0), vec2(300.0, 200.0)),
        )]),
        ..Settings::default()
    };
    store.save(&settings).expect("saved");
    assert_eq!(store.load().settings, settings);
    assert_eq!(store.load().ignored, None);
    assert!(
        !path.with_extension("json.part").exists(),
        "the part file renamed"
    );
    // A file that is a directory cannot be read: ignored, with its line.
    let unreadable = Store::at(&dir);
    let loaded = unreadable.load();
    assert_eq!(loaded.settings, Settings::default());
    assert!(loaded.ignored.is_some());
    // A save under a file cannot be made: logged once a session.
    let blocked = dir.join("deeper").join(FILE).join(FILE);
    assert!(Store::at(&blocked).save(&settings).is_err());
    let mut app = gui(&blocked);
    app.places = settings.places.clone();
    let saves = |app: &App<MockSide>| -> Vec<String> {
        settings_lines(app)
            .into_iter()
            .filter(|l| l.contains("could not be saved"))
            .collect()
    };
    app.save_settings();
    assert_eq!(saves(&app).len(), 1, "logged at the first failure");
    app.save_settings();
    let lines = saves(&app);
    assert_eq!(lines.len(), 1, "{lines:?}");
    rejects("a save failure logged each time", || {
        assert_eq!(lines.len(), 2)
    });
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_windows_remembered_the_settings_file_is_under_the_platform_config_directory() {
    let path = platform_path().expect("HOME, XDG_CONFIG_HOME or APPDATA is set");
    assert!(path.ends_with(Path::new(DIR).join(FILE)), "{path:?}");
    if cfg!(target_os = "macos") {
        let home = PathBuf::from(std::env::var_os("HOME").expect("HOME"));
        assert_eq!(
            path,
            home.join("Library/Application Support")
                .join(DIR)
                .join(FILE)
        );
    }
    assert_eq!(Store::platform().path(), Some(path.as_path()));
    // The windows by their registry names: the file's keys.
    for window in MOVABLE {
        assert!(place::movable(window));
        assert!(!entry(window).name.is_empty());
    }
    rejects("a settings file in the working directory", || {
        assert!(Path::new(FILE).ends_with(Path::new(DIR).join(FILE)))
    });
}
