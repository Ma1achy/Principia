//! `mock_debug_view` (REQ-GUI-225; R-467 D5, D8; render_gui_spec §G2, §G6; gui_state_contract §2): View › Debug view
//! is a check entry after F3 hide, a registered scope of the View menu, so the keyboard reaches it and Enter acts as its
//! click; each toggle sends one `SetField DebugView`, undoable as an Overlays ▾ toggle is, re-integrating nothing, off at
//! launch and kept through F3 and present mode. With it off the outcome legend has no "forbidden region" key and no
//! "Debug" key; with it on the legend adds "Debug" after the stain's keys, its three entries each in its style with its
//! pixel count and share, the forbidden count the flagged region's pixels in the window. The mass-simplex analogue on
//! the mock: with the hook flagging a region past the shown rect and F3 off, the presentation capture is the clear
//! colour at exactly the flagged pixels and the debug-view capture the hatch at exactly those pixels, every other pixel
//! equal in both. Each check also runs on an input it must reject (R-176, through `rejects`).

use std::sync::Arc;

use eframe::egui::{Key, Modifiers};
use engine::contract::legend::{LegendKey, SwatchStyle};
use engine::contract::log::Source;
use engine::contract::set_field::{Edit, RenderField, SetField};

use super::f3_fallback_mock::{hatch_at, DEBUG_VIEW};
use super::f3_fills_window_mock::{canvas, clear, layout, pixel};
use super::support::{click, focus, frame_texts, headless, mock_app, press, rejects};
use crate::app::App;
use crate::capture::{render_frame, Shot, Step, FORMAT, PIXELS_PER_POINT, SIZE};
use crate::explore::top_bar::{debug_view_edit, DEBUG_VIEW as ENTRY, HIDE};
use crate::headless::Headless;
use crate::keyboard::scopes::{Scope, ScopeTree};
use crate::mock::canvas::{MockCanvas, NO_INVALID};
use crate::mock::invalid::covered_pixels;
use crate::mock::legend::{DEBUG, DEBUG_ENTRIES};
use crate::mock::MockEngine;
use crate::side::{FigureView, MockSide};

/// The SetField DebugView lines of the console, in order.
fn debug_lines(app: &App<MockSide>) -> Vec<String> {
    app.console()
        .iter()
        .filter(|e| e.source == Source::Contract && e.message.starts_with("SetField DebugView"))
        .map(|e| e.message.clone())
        .collect()
}

/// Checks that a toggle of the debug view from `before` sent exactly one `SetField DebugView before → !before`, the
/// undo depth one greater than `depth`, and the contract's state toggled, nothing of the sim key changed.
fn check_toggled(
    app: &App<MockSide>,
    before: bool,
    depth: u32,
    sim: &engine::contract::sim_config::SimConfig,
) {
    let snap = app.snapshot();
    assert_eq!(
        snap.render.debug_view, !before,
        "the debug view not toggled"
    );
    assert_eq!(
        debug_lines(app).last().map(String::as_str),
        Some(format!("SetField DebugView {before} → {}", !before).as_str())
    );
    assert_eq!(snap.history.undo_depth, depth + 1, "one undo entry");
    assert_eq!(&snap.sim, sim, "the sim key changed");
}

/// Runs `n` frames of `app`.
fn frames(h: &mut Headless, app: &mut App<MockSide>, n: usize) {
    for _ in 0..n {
        let _ = h.frame(app, Vec::new());
    }
}

/// View › Debug view, clicked: opens View, clicks the entry, then lets the snapshot catch up.
fn click_entry(h: &mut Headless, app: &mut App<MockSide>) {
    click(h, app, "View");
    click(h, app, ENTRY.1);
    frames(h, app, 2);
}

#[test]
fn mock_debug_view_clicked_is_one_undoable_set_field() {
    let mut app = mock_app();
    let mut h = headless();
    frames(&mut h, &mut app, 2);
    assert!(!app.snapshot().render.debug_view, "off at launch");
    let sim = app.snapshot().sim.clone();
    let depth = app.snapshot().history.undo_depth;
    click_entry(&mut h, &mut app);
    check_toggled(&app, false, depth, &sim);
    assert_eq!(debug_lines(&app).len(), 1, "one SetField per toggle");
    // Off again, a second entry.
    click_entry(&mut h, &mut app);
    check_toggled(&app, true, depth + 1, &sim);
    // Undo restores it: on, then off.
    press(&mut h, &mut app, Key::Z, Modifiers::CTRL);
    frames(&mut h, &mut app, 2);
    assert!(app.snapshot().render.debug_view);
    assert_eq!(app.snapshot().history.undo_depth, depth + 1);
    press(&mut h, &mut app, Key::Z, Modifiers::CTRL);
    frames(&mut h, &mut app, 2);
    assert!(!app.snapshot().render.debug_view);
    assert_eq!(app.snapshot().history.undo_depth, depth);
    // Control: a toggle marked "no history" adds no undo entry, and fails the check.
    let mut quiet = mock_app();
    let mut h = headless();
    frames(&mut h, &mut quiet, 2);
    quiet.set_field(SetField {
        edit: Edit::Render(RenderField::DebugView(true)),
        no_history: true,
    });
    frames(&mut h, &mut quiet, 2);
    rejects("a toggle that adds no undo entry", || {
        check_toggled(&quiet, false, depth, &sim)
    });
}

#[test]
fn mock_debug_view_entry_after_f3_hide_with_a_check_mark() {
    let mut tree = ScopeTree::new();
    crate::explore::top_bar::register(&mut tree);
    assert_eq!(tree.children(Some("view")), [HIDE.0, ENTRY.0]);
    assert!(tree.get(ENTRY.0).unwrap().activates, "Enter acts on it");
    // Drawn below F3 hide, as a check box, ticked as the contract holds it.
    let mut app = mock_app();
    let mut h = headless();
    frames(&mut h, &mut app, 2);
    click(&mut h, &mut app, "View");
    let place = |app: &App<MockSide>, id| app.keyboard.place_of(id).map(|p| p.rect);
    let (hide, entry) = (place(&app, HIDE.0).unwrap(), place(&app, ENTRY.0).unwrap());
    assert!(entry.min.y >= hide.max.y - 0.5, "{entry:?} after {hide:?}");
    let checked = |h: &mut Headless, app: &mut App<MockSide>| {
        super::support::nodes(h, app)
            .into_iter()
            .find(|n| n.text == ENTRY.1)
            .map(|n| n.role)
    };
    assert_eq!(
        checked(&mut h, &mut app),
        Some(eframe::egui::accesskit::Role::CheckBox),
        "a check entry"
    );
    rejects("the entry before F3 hide", || {
        assert!(hide.min.y >= entry.max.y - 0.5)
    });
}

/// Checks that the keyboard reaches View › Debug view in `tree`: its path is the top bar, View, the entry.
fn check_reached(tree: &ScopeTree) {
    assert_eq!(tree.path_to(ENTRY.0), ["top_bar", "view", ENTRY.0]);
}

#[test]
fn mock_debug_view_keyboard() {
    let mut app = mock_app();
    let mut h = headless();
    frames(&mut h, &mut app, 2);
    let sim = app.snapshot().sim.clone();
    let depth = app.snapshot().history.undo_depth;
    // Tab to the top bar, Enter, → to View, Enter: the menu open on F3 hide; ↓ to Debug view.
    for key in [
        Key::Tab,
        Key::Enter,
        Key::ArrowRight,
        Key::Enter,
        Key::ArrowDown,
    ] {
        press(&mut h, &mut app, key, Modifiers::NONE);
    }
    assert_eq!(focus(&app), ["top_bar", "view", ENTRY.0]);
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    frames(&mut h, &mut app, 2);
    check_toggled(&app, false, depth, &sim);
    assert_eq!(debug_lines(&app).len(), 1, "one SetField DebugView");
    press(&mut h, &mut app, Key::Z, Modifiers::CTRL);
    frames(&mut h, &mut app, 2);
    assert!(!app.snapshot().render.debug_view, "Ctrl+Z restores it");
    assert_eq!(app.snapshot().history.undo_depth, depth);
    check_reached(app.keyboard.tree(engine::contract::view_ui::Mode::Explore));
    // Control: a View menu whose scope tree does not hold the entry.
    let mut without = ScopeTree::new();
    without.register(None, Scope::group("top_bar", "Top bar"));
    without.register(Some("top_bar"), Scope::menu("view", "View"));
    without.register(Some("view"), Scope::control(HIDE.0, HIDE.1));
    rejects("an entry the scope tree does not hold", || {
        check_reached(&without)
    });
}

#[test]
fn mock_debug_view_kept_through_f3_and_present_mode() {
    let mut app = mock_app();
    let mut h = headless();
    frames(&mut h, &mut app, 2);
    app.set_field(debug_view_edit(true));
    frames(&mut h, &mut app, 2);
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    frames(&mut h, &mut app, 2);
    assert!(app.snapshot().render.debug_view, "F3 off");
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    app.present = true;
    frames(&mut h, &mut app, 2);
    assert!(app.snapshot().render.debug_view, "present mode");
    press(&mut h, &mut app, Key::Escape, Modifiers::NONE);
    frames(&mut h, &mut app, 2);
    assert!(!app.present);
    assert!(app.snapshot().render.debug_view, "back from present mode");
    let off = mock_app();
    rejects("the toggle lost through F3", || {
        assert!(off.snapshot().render.debug_view)
    });
}

// --- The legend ----------------------------------------------------------------------------------------------------

/// The keys' titles, in order.
fn titles(app: &App<MockSide>) -> Vec<String> {
    app.snapshot()
        .legend
        .keys
        .iter()
        .map(|k| match k {
            LegendKey::Categorical { title, .. } | LegendKey::Ramp { title, .. } => title.clone(),
        })
        .collect()
}

/// The "Debug" key's entries: label, style, count and share.
fn debug_entries(app: &App<MockSide>) -> Vec<(String, SwatchStyle, Option<u64>, f64)> {
    let Some(LegendKey::Categorical { title, entries }) = app.snapshot().legend.keys.last() else {
        panic!("no categorical key last")
    };
    assert_eq!(title, DEBUG, "the last key");
    entries
        .iter()
        .map(|e| (e.label.clone(), e.swatch, e.count, e.share))
        .collect()
}

/// Checks the "Debug" key: its three entries in order, each in its style, the forbidden count `forbidden` of the
/// `figure` pixels, the other two 0, each with its share of the figure's pixels.
fn check_debug_key(app: &App<MockSide>, forbidden: u64, figure: u64) {
    let entries = debug_entries(app);
    let labels: Vec<&str> = entries.iter().map(|e| e.0.as_str()).collect();
    assert_eq!(labels, DEBUG_ENTRIES);
    assert_eq!(
        DEBUG_ENTRIES,
        ["forbidden (chart)", "decode or sim failure", "NaN / Inf"]
    );
    let styles: Vec<SwatchStyle> = entries.iter().map(|e| e.1).collect();
    assert_eq!(
        styles,
        [SwatchStyle::Hatch, SwatchStyle::Hatch, SwatchStyle::NanInf]
    );
    let counts: Vec<Option<u64>> = entries.iter().map(|e| e.2).collect();
    assert_eq!(counts, [Some(forbidden), Some(0), Some(0)]);
    assert!((entries[0].3 - 100.0 * forbidden as f64 / figure as f64).abs() < 1e-9);
    assert_eq!((entries[1].3, entries[2].3), (0.0, 0.0));
}

/// An app on a frozen mock drawing through `canvas`.
fn app_on(canvas: &Arc<MockCanvas>) -> (App<MockSide>, Headless) {
    let mut app = App::new(
        MockSide::new(MockEngine::frozen(), Some(canvas.clone())),
        FORMAT,
    );
    let mut h = headless();
    frames(&mut h, &mut app, 2);
    (app, h)
}

/// The figure with the layer hidden, F3 off, as the app draws it in the capture.
fn hidden_view() -> FigureView {
    FigureView::of(
        &layout(),
        false,
        PIXELS_PER_POINT,
        crate::mock::canvas::default_background(),
    )
}

/// Left of the shown rect and reaching a quarter into it: D8's example band, widened past `s = 0`.
const BAND: [f32; 4] = [-0.5, -0.5, 0.25, 1.5];

#[test]
fn mock_debug_view_legend_keys() {
    let canvas = canvas();
    let (mut app, mut h) = app_on(&canvas);
    // Off: the stain's keys only, no "forbidden region" and no "Debug".
    let stain = titles(&app);
    assert_eq!(
        stain,
        ["colour · outcome class", "brightness · t_end (late fade)"]
    );
    let texts = frame_texts(&mut h, &mut app);
    assert!(!texts.iter().any(|t| t == "forbidden region" || t == DEBUG));
    // On, nothing flagged: the "Debug" key after the stain's, every count 0.
    app.set_field(debug_view_edit(true));
    frames(&mut h, &mut app, 2);
    let mut want = stain.clone();
    want.push(DEBUG.to_owned());
    assert_eq!(titles(&app), want);
    let shown = FigureView::of(
        &layout(),
        true,
        PIXELS_PER_POINT,
        crate::mock::canvas::default_background(),
    );
    let figure =
        |v: &FigureView| u64::from(v.clip[2] - v.clip[0]) * u64::from(v.clip[3] - v.clip[1]);
    check_debug_key(&app, 0, figure(&shown));
    // Flagged, F3 off: the count is the flagged pixels of the window.
    canvas.flag_invalid(BAND);
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    frames(&mut h, &mut app, 2);
    let view = hidden_view();
    let (columns, rows) = covered_pixels(&view, BAND);
    let n = (columns.len() * rows.len()) as u64;
    assert!(n > 100_000, "{n}");
    check_debug_key(&app, n, figure(&view));
    assert_eq!(figure(&view), u64::from(SIZE[0]) * u64::from(SIZE[1]));
    // The layer shown: only the part inside the shown rect is the figure's.
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    frames(&mut h, &mut app, 2);
    let (columns, rows) = covered_pixels(&shown, BAND);
    let inside = (columns.len() * rows.len()) as u64;
    assert!(inside > 0 && inside < n);
    check_debug_key(&app, inside, figure(&shown));
    // The GUI draws the key, each entry with its count and share.
    let texts = frame_texts(&mut h, &mut app);
    for label in DEBUG_ENTRIES {
        assert!(texts.iter().any(|t| t == label), "{label}: {texts:?}");
    }
    let share = 100.0 * inside as f64 / figure(&shown) as f64;
    let value = format!("{} · {share:.1}", crate::explore::top_bar::grouped(inside));
    assert!(texts.contains(&value), "{value}: {texts:?}");
    // Controls: a legend that keeps the forbidden key with the toggle off, and a count of another region, fail.
    canvas.flag_invalid(NO_INVALID);
    rejects("a forbidden count that is not the flagged region's", || {
        check_debug_key(&app, n, figure(&shown))
    });
    let mut kept = stain.clone();
    kept.push("forbidden region".to_owned());
    let (off, _) = app_on(&canvas);
    rejects("a legend that keeps the forbidden key", || {
        assert_eq!(titles(&off), kept)
    });
}

/// Checks that the flagged pixels, those whose column is in `columns` and row in `rows`, are the clear colour in
/// `presented` and the hatch in `debugged`, and that every other pixel is the same in both.
fn check_exactly(presented: &[u8], debugged: &[u8], (columns, rows): (&[u32], &[u32])) {
    let [w, h] = SIZE.map(|v| v as usize);
    let mut column = vec![false; w];
    let mut row = vec![false; h];
    columns.iter().for_each(|&c| column[c as usize] = true);
    rows.iter().for_each(|&r| row[r as usize] = true);
    for (y, &in_row) in row.iter().enumerate() {
        for (x, &in_column) in column.iter().enumerate() {
            let (p, d) = (pixel(presented, x, y), pixel(debugged, x, y));
            if in_column && in_row {
                assert_eq!(p, clear(), "presentation at ({x}, {y})");
                assert_eq!(d, hatch_at(x, y), "debug view at ({x}, {y})");
            } else {
                assert_eq!(p, d, "({x}, {y}) differs");
            }
        }
    }
}

/// The capture of `app` after `steps`, as the capture mode renders it.
fn take(canvas: &Arc<MockCanvas>, steps: &[Step]) -> Shot {
    let (mut app, mut h) = app_on(canvas);
    for step in steps {
        let key = match step {
            Step::F3 => Key::F3,
            Step::Tab => Key::Tab,
            Step::Enter => Key::Enter,
            Step::ArrowRight => Key::ArrowRight,
            Step::ArrowDown => Key::ArrowDown,
            other => panic!("no key for {other:?}"),
        };
        press(&mut h, &mut app, key, Modifiers::NONE);
    }
    frames(&mut h, &mut app, 2);
    render_frame(canvas, &mut app, &mut h)
}

/// The mass-simplex analogue on the mock (R-467 D12, carried to the hook's region): F3 off, the region flagged past
/// the shown rect and into it, the presentation capture is the clear colour at exactly the flagged pixels and the debug
/// view's capture the hatch at exactly those pixels, and every other pixel is equal in both.
#[test]
fn mock_debug_view_mass_simplex_analogue() {
    let canvas = canvas();
    canvas.flag_invalid(BAND);
    let presented = take(&canvas, &[Step::F3]);
    let mut steps = DEBUG_VIEW.to_vec();
    steps.push(Step::F3);
    let debugged = take(&canvas, &steps);
    canvas.flag_invalid(NO_INVALID);
    let (columns, rows) = covered_pixels(&hidden_view(), BAND);
    assert!(!columns.is_empty() && !rows.is_empty());
    check_exactly(&presented.rgba, &debugged.rgba, (&columns, &rows));
    // Controls: a presentation capture that draws the hatch, and a debug view that hatches one pixel off, fail.
    rejects("a presentation capture that draws the hatch", || {
        check_exactly(&debugged.rgba, &debugged.rgba, (&columns, &rows))
    });
    let shifted: Vec<u32> = columns.iter().map(|c| c + 1).collect();
    rejects("a debug view that hatches one pixel off", || {
        check_exactly(&presented.rgba, &debugged.rgba, (&shifted, &rows))
    });
}

/// Every share in the "Debug" key is `want`.
fn check_shares(key: &LegendKey, want: f64) {
    let LegendKey::Categorical { entries, .. } = key else {
        panic!("a categorical key")
    };
    for e in entries.iter().take(1) {
        assert_eq!(e.share, want, "{}'s share", e.label);
    }
    for e in entries.iter().skip(1) {
        assert_eq!(e.share, 0.0, "{}'s share", e.label);
    }
}

#[test]
fn mock_debug_view_shares_of_an_empty_figure() {
    use crate::mock::invalid::Flagged;
    use crate::mock::legend::debug_key;
    // No figure, before the first frame: every share 0, not 0 / 0.
    check_shares(&debug_key(&Flagged::default()), 0.0);
    // A figure of one pixel, flagged: its whole.
    let one = Flagged {
        figure: 1,
        flagged: 1,
        inside: None,
    };
    check_shares(&debug_key(&one), 100.0);
    rejects("an empty figure's share as 0 / 0", || {
        let mut key = debug_key(&Flagged::default());
        if let LegendKey::Categorical { entries, .. } = &mut key {
            entries[0].share = f64::NAN;
        }
        check_shares(&key, 0.0);
    });
}
