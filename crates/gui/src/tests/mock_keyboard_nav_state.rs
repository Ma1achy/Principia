//! `mock_keyboard_nav_state` (REQ-GUI-210; render_gui_spec §G3; R-460, R-464): the focus and keyboard navigation are
//! apart. Keyboard navigation (the ring, and the breadcrumb's focus path) starts only on a key press; any click or drag
//! turns it off, the focus moving only as R-450 A3 and R-456 say; a scroll or a hover leaves it. Pointer and keyboard
//! scripts on the mock: one per entry point, opened by a click with no key pressed (the focus on the window's landing,
//! no ring) and, its control arm, by keys (the ring on the landing); Tab, Enter and Esc after a mouse open; a menu
//! opened by a click; R-464's click then key, drag, text-field and click-off-a-menu scripts. Each check also runs on an
//! input it must reject (R-176, through `rejects`).

use eframe::egui::{self, Color32, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::Lock;
use engine::contract::view_ui::{KeyMode, Window};

use super::support::{click_scope, headless, mock_app, press, rejects, UNBOUND};
use crate::app::App;
use crate::capture::{footer_point, PIXELS_PER_POINT};
use crate::explore::breadcrumb::{self, NAVIGATION, RING_WIDTH};
use crate::explore::manifold_view as mv;
use crate::explore::{footer, legend, trajectory, COMPASS, COMPASS_VIEW, FIGURE, MANIFOLD_VIEW};
use crate::headless::Headless;
use crate::keyboard::scopes::{Ring, ScopeId};
use crate::layout::Layout;
use crate::side::MockSide;
use crate::windows::{self, console, REGISTRY};

const NONE: Modifiers = Modifiers::NONE;

/// Eight frames: egui lays a new window out in a sizing pass first.
fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..8 {
        let _ = h.frame(app, Vec::new());
    }
}

/// A fresh app, two frames drawn.
fn fresh() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

fn layout(h: &Headless) -> Layout {
    Layout::new(h.screen(), PIXELS_PER_POINT)
}

fn owned(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

fn path_to(app: &App<MockSide>, id: &str) -> Vec<String> {
    app.keyboard.tree(app.view.mode).path_to(id)
}

/// What one frame shows of the keyboard.
#[derive(Clone, Debug)]
struct Seen {
    focus: Vec<String>,
    mode: KeyMode,
    navigating: bool,
    /// The rings drawn: their rects and colours.
    rings: Vec<(Rect, Color32)>,
    /// The breadcrumb of the focus, if drawn in the top bar.
    breadcrumb: Option<String>,
    /// Where the ring goes on the focused scope, as it was placed this frame.
    ring_at: Option<Rect>,
}

/// Where the ring goes on scope `id`: its place, grown by a point for a ring outside it.
fn ring_rect(app: &App<MockSide>, id: &str) -> Option<Rect> {
    let place = app.keyboard.place_of(id)?;
    let scope = app.keyboard.tree(app.view.mode).get(id)?;
    Some(match scope.ring {
        Ring::Inside => place.rect,
        Ring::Outside => place.rect.expand(1.0),
    })
}

/// One more frame, and what it shows.
fn seen(h: &mut Headless, app: &mut App<MockSide>) -> Seen {
    let output = h.frame(app, Vec::new());
    let blue = h.ctx.global_style().visuals.hyperlink_color;
    let mut flat = Vec::new();
    fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
        match shape {
            egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    for clipped in &output.shapes {
        flatten(&clipped.shape, &mut flat);
    }
    let rings = flat
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Rect(r)
                if r.stroke.width == RING_WIDTH
                    && (r.stroke.color == NAVIGATION || r.stroke.color == blue) =>
            {
                Some((r.rect, r.stroke.color))
            }
            _ => None,
        })
        .collect();
    let crumb = breadcrumb::text(app.keyboard.tree(app.view.mode), &app.view.focus);
    let names = h.names(&output);
    // The breadcrumb is its text after the keyboard glyph, which nothing else draws.
    let drawn = |t: &str| names.iter().any(|n| n.name == t);
    let breadcrumb = (drawn(breadcrumb::GLYPH) && drawn(&crumb)).then_some(crumb);
    let ring_at = app.view.focus.path.last().and_then(|id| ring_rect(app, id));
    Seen {
        focus: app.view.focus.path.clone(),
        mode: app.view.focus.mode,
        navigating: app.keyboard.navigating(),
        rings,
        breadcrumb,
        ring_at,
    }
}

/// Asserts the focus is on `want`, keyboard navigation off: no ring drawn and no breadcrumb.
fn check_hidden(s: &Seen, want: &[String], what: &str) {
    assert_eq!(s.focus, want, "{what}: the focus");
    assert!(!s.navigating, "{what}: keyboard navigation on");
    assert!(s.rings.is_empty(), "{what}: a ring drawn, {:?}", s.rings);
    assert_eq!(s.breadcrumb, None, "{what}: the breadcrumb drawn");
}

/// Asserts the focus is on `want`, keyboard navigation on: one ring, in `colour`, on the focused scope, and the
/// breadcrumb naming its path.
fn check_shown(s: &Seen, want: &[String], colour: Color32, what: &str) {
    assert_eq!(s.focus, want, "{what}: the focus");
    assert!(s.navigating, "{what}: keyboard navigation off");
    let at = s
        .ring_at
        .unwrap_or_else(|| panic!("{what}: the focus not placed"));
    assert_eq!(s.rings, [(at, colour)], "{what}: one ring, on the focus");
    assert!(s.breadcrumb.is_some(), "{what}: no breadcrumb");
}

/// Presses `key` with `modifiers`, returning whether any of its frames asked for a copy to the clipboard.
fn press_copies(h: &mut Headless, app: &mut App<MockSide>, key: Key) -> bool {
    crate::capture::key(key, NONE)
        .into_iter()
        .chain([Vec::new(), Vec::new()])
        .map(|events| h.frame(app, events))
        .any(|o| {
            o.platform_output
                .commands
                .iter()
                .any(|c| matches!(c, egui::OutputCommand::CopyText(_)))
        })
}

/// Clicks at `at`: three frames.
fn click_at(h: &mut Headless, app: &mut App<MockSide>, at: Pos2) {
    for events in crate::capture::click(at) {
        let _ = h.frame(app, events);
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    }
}

/// A primary drag from `from` by `delta`, one frame a step.
fn drag(h: &mut Headless, app: &mut App<MockSide>, from: Pos2, delta: egui::Vec2) {
    for events in [
        vec![Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![Event::PointerMoved(from + delta / 2.0)],
        vec![Event::PointerMoved(from + delta)],
        vec![button(from + delta, false)],
        Vec::new(),
    ] {
        let _ = h.frame(app, events);
    }
}

/// Reaches scope `id` by the keys alone: Tab to its big scope, then at each level Enter into it and the arrows across.
fn reach(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    let target = path_to(app, id);
    assert!(!target.is_empty(), "no scope {id}");
    for _ in 0..16 {
        if app.view.focus.path == target[..1] {
            break;
        }
        press(h, app, Key::Tab, NONE);
    }
    assert_eq!(app.view.focus.path, target[..1], "Tab to {}", target[0]);
    for depth in 1..target.len() {
        press(h, app, Key::Enter, NONE);
        for _ in 0..40 {
            if app.view.focus.path.get(depth) == Some(&target[depth]) {
                break;
            }
            let before = app.view.focus.path.clone();
            press(h, app, Key::ArrowRight, NONE);
            if app.view.focus.path == before {
                press(h, app, Key::ArrowDown, NONE);
            }
        }
        assert_eq!(
            app.view.focus.path[..=depth],
            target[..=depth],
            "the keys to {id}"
        );
    }
}

/// Locks the plane, as K over the figure does, by its `SetField`: open in Inspector is enabled only while locked.
fn lock(h: &mut Headless, app: &mut App<MockSide>) {
    app.set_field(SetField {
        edit: Edit::Sim(SimField::Lock(Lock {
            locked: true,
            z_locked: [0.0; 8],
        })),
        no_history: false,
    });
    settle(h, app);
}

/// An entry point: what it is called, the control that opens the window, and the window.
struct EntryPoint {
    name: String,
    opener: ScopeId,
    window: Window,
}

/// Every entry point (R-460 H5): the footer, Windows › each of the seven, the top bar's Run…, Profiler… and Export…,
/// the Manifold view's Chart builder… and open in Inspector, and the Trajectory panel's IC Inspector… and Open full
/// viewer….
fn entry_points() -> Vec<EntryPoint> {
    let mut all = vec![EntryPoint {
        name: "the footer".to_owned(),
        opener: footer::FOOTER,
        window: Window::Console,
    }];
    all.extend(REGISTRY.iter().map(|e| EntryPoint {
        name: format!("Windows › {}", e.name),
        opener: e.menu_entry,
        window: e.window,
    }));
    for (name, opener, window) in [
        ("Run…", "run", Window::Run),
        ("Profiler…", "profiler", Window::Profiler),
        ("Export…", "export", Window::Export),
        ("Chart builder…", mv::BUILDER, Window::ChartBuilder),
        ("open in Inspector", mv::OPEN_INSPECTOR, Window::Inspector),
        ("IC Inspector…", trajectory::INSPECTOR, Window::Inspector),
        ("Open full viewer…", trajectory::VIEWER, Window::Inspector),
    ] {
        all.push(EntryPoint {
            name: name.to_owned(),
            opener,
            window,
        });
    }
    all
}

/// A fresh app with `entry`'s opener enabled.
fn fresh_for(entry: &EntryPoint) -> (App<MockSide>, Headless) {
    let (mut app, mut h) = fresh();
    if entry.opener == mv::OPEN_INSPECTOR {
        lock(&mut h, &mut app);
    }
    (app, h)
}

/// Opens `entry`'s window with the mouse alone: a click on the footer, on the menu then its entry, or on the button.
fn open_by_mouse(h: &mut Headless, app: &mut App<MockSide>, entry: &EntryPoint) {
    if entry.opener == footer::FOOTER {
        let at = footer_point(layout(h).footer);
        click_at(h, app, at);
    } else {
        let tree = app.keyboard.tree(app.view.mode);
        if let Some(menu) = tree
            .parent(entry.opener)
            .filter(|m| tree.get(m).is_some_and(|s| s.menu))
        {
            click_scope(h, app, menu);
            settle(h, app);
        }
        click_scope(h, app, entry.opener);
    }
    settle(h, app);
}

/// Opens `entry`'s window from the keyboard: its opener reached by the keys, then Enter.
fn open_by_keys(h: &mut Headless, app: &mut App<MockSide>, entry: &EntryPoint) {
    reach(h, app, entry.opener);
    press(h, app, Key::Enter, NONE);
    settle(h, app);
}

fn landing(app: &App<MockSide>, window: Window) -> Vec<String> {
    app.keyboard
        .tree(app.view.mode)
        .window_landing(windows::entry(window).scope)
}

#[test]
fn mock_keyboard_nav_state_the_gui_starts_with_it_off() {
    let (mut app, mut h) = fresh();
    let s = seen(&mut h, &mut app);
    check_hidden(&s, &[], "at start");
    // Control: a Tab turns it on, the ring on the top bar.
    press(&mut h, &mut app, Key::Tab, NONE);
    let tabbed = seen(&mut h, &mut app);
    check_shown(&tabbed, &owned(&["top_bar"]), NAVIGATION, "Tab");
    rejects("a GUI that starts with it on", || {
        check_hidden(&tabbed, &[], "at start")
    });
}

#[test]
fn mock_keyboard_nav_state_each_entry_point_opened_by_a_click_shows_no_ring() {
    let all = entry_points();
    assert_eq!(all.len(), 15, "fifteen entry points");
    for entry in &all {
        // Opened by a click with no key pressed: the focus on the landing, keyboard navigation off.
        let (mut app, mut h) = fresh_for(entry);
        open_by_mouse(&mut h, &mut app, entry);
        assert!(app.is_open(entry.window), "{}: not opened", entry.name);
        let want = landing(&app, entry.window);
        let by_mouse = seen(&mut h, &mut app);
        check_hidden(&by_mouse, &want, &entry.name);
        // Its control arm: opened by keys, keyboard navigation on and the ring on the landing.
        let (mut app, mut h) = fresh_for(entry);
        open_by_keys(&mut h, &mut app, entry);
        assert!(
            app.is_open(entry.window),
            "{}: not opened by keys",
            entry.name
        );
        let by_keys = seen(&mut h, &mut app);
        let what = format!("{} by keys", entry.name);
        check_shown(&by_keys, &want, NAVIGATION, &what);
        // A GUI that never turns it on fails the keys' check; one that turns it on at a mouse open (today's) fails
        // the mouse's.
        rejects(&format!("{}: never on", entry.name), || {
            check_shown(&by_mouse, &want, NAVIGATION, &what)
        });
        rejects(&format!("{}: on at a mouse open", entry.name), || {
            check_hidden(&by_keys, &want, &entry.name)
        });
    }
}

/// The console opened by a click on the footer.
fn console_by_mouse() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = fresh();
    let at = footer_point(layout(&h).footer);
    click_at(&mut h, &mut app, at);
    settle(&mut h, &mut app);
    assert!(app.console_open);
    (app, h)
}

#[test]
fn mock_keyboard_nav_state_after_a_mouse_open_tab_turns_it_on_and_moves_on() {
    let (mut app, mut h) = console_by_mouse();
    let want = landing(&app, Window::Console);
    let before = seen(&mut h, &mut app);
    check_hidden(&before, &want, "opened by the mouse");
    press(&mut h, &mut app, Key::Tab, NONE);
    let log = owned(&[console::WINDOW, console::LOG]);
    let tabbed = seen(&mut h, &mut app);
    check_shown(&tabbed, &log, NAVIGATION, "Tab from the landing");
    rejects("no ring after Tab", || {
        check_shown(&before, &log, NAVIGATION, "Tab")
    });
}

#[test]
fn mock_keyboard_nav_state_after_a_mouse_open_enter_turns_it_on_without_acting() {
    let (mut app, mut h) = console_by_mouse();
    let want = landing(&app, Window::Console);
    press(&mut h, &mut app, Key::Enter, NONE);
    let revealed = seen(&mut h, &mut app);
    check_shown(&revealed, &want, NAVIGATION, "Enter after a mouse open");
    // The next Enter acts: into the header bar, onto its first control.
    press(&mut h, &mut app, Key::Enter, NONE);
    let entered = seen(&mut h, &mut app);
    assert_eq!(
        entered.focus.len(),
        want.len() + 1,
        "the second Enter went in"
    );
    check_shown(&entered, &entered.focus, NAVIGATION, "the second Enter");
    rejects("an Enter that acts at once", || {
        check_shown(&entered, &want, NAVIGATION, "Enter")
    });
}

#[test]
fn mock_keyboard_nav_state_esc_closes_a_mouse_opened_window() {
    // Esc acts as always with keyboard navigation off: it closes the console, the focus back on the footer, and,
    // as any key press, shows the ring there (R-460 H2, R-464 M2).
    let (mut app, mut h) = console_by_mouse();
    let opened = seen(&mut h, &mut app);
    press(&mut h, &mut app, Key::Escape, NONE);
    settle(&mut h, &mut app);
    assert!(!app.console_open, "Esc closed the console");
    let closed = seen(&mut h, &mut app);
    let footer = owned(&[footer::FOOTER]);
    check_shown(&closed, &footer, NAVIGATION, "Esc");
    rejects("an Esc that leaves the window", || {
        check_shown(&opened, &footer, NAVIGATION, "Esc")
    });
}

#[test]
fn mock_keyboard_nav_state_a_menu_opened_by_a_click_shows_no_ring() {
    let (mut app, mut h) = fresh();
    click_scope(&mut h, &mut app, "windows");
    settle(&mut h, &mut app);
    assert_eq!(app.keyboard.open_menu(), Some("windows"));
    let first = owned(&["top_bar", "windows", REGISTRY[0].menu_entry]);
    let clicked = seen(&mut h, &mut app);
    check_hidden(&clicked, &first, "the Windows menu clicked");
    // Control: opened by keys, the ring on its first entry.
    let (mut app, mut h) = fresh();
    reach(&mut h, &mut app, "windows");
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert_eq!(app.keyboard.open_menu(), Some("windows"));
    let keyed = seen(&mut h, &mut app);
    check_shown(&keyed, &first, NAVIGATION, "the Windows menu by keys");
    rejects("a ring on a menu the mouse opened", || {
        check_hidden(&keyed, &first, "the Windows menu")
    });
}

#[test]
fn mock_keyboard_nav_state_a_click_off_an_open_menu_hides_the_ring() {
    let (mut app, mut h) = fresh();
    reach(&mut h, &mut app, "windows");
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    let open = seen(&mut h, &mut app);
    let first = owned(&["top_bar", "windows", REGISTRY[0].menu_entry]);
    check_shown(&open, &first, NAVIGATION, "the menu open by keys");
    let off = app
        .keyboard
        .place_of(legend::LEGEND)
        .expect("the legend placed")
        .rect
        .center();
    click_at(&mut h, &mut app, off);
    settle(&mut h, &mut app);
    assert_eq!(app.keyboard.open_menu(), None, "the click off closed it");
    let menu = owned(&["top_bar", "windows"]);
    let closed = seen(&mut h, &mut app);
    check_hidden(&closed, &menu, "a click off the menu");
    rejects("a ring kept through the click", || {
        check_hidden(&open, &menu, "a click off the menu")
    });
    // The next key shows it on the menu's control.
    press(&mut h, &mut app, UNBOUND, NONE);
    check_shown(&seen(&mut h, &mut app), &menu, NAVIGATION, "the next key");
}

/// The console opened from the keyboard, the ring on its landing.
fn console_by_keys() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = fresh();
    reach(&mut h, &mut app, footer::FOOTER);
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    let want = landing(&app, Window::Console);
    check_shown(
        &seen(&mut h, &mut app),
        &want,
        NAVIGATION,
        "the console by keys",
    );
    (app, h)
}

#[test]
fn mock_keyboard_nav_state_a_click_on_copy_hides_the_ring_and_enter_shows_it_there() {
    let (mut app, mut h) = console_by_keys();
    let before = seen(&mut h, &mut app);
    click_scope(&mut h, &mut app, console::COPY);
    let copy = owned(&[console::WINDOW, console::HEADER, console::COPY]);
    let clicked = seen(&mut h, &mut app);
    check_hidden(&clicked, &copy, "a click on Copy");
    assert_eq!(
        clicked.mode,
        KeyMode::Navigation,
        "R-450 A3's navigation mode"
    );
    // Enter shows it on Copy, orange, without acting: nothing is copied.
    let copied = press_copies(&mut h, &mut app, Key::Enter);
    assert!(!copied, "the first Enter copied");
    let shown = seen(&mut h, &mut app);
    check_shown(&shown, &copy, NAVIGATION, "Enter after the click");
    // The next Enter acts, as Copy's click.
    assert!(
        press_copies(&mut h, &mut app, Key::Enter),
        "the second Enter did not copy"
    );
    // Controls: a ring still drawn after the click, none after the key, and one off the focus the click left.
    rejects("a ring kept through the click", || {
        check_hidden(&before, &copy, "a click on Copy")
    });
    rejects("no ring after the key", || {
        check_shown(&clicked, &copy, NAVIGATION, "Enter")
    });
    let mut elsewhere = shown.clone();
    elsewhere.rings[0].0 = before.ring_at.expect("the landing placed");
    rejects("a ring off the focus", || {
        check_shown(&elsewhere, &copy, NAVIGATION, "Enter")
    });
}

#[test]
fn mock_keyboard_nav_state_a_click_on_the_figure_hides_the_ring_and_enter_shows_it_there() {
    let (mut app, mut h) = console_by_keys();
    let want = landing(&app, Window::Console);
    let before = seen(&mut h, &mut app);
    let centre = layout(&h).figure.center();
    click_at(&mut h, &mut app, centre);
    // The click pins a sample and leaves the focus where it was (R-456 B6).
    let clicked = seen(&mut h, &mut app);
    check_hidden(&clicked, &want, "a click on the Figure");
    press(&mut h, &mut app, Key::Enter, NONE);
    let shown = seen(&mut h, &mut app);
    check_shown(&shown, &want, NAVIGATION, "Enter after the click");
    rejects("a ring kept through the click", || {
        check_hidden(&before, &want, "a click on the Figure")
    });
    rejects("no ring after the key", || {
        check_shown(&clicked, &want, NAVIGATION, "Enter")
    });
    let mut elsewhere = shown.clone();
    elsewhere.rings[0].0 = elsewhere.rings[0].0.translate(egui::vec2(40.0, 0.0));
    rejects("a ring off the focus", || {
        check_shown(&elsewhere, &want, NAVIGATION, "Enter")
    });
}

#[test]
fn mock_keyboard_nav_state_a_letter_shortcut_after_a_click_shows_the_ring_and_acts() {
    let (mut app, mut h) = fresh();
    reach(&mut h, &mut app, FIGURE);
    let figure = owned(&[FIGURE]);
    let before = seen(&mut h, &mut app);
    check_shown(&before, &figure, NAVIGATION, "Tab to the Figure");
    let centre = layout(&h).figure.center();
    click_at(&mut h, &mut app, centre);
    let clicked = seen(&mut h, &mut app);
    check_hidden(&clicked, &figure, "a click on the Figure");
    assert!(!app.trajectory.listen);
    press(&mut h, &mut app, Key::L, NONE);
    assert!(app.trajectory.listen, "L acted");
    let shown = seen(&mut h, &mut app);
    check_shown(&shown, &figure, NAVIGATION, "L after the click");
    rejects("no ring after the letter", || {
        check_shown(&clicked, &figure, NAVIGATION, "L")
    });
    rejects("a ring kept through the click", || {
        check_hidden(&before, &figure, "the click")
    });
}

/// A drag on scope `id`'s place, the ring on first on `on`: hidden through the drag, the focus unmoved, and shown
/// there by the next Enter, which does not act.
fn drag_script(on: &str, id: &str) {
    let (mut app, mut h) = fresh();
    reach(&mut h, &mut app, on);
    let at = path_to(&app, on);
    let before = seen(&mut h, &mut app);
    check_shown(&before, &at, NAVIGATION, "the keys");
    let from = app
        .keyboard
        .place_of(id)
        .unwrap_or_else(|| panic!("{id} placed"))
        .rect
        .center();
    drag(&mut h, &mut app, from, egui::vec2(24.0, -12.0));
    let dragged = seen(&mut h, &mut app);
    check_hidden(&dragged, &at, &format!("a drag on {id}"));
    press(&mut h, &mut app, Key::Enter, NONE);
    check_shown(
        &seen(&mut h, &mut app),
        &at,
        NAVIGATION,
        "Enter after the drag",
    );
    rejects("a ring kept through the drag", || {
        check_hidden(&before, &at, "the drag")
    });
}

#[test]
fn mock_keyboard_nav_state_a_drag_on_a_slice_centre_slider_hides_the_ring() {
    drag_script(MANIFOLD_VIEW, mv::SLIDERS[0]);
}

#[test]
fn mock_keyboard_nav_state_a_drag_on_the_compass_hides_the_ring() {
    drag_script(COMPASS, COMPASS_VIEW);
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

#[test]
fn mock_keyboard_nav_state_a_click_on_the_search_field_types_with_no_ring_until_a_letter() {
    let (mut app, mut h) = console_by_keys();
    click_scope(&mut h, &mut app, console::SEARCH);
    let search = owned(&[console::WINDOW, console::HEADER, console::SEARCH]);
    let clicked = seen(&mut h, &mut app);
    check_hidden(&clicked, &search, "a click on the search field");
    assert_eq!(
        clicked.mode,
        KeyMode::Interaction,
        "the click starts typing"
    );
    assert_eq!(app.keyboard.typing(), Some(console::SEARCH));
    type_char(&mut h, &mut app, 'q');
    assert_eq!(app.windows.console.text, "q", "the letter typed");
    let blue = h.ctx.global_style().visuals.hyperlink_color;
    let typed = seen(&mut h, &mut app);
    check_shown(&typed, &search, blue, "a letter after the click");
    rejects("no ring after the letter", || {
        check_shown(&clicked, &search, blue, "the letter")
    });
    rejects("a ring drawn at the click", || {
        check_hidden(&typed, &search, "the click")
    });
}

#[test]
fn mock_keyboard_nav_state_enter_after_a_click_on_the_search_field_stops_typing() {
    // Enter while typing acts as typing does: it stops typing, the ring orange on the field (R-464).
    let (mut app, mut h) = console_by_keys();
    click_scope(&mut h, &mut app, console::SEARCH);
    let search = owned(&[console::WINDOW, console::HEADER, console::SEARCH]);
    let clicked = seen(&mut h, &mut app);
    check_hidden(&clicked, &search, "a click on the search field");
    press(&mut h, &mut app, Key::Enter, NONE);
    let stopped = seen(&mut h, &mut app);
    assert_eq!(stopped.mode, KeyMode::Navigation, "Enter stopped typing");
    assert_eq!(app.keyboard.typing(), None);
    check_shown(&stopped, &search, NAVIGATION, "Enter after the click");
    rejects("an Enter that only shows the ring while typing", || {
        assert_eq!(clicked.mode, KeyMode::Navigation)
    });
}

#[test]
fn mock_keyboard_nav_state_a_scroll_or_a_hover_leaves_the_ring() {
    let (mut app, mut h) = fresh();
    reach(&mut h, &mut app, FIGURE);
    let figure = owned(&[FIGURE]);
    let centre = layout(&h).figure.center();
    for events in [
        vec![Event::PointerMoved(centre)],
        vec![Event::PointerMoved(centre + egui::vec2(30.0, 10.0))],
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            modifiers: NONE,
            phase: egui::TouchPhase::Move,
        }],
    ] {
        let _ = h.frame(&mut app, events);
    }
    let kept = seen(&mut h, &mut app);
    check_shown(&kept, &figure, NAVIGATION, "a hover and a scroll");
    // Control: a click hides it.
    click_at(&mut h, &mut app, centre);
    let clicked = seen(&mut h, &mut app);
    rejects("a click that leaves the ring", || {
        check_shown(&clicked, &figure, NAVIGATION, "the click")
    });
}

#[test]
fn mock_keyboard_nav_state_only_presses_turn_it_on_and_off() {
    let (mut app, mut h) = fresh();
    reach(&mut h, &mut app, FIGURE);
    let centre = layout(&h).figure.center();
    // A pointer press turns it off; a key's release then leaves it off.
    let _ = h.frame(
        &mut app,
        vec![Event::PointerMoved(centre), button(centre, true)],
    );
    assert!(!app.keyboard.navigating(), "the press turned it off");
    let release = |key| Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: NONE,
    };
    let _ = h.frame(&mut app, vec![release(Key::Tab)]);
    assert!(!app.keyboard.navigating(), "a key's release turned it on");
    // A key's press turns it on; the pointer's release then leaves it on.
    let _ = h.frame(&mut app, crate::capture::key(UNBOUND, NONE)[0].clone());
    assert!(app.keyboard.navigating(), "the key turned it on");
    let _ = h.frame(&mut app, vec![button(centre, false)]);
    assert!(
        app.keyboard.navigating(),
        "the pointer's release turned it off"
    );
    // A key egui gives as its own event (Cmd+C, X, V) or as text alone turns it on too.
    for event in [
        Event::Text("é".to_owned()),
        Event::Copy,
        Event::Cut,
        Event::Paste("p".to_owned()),
    ] {
        click_at(&mut h, &mut app, centre);
        assert!(!app.keyboard.navigating(), "the click turned it off");
        let what = format!("{event:?}");
        let _ = h.frame(&mut app, vec![event]);
        assert!(app.keyboard.navigating(), "{what} left it off");
    }
    let on = app.keyboard.navigating();
    rejects("a flag that never turns on", || assert!(!on));
}

#[test]
fn mock_keyboard_nav_state_the_hidden_layer_and_present_mode_follow_it() {
    // The egui layer hidden (F3): a click turns it off and a key on, though the layer takes no keys.
    let (mut app, mut h) = fresh();
    press(&mut h, &mut app, Key::Tab, NONE);
    press(&mut h, &mut app, Key::F3, NONE);
    assert!(!app.shown, "F3 hid the layer");
    assert!(app.keyboard.navigating());
    let centre = layout(&h).figure.center();
    click_at(&mut h, &mut app, centre);
    let off = app.keyboard.navigating();
    assert!(!off, "a click with the layer hidden left it on");
    press(&mut h, &mut app, UNBOUND, NONE);
    assert!(
        app.keyboard.navigating(),
        "a key with the layer hidden left it off"
    );
    rejects("a hidden layer that keeps the ring through a click", || {
        assert!(off)
    });
    // Present mode: Esc returns, the ring shown on Present, as any key press shows it.
    let (mut app, mut h) = fresh();
    app.present = true;
    click_at(&mut h, &mut app, centre);
    assert!(!app.keyboard.navigating());
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(!app.present, "Esc left present mode");
    let back = seen(&mut h, &mut app);
    assert_eq!(back.focus, path_to(&app, windows::export::PRESENT));
    assert!(back.navigating, "Esc left keyboard navigation off");
}
