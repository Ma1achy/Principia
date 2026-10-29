//! REQ-GUI-190 (R-450 A5, A7; render_gui_spec §G3): every visible control reachable by §G3's keys, whatever the
//! state, checked with the reusable walk (`reachability`): the console in each of its states, every other window (Run,
//! Chart builder, Export & share, Display, Profiler, the Inspector frame) and every big scope of both pages, a kept
//! orbit and an open menu among their states. The negative arm runs the same check on fixtures it must reject: a drawn
//! control left out of the scope tree, and a scope registered only when the list is non-empty.

use eframe::egui::Key;
use engine::contract::view_ui::{Focus, KeyMode, Mode, Window};

use super::reachability::{check_reached, check_same_order, drawn_controls, new_controls, walk};
use super::support::{headless, mock_app, nodes, press, rejects, Node};
use crate::app::App;
use crate::headless::Headless;
use crate::keyboard::scopes::{Scope, ScopeId, ScopeTree};
use crate::side::{EngineSide, MockSide};
use crate::windows::console::{self, Filter, CLEAR, CLOSE, COPY, HEADER, LOG, SEARCH, WINDOW};
use crate::windows::{entry, REGISTRY};

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..3 {
        let _ = h.frame(app, Vec::new());
    }
}

fn fresh() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

fn nav(path: Vec<String>) -> Focus {
    Focus {
        path,
        mode: KeyMode::Navigation,
    }
}

/// A window's walk and the controls it drew: those the frame drew once it opened that it had not drawn before, inside
/// the window's rect.
struct Opened {
    reached: Vec<String>,
    controls: Vec<Node>,
}

/// Opens `window` on `app`, then `before_walk` sets its state; returns its walk from its landing and its controls.
fn open(
    app: &mut App<MockSide>,
    h: &mut Headless,
    window: Window,
    before_walk: impl FnOnce(&mut App<MockSide>, &mut Headless),
) -> Opened {
    let ctx = h.ctx.clone();
    open_by(
        app,
        h,
        window,
        |app, _| app.open_window(&ctx, window, None),
        before_walk,
    )
}

/// As [`open`], the window opened by `opening`.
fn open_by(
    app: &mut App<MockSide>,
    h: &mut Headless,
    window: Window,
    opening: impl FnOnce(&mut App<MockSide>, &mut Headless),
    before_walk: impl FnOnce(&mut App<MockSide>, &mut Headless),
) -> Opened {
    let before = nodes(h, app);
    opening(app, h);
    settle(h, app);
    assert!(app.is_open(window), "{window:?} opened");
    before_walk(app, h);
    settle(h, app);
    let after = nodes(h, app);
    let e = entry(window);
    let rect = app
        .keyboard
        .place_of(e.scope)
        .expect("the window placed")
        .rect;
    let controls = new_controls(&before, &after)
        .into_iter()
        .filter(|n| rect.contains(n.rect.center()))
        .collect();
    let tree = app.keyboard.tree(app.view.mode);
    let reached = walk(tree, &nav(tree.window_landing(e.scope)), e.scope);
    Opened { reached, controls }
}

fn check_opened(what: &str, app: &App<MockSide>, opened: &Opened) {
    assert!(!opened.controls.is_empty(), "{what}: no control drawn");
    let tree = app.keyboard.tree(app.view.mode);
    check_reached(
        what,
        tree,
        &opened.reached,
        |id| app.keyboard.place_of(id),
        &opened.controls,
    );
}

#[test]
fn mock_reachability_every_window_reaches_every_drawn_control() {
    for e in REGISTRY {
        for mode in [Mode::Explore, Mode::Stain] {
            let (mut app, mut h) = fresh();
            app.view.mode = mode;
            settle(&mut h, &mut app);
            let opened = open(&mut app, &mut h, e.window, |_, _| {});
            check_opened(&format!("{:?} on {mode:?}", e.window), &app, &opened);
            // Its ×, in the scope tree, among them.
            assert!(
                opened.reached.iter().any(|r| r == e.close),
                "{:?}",
                e.window
            );
        }
    }
}

/// Presses `key` `n` times.
fn presses(h: &mut Headless, app: &mut App<MockSide>, key: Key, n: usize) {
    for _ in 0..n {
        press(h, app, key, eframe::egui::Modifiers::NONE);
    }
}

/// The console's states: as opened from Windows › Console, opened by itself on an error, a filter emptying the list, a
/// long list scrolled to its end, cleared, and its search field typed in.
fn console_states() -> Vec<(&'static str, Opened, App<MockSide>)> {
    type Set = fn(&mut App<MockSide>, &mut Headless);
    let states: [(&str, Set); 6] = [
        ("as opened", |_, _| {}),
        ("opened by itself on an error", |_, _| {}),
        ("a filter emptying the list", |app, _| {
            app.windows.console.filter = Filter::Errors;
        }),
        ("a long list at its end", |app, h| {
            for _ in 0..80 {
                app.side().engine().raise_warning();
                let _ = h.frame(app, Vec::new());
            }
            app.windows.console.top_row = 79;
            app.windows.console.scroll_pending = true;
        }),
        ("cleared", |app, h| {
            // Enter on Clear, by the keys: into the header bar, → to Clear.
            let ctx = h.ctx.clone();
            app.open_window(&ctx, Window::Console, None);
            // Opened with no key, the ring hidden (R-460 H2): the first Enter only shows it (R-464 M2).
            presses(h, app, Key::Enter, 2);
            presses(h, app, Key::ArrowRight, 6);
            presses(h, app, Key::Enter, 1);
            assert!(app.console().is_empty(), "cleared");
        }),
        ("its search field typed in", |app, _| {
            app.windows.console.text = "nothing matches this".to_owned();
        }),
    ];
    states
        .into_iter()
        .map(|(what, set)| {
            let (mut app, mut h) = fresh();
            app.side().engine().raise_warning();
            settle(&mut h, &mut app);
            let opened = if what == "opened by itself on an error" {
                let opening = |app: &mut App<MockSide>, _: &mut Headless| {
                    app.side().engine().raise_error();
                };
                open_by(&mut app, &mut h, Window::Console, opening, set)
            } else {
                open(&mut app, &mut h, Window::Console, set)
            };
            (what, opened, app)
        })
        .collect()
}

#[test]
fn mock_reachability_the_console_in_each_state_reaches_every_control_in_one_order() {
    let states = console_states();
    let first = &states[0].1.reached;
    for (what, opened, app) in &states {
        check_opened(&format!("the console, {what}"), app, opened);
        check_same_order(what, &opened.reached, first);
        for id in [HEADER, LOG, SEARCH, COPY, CLEAR, CLOSE] {
            assert!(opened.reached.iter().any(|r| r == id), "{what}: {id}");
        }
    }
    assert_eq!(first[0], HEADER, "the walk starts on the landing");
}

/// The big scopes of `mode`, each walked from itself, the controls drawn in its rect each reached.
fn check_bigs(what: &str, app: &mut App<MockSide>, h: &mut Headless) {
    let all = nodes(h, app);
    let tree = app.keyboard.tree(app.view.mode);
    for big in tree.bigs() {
        let Some(place) = app.keyboard.place_of(big) else {
            continue;
        };
        let controls = drawn_controls(&all, place.rect);
        let reached = walk(tree, &nav(vec![big.to_owned()]), big);
        check_reached(
            &format!("{what}: {big}"),
            tree,
            &reached,
            |id| app.keyboard.place_of(id),
            &controls,
        );
    }
}

#[test]
fn mock_reachability_every_big_scope_reaches_every_drawn_control() {
    for mode in [Mode::Explore, Mode::Stain] {
        let (mut app, mut h) = fresh();
        app.view.mode = mode;
        settle(&mut h, &mut app);
        check_bigs(&format!("{mode:?}"), &mut app, &mut h);
        let placed = app
            .keyboard
            .tree(mode)
            .bigs()
            .into_iter()
            .filter(|b| app.keyboard.place_of(b).is_some())
            .count();
        assert!(placed > 3, "{mode:?}: {placed} big scopes placed");
    }
    // A kept orbit: its row's controls, the × among them.
    let (mut app, mut h) = fresh();
    let figure = crate::layout::Layout::new(h.screen(), h.pixels_per_point()).figure;
    let at = crate::capture::hover_point(figure);
    let _ = h.frame(&mut app, vec![eframe::egui::Event::PointerMoved(at)]);
    settle(&mut h, &mut app);
    let sample = app
        .snapshot()
        .sample
        .clone()
        .expect("a sample under the cursor");
    crate::explore::trajectory::keep(&sample, &mut app.view.kept_orbits, &mut app.trajectory);
    let _ = h.frame(&mut app, vec![eframe::egui::Event::PointerGone]);
    settle(&mut h, &mut app);
    assert_eq!(app.view.kept_orbits.orbits.len(), 1);
    // The audit's finding 2, the kept-orbit tab `#1` outside the scope tree, fixed by TASK-M6-36 (R-453): drawn, and
    // reached with the rest.
    let all = nodes(&mut h, &mut app);
    let tab = all
        .iter()
        .find(|n| n.text == "#1")
        .expect("the kept tab drawn");
    let tree = app.keyboard.tree(app.view.mode);
    let mut reached = Vec::new();
    for big in tree.bigs() {
        reached.extend(walk(tree, &nav(vec![big.to_owned()]), big));
    }
    let missed = super::reachability::unreached(
        tree,
        &reached,
        |id| app.keyboard.place_of(id),
        std::slice::from_ref(tab),
    );
    assert!(missed.is_empty(), "the kept tab unreached: {missed:?}");
    rejects("a walk of no scope", || {
        let missed = super::reachability::unreached(
            tree,
            &[],
            |id| app.keyboard.place_of(id),
            std::slice::from_ref(tab),
        );
        assert!(missed.is_empty(), "{missed:?}")
    });
    check_bigs("a kept orbit", &mut app, &mut h);
}

/// The keys that open `menu` from nothing focused, and the big scope it is under: a top bar control by Tab, Enter, →
/// to it and Enter; a Manifold view control (the Tilt section's targets, R-457 E5) by Tab, Tab, Enter, ↓ to its
/// section, Enter, ↓ to it and Enter.
fn opening(tree: &ScopeTree, menu: &str) -> (Vec<Key>, &'static str) {
    let index = |list: Vec<&str>, id: &str| list.iter().position(|c| *c == id);
    if let Some(k) = index(tree.children(Some("top_bar")), menu) {
        let mut keys = vec![Key::Tab, Key::Enter];
        keys.extend(std::iter::repeat_n(Key::ArrowRight, k));
        keys.push(Key::Enter);
        return (keys, "top_bar");
    }
    let sections = tree.children(Some("manifold_view"));
    let (s, k) = sections
        .iter()
        .enumerate()
        .find_map(|(s, section)| index(tree.children(Some(section)), menu).map(|k| (s, k)))
        .unwrap_or_else(|| panic!("{menu}: neither a top bar nor a Manifold view control"));
    let mut keys = vec![Key::Tab, Key::Tab, Key::Enter];
    keys.extend(std::iter::repeat_n(Key::ArrowDown, s));
    keys.push(Key::Enter);
    keys.extend(std::iter::repeat_n(Key::ArrowDown, k));
    keys.push(Key::Enter);
    (keys, "manifold_view")
}

#[test]
fn mock_reachability_every_menu_reaches_its_entries() {
    let (app, _) = fresh();
    let menus = app.keyboard.tree(Mode::Explore).menus();
    assert!(menus.len() > 3, "{menus:?}");
    for menu in ["tilt_x", "tilt_y"] {
        assert!(menus.contains(&menu), "{menu} a menu: {menus:?}");
    }
    for menu in menus {
        let (keys, within) = opening(app.keyboard.tree(Mode::Explore), menu);
        let (mut app, mut h) = fresh();
        let before = nodes(&mut h, &mut app);
        for key in keys {
            press(&mut h, &mut app, key, eframe::egui::Modifiers::NONE);
        }
        assert_eq!(app.keyboard.open_menu(), Some(menu), "{menu} opened");
        let after = nodes(&mut h, &mut app);
        let entries = new_controls(&before, &after);
        assert!(!entries.is_empty(), "{menu}: no entry drawn");
        let tree = app.keyboard.tree(Mode::Explore);
        let reached = walk(tree, &nav(vec![within.to_owned()]), within);
        check_reached(
            menu,
            tree,
            &reached,
            |id| app.keyboard.place_of(id),
            &entries,
        );
    }
    rejects(
        "a menu neither in the top bar nor in the Manifold view",
        || {
            let _ = opening(app.keyboard.tree(Mode::Explore), "no_such_menu");
        },
    );
}

// --- The negative arm: fixtures the check must reject ----------------------------------------------------------------

/// A console tree for the fixtures: its two scopes and header controls as `console::register` has them, but for the
/// controls in `skip`, and the log registered only when `rows` is not `Some(0)`.
fn fixture(skip: &[ScopeId], rows: Option<usize>) -> ScopeTree {
    let mut tree = ScopeTree::new();
    tree.register(None, Scope::scoped_window(WINDOW, console::TITLE));
    tree.register(Some(WINDOW), Scope::group(HEADER, "Header").row());
    for (id, _, _, label) in console::FILTER_IDS {
        if !skip.contains(&id) {
            tree.register(Some(HEADER), Scope::control(id, label));
        }
    }
    if !skip.contains(&SEARCH) {
        tree.register(Some(HEADER), Scope::text(SEARCH, "Search"));
    }
    for (id, label) in [(COPY, "Copy"), (CLEAR, "Clear"), (CLOSE, "Close")] {
        if !skip.contains(&id) {
            tree.register(Some(HEADER), Scope::control(id, label));
        }
    }
    if rows != Some(0) {
        tree.register(Some(WINDOW), Scope::scroll(LOG, "Log"));
    }
    tree
}

#[test]
fn mock_reachability_negative_arm_a_control_left_out_of_the_tree() {
    let (mut app, mut h) = fresh();
    let opened = open(&mut app, &mut h, Window::Console, |_, _| {});
    // The fixture's own walk, with nothing left out, but the search field (a text field the fixture leaves out):
    // the check names it, and Copy left out too.
    for skip in [&[SEARCH][..], &[SEARCH, COPY][..]] {
        let tree = fixture(skip, None);
        let reached = walk(&tree, &nav(tree.window_landing(WINDOW)), WINDOW);
        rejects(
            &format!("the console with {skip:?} left out of the tree"),
            || {
                check_reached(
                    "fixture",
                    &tree,
                    &reached,
                    |id| app.keyboard.place_of(id),
                    &opened.controls,
                )
            },
        );
    }
    // Every window's × left out: egui's own title-bar ×, as before this task.
    for e in REGISTRY.iter().filter(|e| e.window != Window::Console) {
        let (mut app, mut h) = fresh();
        let opened = open(&mut app, &mut h, e.window, |_, _| {});
        let reached: Vec<String> = opened
            .reached
            .iter()
            .filter(|r| *r != e.close)
            .cloned()
            .collect();
        let tree = app.keyboard.tree(app.view.mode);
        rejects(
            &format!("{:?} with its × outside the tree", e.window),
            || {
                check_reached(
                    "fixture",
                    tree,
                    &reached,
                    |id| app.keyboard.place_of(id),
                    &opened.controls,
                )
            },
        );
    }
}

#[test]
fn mock_reachability_negative_arm_a_scope_registered_only_when_the_list_is_non_empty() {
    let walk_of = |tree: &ScopeTree| walk(tree, &nav(tree.window_landing(WINDOW)), WINDOW);
    let full = walk_of(&fixture(&[], Some(3)));
    let empty = walk_of(&fixture(&[], Some(0)));
    rejects("a log registered only when the list is non-empty", || {
        check_same_order("fixture", &empty, &full)
    });
    // The console's own registration: one order, whatever the list.
    let (app, _) = fresh();
    let own = walk_of(app.keyboard.tree(Mode::Explore));
    check_same_order("console::register", &own, &full);
}
