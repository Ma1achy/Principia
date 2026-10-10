//! `mock_keyboard_modes` (REQ-GUI-182; render_gui_spec §G3, "Two modes: navigation and interaction"; R-409): every
//! landing is in navigation mode, a Tab on the big scope itself and Enter into a scope on its first element; in
//! navigation mode the arrows move between siblings, on a big scope between the big scopes; Enter on an element starts
//! interaction mode, on the Figure too, on a button acts as its click, and on the Legend does nothing; in interaction
//! mode the element takes the arrows while Tab, Ctrl+Z, `?` and the shortcuts keep their meaning; Esc or Enter returns
//! to navigation mode on the same element, and Esc in navigation mode goes up a scope; a text field takes every key but
//! Esc, Enter and Tab while typed in; and the ring is orange in navigation mode and blue in interaction mode. Each check
//! also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, Event, Key, Modifiers};
use engine::contract::view_ui::{Focus, KeyMode, Mode};

use super::support::{focus, frame_texts, headless, mock_app, press, rejects, shapes};
use crate::app::App;
use crate::explore::breadcrumb::{self, NAVIGATION, RING_WIDTH};
use crate::explore::manifold_view::{CENTRE_Z0, SLIDERS};
use crate::explore::{COMPASS, COMPASS_TILT, COMPASS_VIEW, FIGURE, MANIFOLD_VIEW};
use crate::headless::Headless;
use crate::keyboard::keymap::{Command, Direction};
use crate::keyboard::scopes::{Outcome, Scope, ScopeTree, StepKind};
use crate::keyboard::Keyboard;
use crate::layout::Layout;
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;

fn keys(h: &mut Headless, app: &mut App<MockSide>, keys: &[Key]) {
    for key in keys {
        press(h, app, *key, NONE);
    }
}

/// A focus path and its mode, owned.
type At = (Vec<String>, KeyMode);

/// `path` in `mode`.
fn st(path: &[&str], mode: KeyMode) -> At {
    (path.iter().map(|s| (*s).to_owned()).collect(), mode)
}

/// The focus and its mode.
fn at(app: &App<MockSide>) -> At {
    st(&focus(app), app.view.focus.mode)
}

/// Asserts the focus is `path`, in `mode`.
fn check_at(got: At, path: &[&str], mode: KeyMode, what: &str) {
    assert_eq!(got, st(path, mode), "{what}");
}

/// The app after one frame, its plane read.
fn app() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

#[test]
fn mock_keyboard_modes_a_tab_lands_on_the_big_scope_and_enter_on_its_first_element() {
    let (mut app, mut h) = app();
    keys(&mut h, &mut app, &[Key::Tab, Key::Tab]);
    let landed = at(&app);
    check_at(
        landed.clone(),
        &[MANIFOLD_VIEW],
        NAV,
        "Tab onto Manifold view",
    );
    rejects("a Tab that lands on a big scope's first element", || {
        check_at(
            st(&[MANIFOLD_VIEW, "chart"], NAV),
            &[MANIFOLD_VIEW],
            NAV,
            "Tab",
        )
    });
    // On a big scope the arrows move between the big scopes, in Tab order.
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    check_at(at(&app), &[FIGURE], NAV, "→ to the Figure");
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    check_at(at(&app), &[MANIFOLD_VIEW], NAV, "← back");
    rejects("arrows on a big scope that do nothing", || {
        check_at(landed.clone(), &[FIGURE], NAV, "→")
    });
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(at(&app), &[MANIFOLD_VIEW, "chart"], NAV, "Enter onto Chart");
    rejects(
        "an Enter into a scope that lands in interaction mode",
        || {
            check_at(
                st(&[MANIFOLD_VIEW, "chart"], INTERACT),
                &[MANIFOLD_VIEW, "chart"],
                NAV,
                "Enter",
            )
        },
    );
    // The Compass scope: a Tab lands on it, Enter onto the compass, still navigating (R-409 A2, F4).
    keys(&mut h, &mut app, &[Key::Tab, Key::Tab, Key::Tab]);
    check_at(at(&app), &[COMPASS], NAV, "Tab onto the Compass");
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(
        at(&app),
        &[COMPASS, COMPASS_VIEW],
        NAV,
        "Enter onto the compass",
    );
}

/// The value of Centre z₀'s slider `i`.
fn slider(app: &App<MockSide>, i: usize) -> f64 {
    app.snapshot().sim.plane.z0[i]
}

#[test]
fn mock_keyboard_modes_a_slider_needs_enter_and_esc_comes_back_out() {
    let (mut app, mut h) = app();
    keys(
        &mut h,
        &mut app,
        &[
            Key::Tab,
            Key::Tab,
            Key::Enter,
            Key::ArrowDown,
            Key::ArrowDown,
            Key::Enter,
        ],
    );
    let first = [MANIFOLD_VIEW, CENTRE_Z0, SLIDERS[0]];
    let second = [MANIFOLD_VIEW, CENTRE_Z0, SLIDERS[1]];
    check_at(at(&app), &first, NAV, "Enter on Centre z₀");
    let values = (slider(&app, 0), slider(&app, 1));
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    let _ = h.frame(&mut app, Vec::new());
    check_at(at(&app), &second, NAV, "↓ to the second slider");
    assert_eq!((slider(&app, 0), slider(&app, 1)), values, "no value moved");
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(at(&app), &second, INTERACT, "Enter starts interaction");
    press(&mut h, &mut app, Key::ArrowUp, NONE);
    let _ = h.frame(&mut app, Vec::new());
    // A hundredth of the range, 2 (R-404).
    assert!((slider(&app, 1) - (values.1 + 0.02)).abs() < 1e-12);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert!((slider(&app, 1) - (values.1 - 0.02)).abs() < 1e-12);
    check_at(at(&app), &second, INTERACT, "↓ adjusts, the focus kept");
    let interacting = at(&app);
    rejects(
        "the pre-R-409 browsing model: arrows adjusting a value reached by Enter on its section",
        || {
            check_at(
                interacting.clone(),
                &second,
                NAV,
                "↓ after Enter on its section",
            )
        },
    );
    press(&mut h, &mut app, Key::Escape, NONE);
    let out = at(&app);
    check_at(
        out.clone(),
        &second,
        NAV,
        "Esc back to navigation on the slider",
    );
    rejects("an Esc in interaction mode that goes up a scope", || {
        check_at(st(&[MANIFOLD_VIEW, CENTRE_Z0], NAV), &second, NAV, "Esc")
    });
    press(&mut h, &mut app, Key::Escape, NONE);
    check_at(
        at(&app),
        &[MANIFOLD_VIEW, CENTRE_Z0],
        NAV,
        "a second Esc goes up",
    );
    // Enter on the slider again, and Enter ends the interaction as Esc does.
    keys(&mut h, &mut app, &[Key::Enter, Key::Enter]);
    check_at(at(&app), &first, INTERACT, "interacting again");
    press(&mut h, &mut app, Key::Enter, NONE);
    let back = at(&app);
    check_at(back.clone(), &first, NAV, "Enter back to navigation");
    rejects("an Enter in interaction mode that stays in it", || {
        check_at(st(&first, INTERACT), &first, NAV, "Enter")
    });
}

/// What `Ctrl+Z` and `?` do with the focus on Centre z₀'s first slider in `mode`: the focus and mode after Ctrl+Z,
/// the undo depth's change, whether `?` opened the overlay, and the focus and mode after Esc closes it.
type GlobalKeys = ((Vec<String>, KeyMode), u32, bool, (Vec<String>, KeyMode));

fn global_keys(mode: KeyMode) -> GlobalKeys {
    let (mut app, mut h) = app();
    keys(
        &mut h,
        &mut app,
        &[
            Key::Tab,
            Key::Tab,
            Key::Enter,
            Key::ArrowDown,
            Key::ArrowDown,
            Key::Enter,
        ],
    );
    if mode == INTERACT {
        press(&mut h, &mut app, Key::Enter, NONE);
    }
    assert_eq!(app.view.focus.mode, mode);
    let owned = |app: &App<MockSide>| {
        (
            focus(app).iter().map(|s| (*s).to_owned()).collect(),
            app.view.focus.mode,
        )
    };
    let depth = app.snapshot().history.undo_depth;
    press(&mut h, &mut app, Key::Z, Modifiers::CTRL);
    let _ = h.frame(&mut app, Vec::new());
    let after_undo = owned(&app);
    let undone = app.snapshot().history.undo_depth - depth;
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    let overlay = app.keyboard.shortcuts_open;
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(!app.keyboard.shortcuts_open);
    (after_undo, undone, overlay, owned(&app))
}

#[test]
fn mock_keyboard_modes_tab_ctrl_z_and_question_keep_their_meaning_in_interaction_mode() {
    let navigating = global_keys(NAV);
    let interacting = global_keys(INTERACT);
    let path: Vec<String> = [MANIFOLD_VIEW, CENTRE_Z0, SLIDERS[0]]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert_eq!(
        navigating,
        ((path.clone(), NAV), 0, true, (path.clone(), NAV))
    );
    assert_eq!(
        interacting,
        ((path.clone(), INTERACT), 0, true, (path.clone(), INTERACT)),
        "Ctrl+Z and ? act as in navigation mode, the element kept"
    );
    rejects("a ? that the element took", || {
        assert_eq!(
            ((path.clone(), INTERACT), 0, false, (path.clone(), INTERACT)),
            interacting
        )
    });
    // Tab from interaction mode lands on the next big scope, in navigation mode.
    let (mut app, mut h) = app();
    keys(
        &mut h,
        &mut app,
        &[
            Key::Tab,
            Key::Tab,
            Key::Enter,
            Key::ArrowDown,
            Key::ArrowDown,
            Key::Enter,
            Key::Enter,
        ],
    );
    let value = slider(&app, 0);
    press(&mut h, &mut app, Key::Tab, NONE);
    let tabbed = at(&app);
    check_at(tabbed.clone(), &[FIGURE], NAV, "Tab out of interaction");
    assert_eq!(slider(&app, 0), value, "the Tab adjusted nothing");
    rejects("a Tab in interaction mode captured by the element", || {
        check_at(
            st(&[MANIFOLD_VIEW, CENTRE_Z0, SLIDERS[0]], INTERACT),
            &[FIGURE],
            NAV,
            "Tab",
        )
    });
    // Shift+Tab likewise, to the previous big scope.
    keys(&mut h, &mut app, &[Key::Enter]);
    check_at(at(&app), &[FIGURE], INTERACT, "Enter on the Figure");
    press(&mut h, &mut app, Key::Tab, SHIFT);
    check_at(
        at(&app),
        &[MANIFOLD_VIEW],
        NAV,
        "Shift+Tab out of interaction",
    );
}

/// The Figure's shortcuts in `mode`, the pointer over the figure: whether `+`, `−` and `K` each acted.
fn figure_shortcuts(mode: KeyMode) -> (bool, bool, bool) {
    let (mut app, mut h) = app();
    keys(&mut h, &mut app, &[Key::Tab, Key::Tab, Key::Tab]);
    if mode == INTERACT {
        press(&mut h, &mut app, Key::Enter, NONE);
    }
    check_at(at(&app), &[FIGURE], mode, "the Figure");
    let figure = Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT).figure;
    let _ = h.frame(&mut app, vec![Event::PointerMoved(figure.center())]);
    let q1 = |app: &App<MockSide>| app.snapshot().sim.plane.q1;
    let before = q1(&app);
    press(&mut h, &mut app, Key::Plus, NONE);
    let _ = h.frame(&mut app, Vec::new());
    let zoomed_in = q1(&app)[0] < before[0];
    let mid = q1(&app);
    press(&mut h, &mut app, Key::Minus, NONE);
    let _ = h.frame(&mut app, Vec::new());
    let zoomed_out = q1(&app)[0] > mid[0];
    press(&mut h, &mut app, Key::K, NONE);
    let _ = h.frame(&mut app, Vec::new());
    let locked = app.snapshot().sim.lock.locked;
    check_at(at(&app), &[FIGURE], mode, "the shortcuts move no focus");
    (zoomed_in, zoomed_out, locked)
}

#[test]
fn mock_keyboard_modes_the_figure_shortcuts_act_in_either_mode_and_its_arrows_need_enter() {
    let both = (figure_shortcuts(NAV), figure_shortcuts(INTERACT));
    let check = |(nav, interact): ((bool, bool, bool), (bool, bool, bool))| {
        assert_eq!(nav, (true, true, true), "+, − and K in navigation mode");
        assert_eq!(
            interact,
            (true, true, true),
            "+, − and K in interaction mode"
        );
    };
    check(both);
    rejects(
        "a Figure shortcut (+, −, K) that acts in one mode only",
        || check(((true, true, true), (true, false, true))),
    );
    // In navigation mode the arrows on the Figure move to the next big scope and emit no SetField; after Enter they
    // pan.
    let (mut app, mut h) = app();
    keys(&mut h, &mut app, &[Key::Tab, Key::Tab, Key::Tab]);
    let depth = app.snapshot().history.undo_depth;
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    let _ = h.frame(&mut app, Vec::new());
    check_at(at(&app), &["trajectory"], NAV, "↓ on the Figure moves on");
    assert_eq!(app.snapshot().history.undo_depth, depth, "no SetField");
    keys(&mut h, &mut app, &[Key::ArrowUp, Key::Enter]);
    let landed = at(&app);
    check_at(landed.clone(), &[FIGURE], INTERACT, "Enter on the Figure");
    rejects("Enter on the Figure landing in navigation mode", || {
        check_at(landed.clone(), &[FIGURE], NAV, "Enter")
    });
    let z0 = app.snapshot().sim.plane.z0;
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    let _ = h.frame(&mut app, Vec::new());
    let panned = app.snapshot().sim.plane.z0;
    assert!(
        (panned[0] - (z0[0] + 0.1)).abs() < 1e-12,
        "→ pans along q₁ by a twentieth: {panned:?}"
    );
    check_at(at(&app), &[FIGURE], INTERACT, "the pan moves no focus");
}

#[test]
fn mock_keyboard_modes_enter_on_the_legend_does_nothing_and_on_a_button_clicks() {
    let (mut app, mut h) = app();
    // Legend, big scope 7: Shift+Tab from no focus lands on the last big scope.
    press(&mut h, &mut app, Key::Tab, SHIFT);
    check_at(at(&app), &["legend"], NAV, "the Legend");
    press(&mut h, &mut app, Key::Enter, NONE);
    let after = at(&app);
    check_at(
        after.clone(),
        &["legend"],
        NAV,
        "Enter on the Legend changes nothing",
    );
    rejects("Enter on the Legend starting interaction mode", || {
        check_at(st(&["legend"], INTERACT), &["legend"], NAV, "Enter")
    });
    // Enter on Tilt: its click, in navigation mode.
    app.manifold.compass.mode = crate::explore::manifold_view::Touch::Slice;
    keys(
        &mut h,
        &mut app,
        &[
            Key::Tab,
            Key::Tab,
            Key::Tab,
            Key::Tab,
            Key::Tab,
            Key::Enter,
            Key::ArrowRight,
        ],
    );
    check_at(at(&app), &[COMPASS, COMPASS_TILT], NAV, "on Tilt");
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(
        at(&app),
        &[COMPASS, COMPASS_TILT],
        NAV,
        "Enter on Tilt stays navigating",
    );
    assert_eq!(
        app.manifold.compass.mode,
        crate::explore::manifold_view::Touch::Tilt
    );
    // A menu entry: Help › Keys (?) opens the shortcuts, the focus back on Help, navigating.
    keys(
        &mut h,
        &mut app,
        &[Key::Tab, Key::Tab, Key::Tab, Key::Enter],
    );
    for _ in 0..7 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    keys(&mut h, &mut app, &[Key::Enter, Key::Enter]);
    assert!(app.keyboard.shortcuts_open, "Keys (?) acted");
    check_at(at(&app), &["top_bar", "help"], NAV, "after the entry");
}

/// The focus ring's colours drawn in one more frame: the stroke colours of the rects `RING_WIDTH` wide on the ring's
/// layer.
fn ring_colours(h: &mut Headless, app: &mut App<MockSide>) -> Vec<egui::Color32> {
    shapes(h, app)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Rect(r) if r.stroke.width == RING_WIDTH => Some(r.stroke.color),
            _ => None,
        })
        .collect()
}

/// Asserts the ring is drawn once, in `want`.
fn check_ring(colours: &[egui::Color32], want: egui::Color32) {
    assert_eq!(colours, [want], "one ring, in {want:?}");
}

#[test]
fn mock_keyboard_modes_the_ring_is_orange_navigating_and_blue_interacting() {
    let (mut app, mut h) = app();
    let blue = egui::Visuals::dark().hyperlink_color;
    assert_ne!(NAVIGATION, blue);
    keys(
        &mut h,
        &mut app,
        &[
            Key::Tab,
            Key::Tab,
            Key::Enter,
            Key::ArrowDown,
            Key::ArrowDown,
            Key::Enter,
        ],
    );
    let navigating = ring_colours(&mut h, &mut app);
    check_ring(&navigating, NAVIGATION);
    press(&mut h, &mut app, Key::Enter, NONE);
    let interacting = ring_colours(&mut h, &mut app);
    check_ring(&interacting, blue);
    rejects("a ring of one colour in both modes", || {
        check_ring(&navigating, blue)
    });
    // The colours as the breadcrumb module gives them, the breadcrumb's the ring's (R-409 A6).
    let v = egui::Visuals::dark();
    assert_eq!(breadcrumb::colour(&v, NAV), NAVIGATION);
    assert_eq!(breadcrumb::colour(&v, INTERACT), v.hyperlink_color);
    for mode in [NAV, INTERACT] {
        assert_eq!(
            breadcrumb::breadcrumb_colour(&v, mode),
            breadcrumb::colour(&v, mode)
        );
    }
    // The breadcrumb names the slider in both.
    assert!(frame_texts(&mut h, &mut app).contains(&"Manifold view › Centre z₀ › z_α".to_owned()));
}

// --- The state machine itself (`ScopeTree::navigate`) ----------------------------------------------------------------

fn focus_at(path: &[&str], mode: KeyMode) -> Focus {
    Focus {
        path: path.iter().map(|s| (*s).to_owned()).collect(),
        mode,
    }
}

/// A tree of a group holding two values, a control, a disabled leaf and a text field, and a big scope that is a value.
fn tree() -> ScopeTree {
    let mut tree = ScopeTree::new();
    tree.register(None, Scope::group("g", "G"));
    tree.register(Some("g"), Scope::value("a", "A", StepKind::Bounded));
    tree.register(Some("g"), Scope::value("b", "B", StepKind::Bounded));
    tree.register(Some("g"), Scope::control("c", "C"));
    tree.register(Some("g"), Scope::disabled("d", "D"));
    tree.register(Some("g"), Scope::text("t", "T"));
    tree.register(None, Scope::value("v", "V", StepKind::Pan));
    tree
}

#[test]
fn mock_keyboard_modes_the_state_machine() {
    let tree = tree();
    let down = Command::Arrow(Direction::Down);
    let mut f = focus_at(&["g"], NAV);
    assert_eq!(tree.navigate(&mut f, Command::Enter, 1.0), Outcome::Moved);
    assert_eq!(f, focus_at(&["g", "a"], NAV), "into the scope, navigating");
    assert_eq!(tree.navigate(&mut f, down, 1.0), Outcome::Moved);
    assert_eq!(f, focus_at(&["g", "b"], NAV));
    assert_eq!(tree.navigate(&mut f, Command::Enter, 1.0), Outcome::Moved);
    assert_eq!(
        f,
        focus_at(&["g", "b"], INTERACT),
        "Enter on a value interacts"
    );
    let adjusted = tree.navigate(&mut f, down, 10.0);
    assert!(
        matches!(adjusted, Outcome::Adjust(a) if a.scope == "b" && a.times == -10.0 && a.direction == Direction::Down),
        "{adjusted:?}"
    );
    assert_eq!(
        f,
        focus_at(&["g", "b"], INTERACT),
        "the arrows adjust, the focus kept"
    );
    assert_eq!(tree.navigate(&mut f, Command::Back, 1.0), Outcome::Moved);
    assert_eq!(
        f,
        focus_at(&["g", "b"], NAV),
        "Esc: navigating on the value"
    );
    assert_eq!(tree.navigate(&mut f, down, 1.0), Outcome::Moved);
    assert_eq!(f, focus_at(&["g", "c"], NAV));
    assert_eq!(
        tree.navigate(&mut f, Command::Enter, 1.0),
        Outcome::Activate("c"),
        "Enter on a control: its click"
    );
    assert_eq!(f, focus_at(&["g", "c"], NAV), "still navigating");
    assert_eq!(tree.navigate(&mut f, down, 1.0), Outcome::Moved);
    assert_eq!(tree.navigate(&mut f, Command::Enter, 1.0), Outcome::None);
    assert_eq!(f, focus_at(&["g", "d"], NAV), "a disabled leaf: nothing");
    assert_eq!(tree.navigate(&mut f, down, 1.0), Outcome::Moved);
    assert_eq!(
        tree.navigate(&mut f, down, 1.0),
        Outcome::None,
        "the last sibling"
    );
    assert_eq!(tree.navigate(&mut f, Command::Enter, 1.0), Outcome::Moved);
    assert_eq!(
        f,
        focus_at(&["g", "t"], INTERACT),
        "Enter on a text field: typing"
    );
    // A text field's arrows are its own; if they reach the tree, they do nothing.
    assert_eq!(tree.navigate(&mut f, down, 1.0), Outcome::None);
    assert_eq!(tree.navigate(&mut f, Command::Enter, 1.0), Outcome::Moved);
    assert_eq!(f, focus_at(&["g", "t"], NAV), "Enter ends the typing");
    assert_eq!(tree.navigate(&mut f, Command::Back, 1.0), Outcome::Moved);
    assert_eq!(f, focus_at(&["g"], NAV), "Esc navigating: up a scope");
    // Tab from interaction mode: the next big scope, navigating; a big value takes Enter as interaction.
    let mut f = focus_at(&["g", "a"], INTERACT);
    assert_eq!(tree.navigate(&mut f, Command::Next, 1.0), Outcome::Moved);
    assert_eq!(f, focus_at(&["v"], NAV));
    assert_eq!(tree.navigate(&mut f, Command::Enter, 1.0), Outcome::Moved);
    assert_eq!(f, focus_at(&["v"], INTERACT));
    assert_eq!(
        tree.navigate(&mut f, Command::Previous, 1.0),
        Outcome::Moved
    );
    assert_eq!(f, focus_at(&["g"], NAV));
    // On a big scope, the arrows: between the big scopes, stopping at the ends.
    assert_eq!(
        tree.navigate(&mut f, Command::Arrow(Direction::Up), 1.0),
        Outcome::None
    );
    assert_eq!(
        tree.navigate(&mut f, Command::Arrow(Direction::Right), 1.0),
        Outcome::Moved
    );
    assert_eq!(f, focus_at(&["v"], NAV));
    // A group in interaction mode (never reached by keys) has no arrows to take; nothing focused is navigating.
    let mut g = focus_at(&["g"], INTERACT);
    assert_eq!(tree.navigate(&mut g, down, 1.0), Outcome::None);
    let mut none = focus_at(&[], INTERACT);
    assert_eq!(
        tree.navigate(&mut none, Command::Shortcuts, 1.0),
        Outcome::None
    );
    assert_eq!(none.mode, NAV);
    // The control: the pre-R-409 model, where Enter into a scope adjusts its first value.
    let browsing = |f: &mut Focus, c| {
        let o = tree.navigate(f, c, 1.0);
        if c == Command::Enter && f.path.len() == 2 && f.path[1] == "a" {
            f.mode = INTERACT;
        }
        o
    };
    let mut old = focus_at(&["g"], NAV);
    browsing(&mut old, Command::Enter);
    let o = browsing(&mut old, down);
    rejects("the pre-R-409 browsing model", || {
        assert_eq!(o, Outcome::Moved, "↓ after Enter into a scope moves")
    });
}

// --- A text field (R-409 G4, G5) -------------------------------------------------------------------------------------

/// A bare frame drawing a text field, placed as scope `field` for `keyboard`.
fn field_frame(
    ctx: &egui::Context,
    keyboard: &mut Keyboard,
    raw: egui::RawInput,
    text: &mut String,
) {
    let mut output = ctx.run_ui(raw, |ui| {
        let response = ui.text_edit_singleline(text);
        keyboard.clear_places();
        keyboard.place_widget("field", &response);
    });
    output.textures_delta.clear();
}

fn raw(events: Vec<Event>) -> egui::RawInput {
    egui::RawInput {
        events,
        ..Default::default()
    }
}

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

/// The events `take_keys` leaves to egui of `events`, then the layer's run on `focus`.
fn step(
    ctx: &egui::Context,
    keyboard: &mut Keyboard,
    focus: &mut Focus,
    events: Vec<Event>,
) -> Vec<Event> {
    let mut r = raw(events);
    keyboard.take_keys(ctx, &mut r);
    let _ = keyboard.run(ctx, Mode::Explore, focus, 0.0);
    r.events
}

#[test]
fn mock_keyboard_modes_a_text_field_types_after_enter_and_keeps_esc_enter_and_tab() {
    let ctx = egui::Context::default();
    let mut keyboard = Keyboard::new();
    keyboard
        .tree_mut(Mode::Explore)
        .register(None, Scope::text("field", "Field"));
    let mut text = String::new();
    field_frame(&ctx, &mut keyboard, raw(Vec::new()), &mut text);
    // Landed on, navigating: a letter is not the layer's, and the field does not hold the keyboard.
    let mut f = focus_at(&["field"], NAV);
    let _ = keyboard.run(&ctx, Mode::Explore, &mut f, 0.0);
    assert_eq!(keyboard.typing(), None);
    let landed = keyboard.typing();
    rejects("a text field that types on landing", || {
        assert_eq!(landed, Some("field"))
    });
    // Enter starts typing: interaction mode, egui's focus on the field.
    let left = step(&ctx, &mut keyboard, &mut f, vec![key(Key::Enter, NONE)]);
    assert!(left.is_empty(), "the layer took the Enter");
    assert_eq!(f, focus_at(&["field"], INTERACT));
    assert_eq!(keyboard.typing(), Some("field"));
    field_frame(&ctx, &mut keyboard, raw(Vec::new()), &mut text);
    assert!(ctx.text_edit_focused(), "the field has egui's focus");
    // While typing, every key but Esc, Enter and Tab is the field's: a letter, the arrows, `?` and Ctrl+Z.
    let theirs = vec![
        key(Key::A, NONE),
        key(Key::ArrowDown, NONE),
        key(Key::Questionmark, SHIFT),
        key(Key::Z, Modifiers::CTRL),
    ];
    let left = step(&ctx, &mut keyboard, &mut f, theirs.clone());
    assert_eq!(left, theirs, "the field's keys");
    assert!(!keyboard.shortcuts_open, "? typed, not the overlay");
    assert_eq!(f, focus_at(&["field"], INTERACT));
    rejects("a ? while typing that opens the overlay", || {
        assert!(left.len() < theirs.len())
    });
    // Esc ends the typing, the text kept: navigation mode on the field.
    field_frame(
        &ctx,
        &mut keyboard,
        raw(vec![Event::Text("ab".into())]),
        &mut text,
    );
    assert_eq!(text, "ab");
    let left = step(&ctx, &mut keyboard, &mut f, vec![key(Key::Escape, NONE)]);
    assert!(left.is_empty());
    assert_eq!(f, focus_at(&["field"], NAV));
    assert_eq!(keyboard.typing(), None);
    field_frame(&ctx, &mut keyboard, raw(Vec::new()), &mut text);
    assert_eq!(text, "ab", "the text kept");
    // Enter, then Enter: typing, then back.
    let _ = step(&ctx, &mut keyboard, &mut f, vec![key(Key::Enter, NONE)]);
    assert_eq!(f.mode, INTERACT);
    let _ = step(&ctx, &mut keyboard, &mut f, vec![key(Key::Enter, NONE)]);
    assert_eq!(f, focus_at(&["field"], NAV));
    // Typing, Tab moves on, navigating; the field loses egui's focus.
    let _ = step(&ctx, &mut keyboard, &mut f, vec![key(Key::Enter, NONE)]);
    field_frame(&ctx, &mut keyboard, raw(Vec::new()), &mut text);
    let _ = step(&ctx, &mut keyboard, &mut f, vec![key(Key::Tab, NONE)]);
    assert_eq!(f.mode, NAV);
    assert_eq!(keyboard.typing(), None);
    let mut r = raw(Vec::new());
    keyboard.take_keys(&ctx, &mut r);
    assert!(!ctx.text_edit_focused(), "the layer took egui's focus back");
    assert_eq!(text, "ab");
}

// --- The two captures differ only in the ring's colour and the breadcrumb's (REQ-GUI-182, REQ-GUI-098) ---------------

#[test]
fn mock_keyboard_modes_navigation_and_interaction_differ_only_in_the_colours() {
    use super::mock_keyboard::{canvas, check_only_ring_and_breadcrumb, pixel};
    use crate::capture::{shoot, Step};
    let canvas = canvas();
    let navigate = [
        Step::Tab,
        Step::Tab,
        Step::Enter,
        Step::ArrowDown,
        Step::ArrowDown,
        Step::Enter,
    ];
    let mut interact = navigate.to_vec();
    interact.push(Step::Enter);
    let before = shoot(canvas.clone(), &navigate);
    let after = shoot(canvas.clone(), &interact);
    // The slider's ring, where the app places it.
    let (mut app, mut h) = app();
    keys(
        &mut h,
        &mut app,
        &[
            Key::Tab,
            Key::Tab,
            Key::Enter,
            Key::ArrowDown,
            Key::ArrowDown,
            Key::Enter,
        ],
    );
    let _ = h.frame(&mut app, Vec::new());
    let ring = app
        .keyboard
        .place_of(SLIDERS[0])
        .expect("the slider placed")
        .rect;
    let on_ring = check_only_ring_and_breadcrumb(&before, &after, &[ring]);
    assert!(on_ring[0] > 0, "the ring's colour did not change");
    // On the ring, the colours are the two modes'.
    let p = crate::capture::PIXELS_PER_POINT;
    let (x, y) = (
        (ring.center().x * p) as usize,
        (ring.min.y * p + 1.0) as usize,
    );
    let rgb = |c: [u8; 4]| [c[0], c[1], c[2]];
    let blue = egui::Visuals::dark().hyperlink_color;
    assert_eq!(
        rgb(pixel(&before, x, y)),
        [NAVIGATION.r(), NAVIGATION.g(), NAVIGATION.b()]
    );
    assert_eq!(rgb(pixel(&after, x, y)), [blue.r(), blue.g(), blue.b()]);
    // Moving the focus changes more than a colour.
    let moved = shoot(canvas, &[Step::Tab, Step::Tab]);
    rejects("a capture whose focus moved, not just its mode", || {
        check_only_ring_and_breadcrumb(&before, &moved, &[ring]);
    });
}
