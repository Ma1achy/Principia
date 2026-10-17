//! REQ-GUI-190's audit (R-450 A7) under R-456, which amends R-450 A3 (render_gui_spec §G3; R-409 G4, G5): a click on a
//! text input starts typing in it, on every page and in every window, and a click anywhere else lands in navigation
//! mode. Every text field in each mode's scope tree is found from the tree itself (the console's search field, the
//! Stain library's filter and the Run window's fields), drawn, and clicked: the keyboard's focus is on it in
//! interaction mode, egui's text focus on its widget, and a letter types; Tab then leaves it for the next scope in
//! navigation mode, the text kept, and Esc stops typing on the field. Each check also runs on an input it must reject
//! (R-176, through `rejects`). The tests' names carry `mock_reachability`, the audit's acceptance filter.

use eframe::egui::{Event, Key, Modifiers};
use engine::contract::view_ui::{KeyMode, Mode};

use super::support::{click_scope, headless, mock_app, press, rejects};
use crate::app::App;
use crate::headless::Headless;
use crate::keyboard::scopes::{ScopeId, ScopeTree};
use crate::side::MockSide;
use crate::windows::{self, console, run};

const NONE: Modifiers = Modifiers::NONE;

/// Every text field in `tree`, depth first in registration order.
fn text_fields(tree: &ScopeTree) -> Vec<ScopeId> {
    let mut found = Vec::new();
    let mut stack: Vec<ScopeId> = tree.children(None).into_iter().rev().collect();
    while let Some(id) = stack.pop() {
        if tree.get(id).is_some_and(|s| s.text) {
            found.push(id);
        }
        stack.extend(tree.children(Some(id)).into_iter().rev());
    }
    found
}

/// The text of field `id`, from the state that owns it.
fn text_of(app: &App<MockSide>, id: &str) -> String {
    if id == console::SEARCH {
        return app.windows.console.text.clone();
    }
    if id == crate::stain::FILTER {
        return app.stain.library.filter.clone();
    }
    let i = run::FIELDS
        .iter()
        .chain([&run::CEILING])
        .position(|f| f.id == id)
        .unwrap_or_else(|| panic!("no state for the text field `{id}`"));
    app.windows.run.text[i].clone()
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

/// Types `c` as a keyboard does: its key's press with its text, then the release.
fn type_char(h: &mut Headless, app: &mut App<MockSide>, c: char) {
    let key = Key::from_name(&c.to_uppercase().to_string()).expect("a letter key");
    let event = |pressed| Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: NONE,
    };
    let _ = h.frame(app, vec![event(true), Event::Text(c.to_string())]);
    let _ = h.frame(app, vec![event(false)]);
}

/// The app in `mode` with field `id` drawn on screen: its window opened, and the window scrolled to it (the focus on
/// it, in navigation mode, then off it again, so the click is what moves it).
fn with_field_drawn(mode: Mode, id: ScopeId) -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    app.view.mode = mode;
    settle(&mut h, &mut app);
    let tree = app.keyboard.tree(mode);
    let path = tree.path_to(id);
    if let Some(window) = tree.window_of(&path) {
        let e = windows::REGISTRY
            .iter()
            .find(|e| e.scope == window)
            .expect("a registered window");
        let ctx = h.ctx.clone();
        app.open_window(&ctx, e.window, None);
        settle(&mut h, &mut app);
    }
    app.view.focus.path = path;
    settle(&mut h, &mut app);
    app.view.focus.path.clear();
    settle(&mut h, &mut app);
    (app, h)
}

/// Whether egui's text focus is on field `id`'s widget.
fn egui_holds(h: &Headless, app: &App<MockSide>, id: &str) -> bool {
    let widget = app.keyboard.place_of(id).and_then(|p| p.widget);
    widget.is_some() && h.ctx.memory(|m| m.focused()) == widget
}

/// Asserts that Tab left the field at `path`: the focus elsewhere, in navigation mode (§G3, R-409 G5).
fn check_left(left: &(Vec<String>, KeyMode), path: &[String], what: &str) {
    assert_ne!(left.0, path, "{what}: Tab left the field");
    assert_eq!(left.1, KeyMode::Navigation, "{what}: Tab's landing");
}

/// The text fields of `mode`'s tree once a frame of it is drawn (the Stain library's filter joins it on its page).
fn fields_of(mode: Mode) -> Vec<ScopeId> {
    let mut app = mock_app();
    let mut h = headless();
    app.view.mode = mode;
    settle(&mut h, &mut app);
    text_fields(app.keyboard.tree(mode))
}

/// Asserts that `found`, `mode`'s text fields, holds every text input drawn on that page and in its windows.
fn check_found(found: &[ScopeId], mode: Mode) {
    assert!(found.contains(&console::SEARCH), "{found:?}");
    for f in run::FIELDS.iter().chain([&run::CEILING]) {
        assert!(found.contains(&f.id), "{} missing from {found:?}", f.id);
    }
    let filter = found.contains(&crate::stain::FILTER);
    assert_eq!(filter, mode == Mode::Stain, "the filter on {mode:?}");
}

#[test]
fn mock_reachability_text_click_finds_every_text_field() {
    for mode in [Mode::Explore, Mode::Stain] {
        check_found(&fields_of(mode), mode);
    }
    rejects("a walk of the top level alone", || {
        let tree = mock_app();
        let tree = tree.keyboard.tree(Mode::Stain);
        let top: Vec<ScopeId> = tree
            .children(None)
            .into_iter()
            .filter(|id| tree.get(id).is_some_and(|s| s.text))
            .collect();
        check_found(&top, Mode::Stain)
    });
    rejects("the Explore page's fields read as the Stain page's", || {
        check_found(&fields_of(Mode::Explore), Mode::Stain)
    });
}

#[test]
fn mock_reachability_text_click_every_field_types_on_a_click_and_tab_leaves_it() {
    for mode in [Mode::Explore, Mode::Stain] {
        for id in fields_of(mode) {
            let what = format!("{id} on {mode:?}");
            let (mut app, mut h) = with_field_drawn(mode, id);
            let before = text_of(&app, id);
            click_scope(&mut h, &mut app, id);
            let path = app.keyboard.tree(mode).path_to(id);
            let got = (app.view.focus.path.clone(), app.view.focus.mode);
            assert_eq!(got, (path.clone(), KeyMode::Interaction), "{what}: a click");
            rejects("a click landing in navigation mode on a text field", || {
                assert_eq!(got, (path.clone(), KeyMode::Navigation))
            });
            assert!(egui_holds(&h, &app, id), "{what}: egui's focus");
            type_char(&mut h, &mut app, 'q');
            let typed = text_of(&app, id);
            assert_eq!(typed, format!("{before}q"), "{what}: typed at once");
            rejects("a field that types nothing", || assert_eq!(typed, before));
            // Tab leaves it, in navigation mode, the text kept; the next letter is not the field's.
            press(&mut h, &mut app, Key::Tab, NONE);
            let left = (app.view.focus.path.clone(), app.view.focus.mode);
            check_left(&left, &path, &what);
            assert!(!egui_holds(&h, &app, id), "{what}: egui kept the focus");
            type_char(&mut h, &mut app, 'r');
            assert_eq!(text_of(&app, id), typed, "{what}: typed after Tab");
            rejects("a Tab the field took", || {
                check_left(&(path.clone(), KeyMode::Interaction), &path, "control")
            });
        }
    }
}

#[test]
fn mock_reachability_text_click_esc_stops_typing_on_the_field() {
    let (mut app, mut h) = with_field_drawn(Mode::Stain, crate::stain::FILTER);
    click_scope(&mut h, &mut app, crate::stain::FILTER);
    type_char(&mut h, &mut app, 'a');
    press(&mut h, &mut app, Key::Escape, NONE);
    let path = app.keyboard.tree(Mode::Stain).path_to(crate::stain::FILTER);
    let got = (app.view.focus.path.clone(), app.view.focus.mode);
    assert_eq!(got, (path.clone(), KeyMode::Navigation), "Esc while typing");
    rejects("an Esc that went up a scope", || {
        assert_eq!(got.0, path[..1].to_vec())
    });
    assert_eq!(app.stain.library.filter, "a", "Esc kept the text");
    assert!(app.keyboard.typing().is_none(), "still typing after Esc");
}
