//! QA tests for TASK-M8-86, where the measurement panel meets the console's height (R-463 K1, K2): the panel is beside
//! the page (R-627, the app runs in the window less the panel's strip), so a console over the whole page, maximised or
//! dragged to the top, ends at the strip: it covers none of the panel, by the pointer or by the keys. Its drag handle
//! stops at the strip too. The panels it does cover are left to the keys as without a session, and all come back when it
//! closes. On both pages, as both trees hold the panel. gui takes no dependency on validation (R-187), so each check
//! runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M8-86_console` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, pos2, vec2, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::view_ui::Mode;
use gui::app::App;
use gui::capture::{self, FORMAT};
use gui::explore::breadcrumb::RING_WIDTH;
use gui::explore::footer::FOOTER;
use gui::explore::top_bar::TOP_BAR;
use gui::layout::Layout;
use gui::measure::{
    split, Environment, Measured, Phase, Run, Session, PANEL_SCOPE, RUN_IDLE, RUN_PAN,
};
use gui::mock::MockEngine;
use gui::side::MockSide;
use gui::windows::console::WINDOW;

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

/// The window: the app's 1440 × 900 and the strip.
fn window() -> Rect {
    Rect::from_min_max(pos2(0.0, 0.0), pos2(1740.0, 900.0))
}

fn app_part() -> Rect {
    split(window()).0
}

fn strip() -> Rect {
    split(window()).1
}

/// How the console comes to cover the page.
#[derive(Clone, Copy, Debug)]
enum Cover {
    Maximised,
    Dragged,
}

const COVERS: [Cover; 2] = [Cover::Maximised, Cover::Dragged];

struct Rig {
    ctx: egui::Context,
    measured: Measured<MockSide>,
}

impl Rig {
    fn new(mode: Mode) -> Self {
        let session = Session::new(
            Environment::native(Some("1"), None, None, Some(60.0))
                .expect("flags")
                .expect("open"),
        );
        Self::with(mode, Some(session))
    }

    fn with(mode: Mode, session: Option<Session>) -> Self {
        let app = App::new(MockSide::new(MockEngine::new(), None), FORMAT);
        let mut rig = Self {
            ctx: egui::Context::default(),
            measured: Measured::new(app, session, Box::new(|_| {})),
        };
        rig.measured.app.view.mode = mode;
        rig.settle();
        rig
    }

    /// One frame with `events`; the focus rings drawn.
    fn step(&mut self, events: Vec<Event>) -> Vec<Rect> {
        let mut raw = egui::RawInput {
            screen_rect: Some(window()),
            events,
            ..Default::default()
        };
        self.measured.raw_input_hook(&self.ctx, &mut raw);
        let mut out = self.ctx.run_ui(raw, |ui| self.measured.ui(ui));
        out.textures_delta.clear();
        fn collect(shape: &egui::Shape, rings: &mut Vec<Rect>) {
            match shape {
                egui::Shape::Vec(v) => v.iter().for_each(|s| collect(s, rings)),
                egui::Shape::Rect(r) if r.stroke.width == RING_WIDTH => rings.push(r.rect),
                _ => {}
            }
        }
        let mut rings = Vec::new();
        out.shapes
            .iter()
            .for_each(|c| collect(&c.shape, &mut rings));
        rings
    }

    fn settle(&mut self) {
        for _ in 0..3 {
            self.step(Vec::new());
        }
    }

    fn press(&mut self, key: Key, modifiers: Modifiers) {
        for events in capture::key(key, modifiers) {
            self.step(events);
        }
    }

    fn click(&mut self, at: Pos2) {
        for events in capture::click(at) {
            self.step(events);
        }
        self.settle();
    }

    /// A primary drag from `from` by `by`.
    fn drag(&mut self, from: Pos2, by: f32) {
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let to = from + vec2(0.0, by);
        self.step(vec![Event::PointerMoved(from)]);
        self.step(vec![button(from, true)]);
        for k in 1..=4 {
            self.step(vec![Event::PointerMoved(
                from + vec2(0.0, by * k as f32 / 4.0),
            )]);
        }
        self.step(vec![button(to, false)]);
        self.step(vec![Event::PointerGone]);
        self.settle();
    }

    /// The console's rect as drawn this frame.
    fn console(&self) -> Rect {
        let place = self.measured.app.keyboard.place_of(WINDOW);
        place.expect("the console is drawn").rect
    }

    fn open_console(&mut self) {
        self.measured.app.console_open = true;
        self.settle();
    }

    /// The console opened and brought over the page.
    fn cover(&mut self, cover: Cover) {
        self.open_console();
        match cover {
            Cover::Maximised => self.measured.app.windows.console.maximised = true,
            Cover::Dragged => {
                let from = pos2(200.0, self.console().min.y);
                self.drag(from, -window().height());
                assert!(!self.measured.app.windows.console.maximised);
            }
        }
        self.settle();
    }

    fn focus(&self) -> Vec<&str> {
        let path = &self.measured.app.view.focus.path;
        path.iter().map(String::as_str).collect()
    }

    fn focus_on(&mut self, id: &str) {
        self.measured.app.view.focus.path = vec![id.to_owned()];
    }

    fn phase(&self) -> &Phase {
        &self.measured.session.as_ref().expect("the session").phase
    }

    /// The big scopes Tab walks through from where the focus is, the walk's end included, for `count` presses.
    fn walk(&mut self, count: usize) -> Vec<String> {
        let mut seen = Vec::new();
        for _ in 0..count {
            self.press(Key::Tab, Modifiers::NONE);
            seen.push(self.focus().first().expect("a focus").to_string());
        }
        seen
    }

    fn bigs(&self) -> Vec<&'static str> {
        let mode = self.measured.app.view.mode;
        self.measured.app.keyboard.tree(mode).bigs()
    }
}

/// `console` covers the page of the app's part and none of the strip.
fn check_beside(console: Rect) {
    let page = Layout::new(app_part(), 1.0).page;
    assert!(
        console.min.y <= page.min.y + 0.5,
        "not over the page: {console:?}, the page {page:?}"
    );
    assert!(
        console.max.x <= strip().min.x + 0.5,
        "on the strip: {console:?}, the strip {:?}",
        strip()
    );
}

#[test]
fn qa_m8_86_console_over_the_page_ends_at_the_panels_strip() {
    for mode in [Mode::Explore, Mode::Stain] {
        for cover in COVERS {
            let mut rig = Rig::new(mode);
            rig.cover(cover);
            let console = rig.console();
            check_beside(console);
            let mut wide = console;
            wide.max.x = window().max.x;
            rejects("a console over the strip", || check_beside(wide));
            let mut short = console;
            short.min.y += 100.0;
            rejects("a console short of the top", || check_beside(short));
        }
    }
}

/// The panel's buttons take a click with the console over the page: it does not cover them, and a click on the console
/// beside them starts nothing.
#[test]
fn qa_m8_86_panel_buttons_click_beside_a_covering_console() {
    for cover in COVERS {
        let mut rig = Rig::new(Mode::Explore);
        rig.cover(cover);
        let console = rig.console().center();
        rig.click(console);
        assert!(
            matches!(rig.phase(), Phase::Ready),
            "a click on the console"
        );

        let place = rig.measured.app.keyboard.place_of(RUN_IDLE);
        rig.click(place.expect("the panel's button").rect.center());
        assert!(
            matches!(rig.phase(), Phase::Running { run: Run::Idle, .. }),
            "{cover:?}: {:?}",
            rig.phase()
        );
    }
}

/// The console's handle is on its top edge, so it stops at the strip: a press on the strip at that height leaves the
/// height as it is, where the same drag on the app's part moves it.
#[test]
fn qa_m8_86_console_edge_stops_at_the_strip() {
    let mut rig = Rig::new(Mode::Explore);
    rig.open_console();
    let top = rig.console().min.y;
    rig.drag(pos2(strip().center().x, top), -100.0);
    assert_eq!(rig.console().min.y, top, "a drag on the strip");
    assert_eq!(rig.measured.app.splits.console_top(), None);

    rig.drag(pos2(200.0, top), -100.0);
    assert!(
        (rig.console().min.y - (top - 100.0)).abs() <= 1.0,
        "the same drag on the app's part: {} from {top}",
        rig.console().min.y
    );
}

/// The keys inside the panel on `rig`, its console over the page: from the footer to the panel, the ring on the strip,
/// the arrows, M inert, and Enter on the second button runs it.
fn check_panel_keys(rig: &mut Rig) {
    rig.focus_on(FOOTER);
    rig.press(Key::Tab, Modifiers::NONE);
    assert_eq!(rig.focus(), [PANEL_SCOPE], "Tab from the footer");
    let rings = rig.step(Vec::new());
    assert_eq!(rings.len(), 1, "one ring: {rings:?}");
    assert!(strip().contains(rings[0].center()), "{rings:?}");

    rig.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(rig.focus(), [FOOTER], "the arrow out of the panel");
    rig.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(rig.focus(), [PANEL_SCOPE], "the arrow into the panel");

    let maximised = rig.measured.app.windows.console.maximised;
    rig.press(Key::M, Modifiers::NONE);
    assert_eq!(rig.measured.app.windows.console.maximised, maximised);
    assert_eq!(rig.focus(), [PANEL_SCOPE], "M");

    rig.press(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.focus(), [PANEL_SCOPE, RUN_PAN], "Enter goes in");
    rig.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(rig.focus(), [PANEL_SCOPE, RUN_IDLE], "↓ moves on");
    rig.settle();
    assert_eq!(
        rig.focus(),
        [PANEL_SCOPE, RUN_IDLE],
        "a button's focus stays"
    );
    rig.press(Key::Enter, Modifiers::NONE);
    rig.settle();
    assert!(
        matches!(rig.phase(), Phase::Running { run: Run::Idle, .. }),
        "the idle run: {:?}",
        rig.phase()
    );
}

/// With the console over the page the keys go on working inside the panel, on both pages: the ring is on the strip, the
/// arrows move among its buttons and from the footer to it, Enter runs, and M, the console's own key, is not the
/// panel's. The same check refuses an app with no panel, where the keys have nowhere to go, and a panel whose scripted
/// pan is already running, where Enter on the idle button starts nothing.
#[test]
fn qa_m8_86_panel_keys_work_inside_it_beside_a_covering_console() {
    for mode in [Mode::Explore, Mode::Stain] {
        for cover in COVERS {
            let mut rig = Rig::new(mode);
            rig.cover(cover);
            check_panel_keys(&mut rig);

            let mut bare = Rig::with(mode, None);
            bare.cover(cover);
            rejects("an app with no panel", || check_panel_keys(&mut bare));

            let mut busy = Rig::new(mode);
            busy.cover(cover);
            let pan = busy.measured.app.keyboard.place_of(RUN_PAN);
            busy.click(pan.expect("the panel's button").rect.center());
            rejects("a run already under way", || check_panel_keys(&mut busy));
            assert!(
                matches!(busy.phase(), Phase::Running { run: Run::Pan, .. }),
                "the pan runs on: {:?}",
                busy.phase()
            );
        }
    }
}

/// `walk`, over the cycle's length, reads `want` from the panel.
fn check_walk(rig: &mut Rig, want: &[&str]) {
    rig.focus_on(PANEL_SCOPE);
    assert_eq!(rig.walk(want.len()), want);
}

/// The session changes nothing of what the console leaves: a focus on a panel it covers moves to it, Tab reaches the
/// bars and the measurement panel alone, on either page; and when it closes every big scope is reached again, the
/// measurement panel last.
#[test]
fn qa_m8_86_a_covering_console_leaves_the_bars_and_the_panel_and_gives_the_rest_back() {
    const LEFT: [&str; 3] = [TOP_BAR, FOOTER, PANEL_SCOPE];
    for mode in [Mode::Explore, Mode::Stain] {
        for cover in COVERS {
            let mut rig = Rig::new(mode);
            let every = rig.bigs();
            assert!(every.len() > LEFT.len(), "{mode:?}: {every:?}");
            assert_eq!(every.last(), Some(&PANEL_SCOPE));
            let covered = *every
                .iter()
                .find(|big| !LEFT.contains(big))
                .expect("a panel under the console");

            rig.focus_on(covered);
            rig.cover(cover);
            assert_eq!(rig.focus().first(), Some(&WINDOW), "{covered} {cover:?}");

            check_walk(&mut rig, &LEFT);
            rejects("a walk over the panels it covers", || {
                check_walk(&mut rig, &every);
            });

            rig.measured.app.console_open = false;
            rig.settle();
            check_walk(&mut rig, &every);
            rejects("a walk still left to the bars", || {
                check_walk(&mut rig, &LEFT);
            });
        }
    }
}
