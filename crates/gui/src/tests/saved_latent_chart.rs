//! `saved_latent_chart` (render_gui_spec §G2; chart_reference §1.1): on the mock a latent view saves as the oblique chart
//! `(z₀, q₁, q₂)` exactly and reopens bit for bit.
//!
//! **The criteria**, written before the first run:
//! - **At a tilt.** A view at `τ = (30°, 20°)` into one target, under Lean each axis and under Tip as sheet, saved, left for
//!   another preset and reopened: `(z₀, q₁, q₂)` bit for bit, the figure's pixels (the Home button's corner apart) equal to
//!   the view's before the save, its angles 0 in the Tilt section, every control of it disabled, the tooltip the
//!   not-built line, and Enter on each doing nothing, each still a sibling in the keyboard's order.
//! - **The compass** on it, in tilt mode: a drag over the cube, the arrows in interaction mode and a pick of the third
//!   axis's target send no `SetField` at all, and the status line reads the not-built line; a slice still moves it.
//! - **Untilted.** Six targets, the dimensions off the plane; a tilt is sent and the chart stays active over the whole span
//!   of both maps, whatever rounding leaves of a component that is 0; saved again, its parent the saved chart, it reopens bit for bit.
//! - **Home** on it returns its `(z₀, q₁, q₂)` bit for bit after a pan, a slice, a zoom and, untilted, a tilt, the three
//!   section resets together too, each leaving the others' values; the zoom row reads × 1.00 at its scale.
//!
//! **Controls** (through `rejects`): a pair re-orthonormalised, under Lean; a figure of another zoom; the axes
//!   swapped, and a pair off the targets; a Tilt section before
//! the save, enabled; a tilt drag and the arrows before the save; a tilt control gone from the order; a Home that reads the
//! preset's values; a slice's `z₀` taken for home.

use std::sync::Arc;

use eframe::egui::{vec2, Event, Key, Modifiers, Rect};
use engine::contract::sim_config::{Latent, Plane, TiltMap};
use engine::contract::view_ui::KeyMode;

use super::f3_fills_window_mock::canvas;
use super::mock_layout_splitters::drag;
use super::save_view_as_chart::{
    bits, click_lowest, depth, frames, gui, label, listed, save, tilt_view,
};
use super::support::{click_scope, frame_texts, headless, keys_on, press, rejects, FORMAT};
use crate::app::App;
use crate::capture::{render_frame, PIXELS_PER_POINT, SIZE};
use crate::explore::compass;
use crate::explore::manifold_view::reset::{self, SECTIONS};
use crate::explore::manifold_view::saved::{own_angles, NOT_BUILT};
use crate::explore::manifold_view::tilt::{tilt_range, Angles};
use crate::explore::manifold_view::{
    self as mv, add, basis_edit, dot, norm, z0_edit, View, GAMMA, PRESET, TAU1, TAU2, TILT,
    TILT_MAP_LEAN, TILT_MAP_SHEET, TILT_X, TILT_Y,
};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::canvas::MockCanvas;
use crate::mock::MockEngine;
use crate::settings::Store;
use crate::side::{EngineSide, MockSide};

const NONE: Modifiers = Modifiers::NONE;
/// The Tilt section's controls that a chart saved at a tilt dims.
const CONTROLS: [&str; 7] = [
    TAU1,
    TILT_X,
    TAU2,
    TILT_Y,
    GAMMA,
    TILT_MAP_SHEET,
    TILT_MAP_LEAN,
];

fn plane(app: &App<MockSide>) -> Plane {
    app.snapshot().sim.plane.clone()
}

/// A GUI on the mock drawing through `canvas`, its clock frozen.
fn drawn(canvas: &Arc<MockCanvas>) -> (App<MockSide>, Headless) {
    let side = MockSide::new(MockEngine::frozen(), Some(canvas.clone()));
    let mut app = App::new(side, FORMAT);
    let mut h = headless();
    frames(&mut h, &mut app, 3);
    (app, h)
}

/// The figure's pixels, the Home button's corner apart.
fn figure(canvas: &MockCanvas, app: &mut App<MockSide>, h: &mut Headless) -> Vec<u8> {
    let shot = render_frame(canvas, app, h);
    let r = Layout::new(h.screen(), PIXELS_PER_POINT).figure;
    let px = |v: f32| (v * PIXELS_PER_POINT) as usize;
    let mut out = Vec::new();
    for y in px(r.min.y + 40.0)..px(r.max.y) {
        let row = y * SIZE[0] as usize;
        out.extend_from_slice(&shot.rgba[4 * (row + px(r.min.x) + 1)..4 * (row + px(r.max.x) - 1)]);
    }
    out
}

/// Leaves the chart for the preset `z_q0 × z_q1` and reopens "Saved 1" from the picker.
fn leave_and_reopen(h: &mut Headless, app: &mut App<MockSide>, name: &str) {
    click_scope(h, app, PRESET);
    frames(h, app, 2);
    click_lowest(h, app, "z_q0 × z_q1");
    assert_eq!(label(app), None, "left");
    click_scope(h, app, PRESET);
    frames(h, app, 2);
    click_lowest(h, app, name);
    assert_eq!(label(app).as_deref(), Some(name), "reopened");
}

/// The pair re-orthonormalised by Gram–Schmidt, its lengths kept.
fn orthonormalised(p: &Plane) -> Plane {
    let (n1, n2) = (norm(&p.q1), norm(&p.q2));
    let h1 = p.q1.map(|v| v / n1);
    let off = add(&p.q2, -dot(&p.q2, &h1), &h1);
    let n = norm(&off);
    Plane {
        z0: p.z0,
        q1: h1.map(|v| v * n1),
        q2: off.map(|v| v * n2 / n),
    }
}

/// How many of the nodes inside the Tilt section below its header are disabled, and how many take input.
fn tilt_inputs(h: &mut Headless, app: &mut App<MockSide>) -> (usize, usize) {
    let output = h.frame(app, Vec::new());
    let rect = app.keyboard.place_of(TILT).expect("the section").rect;
    let layout = Layout::new(h.screen(), PIXELS_PER_POINT);
    let below = Rect::from_min_max(
        rect.min + vec2(0.0, mv::row_height(layout.manifold_view)),
        rect.max,
    );
    let update = output.platform_output.accesskit_update.expect("a tree");
    use eframe::egui::accesskit::Role;
    let inputs = update.nodes.iter().filter(|(_, n)| {
        matches!(
            n.role(),
            Role::Slider | Role::SpinButton | Role::ComboBox | Role::RadioButton
        ) && n.bounds().is_some_and(|b| {
            below.contains(eframe::egui::pos2(
                ((b.x0 + b.x1) / 2.0) as f32,
                ((b.y0 + b.y1) / 2.0) as f32,
            ))
        })
    });
    let (dim, live): (Vec<_>, Vec<_>) = inputs.partition(|(_, n)| n.is_disabled());
    (dim.len(), live.len())
}

fn check_dimmed(h: &mut Headless, app: &mut App<MockSide>) {
    let (dim, live) = tilt_inputs(h, app);
    assert_eq!(live, 0, "every control dimmed");
    assert!(
        dim >= 10,
        "three sliders, three boxes, two menus, two radios: {dim}"
    );
}

#[test]
fn saved_latent_chart_at_a_tilt_reopens_bitwise() {
    let canvas = canvas();
    for map in [TiltMap::Lean, TiltMap::Sheet] {
        let (mut app, mut h) = drawn(&canvas);
        tilt_view(&mut h, &mut app, map);
        let (was, picture) = (plane(&app), figure(&canvas, &mut app, &mut h));
        rejects("a Tilt section before the save", || {
            check_dimmed(&mut h, &mut app)
        });
        save(&mut h, &mut app);
        leave_and_reopen(&mut h, &mut app, "Saved 1");
        let check = |p: &Plane| assert_eq!(bits(p), bits(&was), "{map:?}");
        check(&plane(&app));
        if map == TiltMap::Lean {
            rejects("a pair re-orthonormalised", || {
                check(&orthonormalised(&was))
            });
        }
        let same = |pixels: &[u8]| assert!(pixels == picture, "{map:?}: the figure moved");
        same(&figure(&canvas, &mut app, &mut h));
        // Its own tilt is none.
        let sim = app.snapshot().sim.clone();
        let own = own_angles(&app.manifold.charts, &sim);
        assert_eq!((own.tau1, own.tau2, own.gamma), (0.0, 0.0, 0.0));
        check_dimmed(&mut h, &mut app);
        // Its tooltip.
        let at = app.keyboard.place_of(TAU2).expect("τ₂").rect.center();
        let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
        frames(&mut h, &mut app, 14);
        assert!(
            frame_texts(&mut h, &mut app).iter().any(|t| t == NOT_BUILT),
            "the tooltip"
        );
        let _ = h.frame(
            &mut app,
            vec![Event::PointerMoved(eframe::egui::pos2(720.0, 5.0))],
        );
        app.set_field(basis_edit((
            sim.plane.q1.map(|v| v * 0.7),
            sim.plane.q2.map(|v| v * 0.7),
        )));
        frames(&mut h, &mut app, 2);
        let zoomed = figure(&canvas, &mut app, &mut h);
        rejects("a figure of another zoom", || same(&zoomed));
        assert_eq!(
            label(&app).as_deref(),
            Some("Saved 1"),
            "a zoom stays on it"
        );
    }
}

/// Puts the focus on scope `id`, keyboard navigation on.
fn focus_on(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(id);
    app.view.focus.mode = KeyMode::Navigation;
    keys_on(h, app);
}

#[test]
fn saved_latent_chart_at_a_tilt_keeps_the_keyboard_order_inert() {
    let (mut app, mut h) = gui(Store::default());
    tilt_view(&mut h, &mut app, TiltMap::Lean);
    let order = app.keyboard.tree(app.view.mode).children(Some(TILT));
    save(&mut h, &mut app);
    let check = |children: &[&str]| assert_eq!(children, order);
    let children = app.keyboard.tree(app.view.mode).children(Some(TILT));
    check(&children);
    rejects("a dimmed control gone from the order", || {
        check(&children[1..])
    });
    let before = (depth(&app), plane(&app));
    for (i, id) in CONTROLS.into_iter().enumerate() {
        // ↓ from the section's first reaches it.
        focus_on(&mut h, &mut app, TAU1);
        for _ in 0..i {
            press(&mut h, &mut app, Key::ArrowDown, NONE);
        }
        assert_eq!(app.view.focus.path.last().map(String::as_str), Some(id));
        let sent = app.side().engine().edits_applied();
        press(&mut h, &mut app, Key::Enter, NONE);
        press(&mut h, &mut app, Key::ArrowRight, NONE);
        frames(&mut h, &mut app, 1);
        assert_eq!(app.view.focus.mode, KeyMode::Navigation, "{id}");
        assert_eq!(app.side().engine().edits_applied(), sent, "{id}");
        assert_eq!((depth(&app), plane(&app)), before, "{id}");
        assert_eq!(
            app.view.focus.path.len(),
            3,
            "{id}: Enter goes into nothing"
        );
    }
    // Off the chart they take the keys again.
    app.undo();
    frames(&mut h, &mut app, 2);
    focus_on(&mut h, &mut app, TAU1);
    press(&mut h, &mut app, Key::Enter, NONE);
    rejects("a control that takes Enter", || {
        assert_eq!(app.view.focus.mode, KeyMode::Navigation)
    });
}

/// A tilt drag over the cube, then the compass's tilt arrows: how many edits they sent.
fn compass_tilts(h: &mut Headless, app: &mut App<MockSide>) -> u64 {
    app.manifold.compass.mode = mv::Touch::Tilt;
    let sent = app.side().engine().edits_applied();
    let square = compass::square(Layout::new(h.screen(), PIXELS_PER_POINT).compass);
    drag(h, app, square.center(), square.center() + vec2(30.0, 12.0));
    frames(h, app, 2);
    focus_on(h, app, crate::explore::COMPASS_VIEW);
    press(h, app, Key::Enter, NONE);
    press(h, app, Key::ArrowRight, NONE);
    press(h, app, Key::ArrowUp, NONE);
    press(h, app, Key::Escape, NONE);
    frames(h, app, 2);
    app.side().engine().edits_applied() - sent
}

#[test]
fn saved_latent_chart_at_a_tilt_refuses_the_compass() {
    let (mut app, mut h) = gui(Store::default());
    tilt_view(&mut h, &mut app, TiltMap::Sheet);
    let none = |sent: u64| assert_eq!(sent, 0, "no SetField");
    let says = |texts: &[String]| assert!(texts.iter().any(|t| t == NOT_BUILT), "{texts:?}");
    let sent = compass_tilts(&mut h, &mut app);
    rejects("a tilt drag and the arrows before the save", || none(sent));
    tilt_view(&mut h, &mut app, TiltMap::Sheet);
    save(&mut h, &mut app);
    let quiet = frame_texts(&mut h, &mut app);
    rejects("a status line with nothing refused", || says(&quiet));
    none(compass_tilts(&mut h, &mut app));
    says(&frame_texts(&mut h, &mut app));
    // The third axis's pick, which is a target's here.
    let snapshot = app.snapshot().clone();
    let mut out = mv::Out::default();
    for id in compass::AXIS_BUTTONS[2].2 {
        mv::activate(id, &snapshot, &mut app.manifold, &mut out);
    }
    assert!(out.edits.is_empty() && out.commands.is_empty(), "{out:?}");
    // A slice moves it, stays on it, and clears the line.
    app.manifold.compass.mode = mv::Touch::Slice;
    let sent = app.side().engine().edits_applied();
    let square = compass::square(Layout::new(h.screen(), PIXELS_PER_POINT).compass);
    drag(
        &mut h,
        &mut app,
        square.center(),
        square.center() + vec2(30.0, 12.0),
    );
    frames(&mut h, &mut app, 2);
    assert!(app.side().engine().edits_applied() > sent, "a slice");
    assert_eq!(label(&app).as_deref(), Some("Saved 1"));
    let after = frame_texts(&mut h, &mut app);
    assert!(after.iter().all(|t| t != NOT_BUILT), "cleared");
    // And it is saved again from there, its parent the saved chart.
    let was = plane(&app);
    save(&mut h, &mut app);
    assert_eq!(listed(&app), ["Saved 1", "Saved 2"]);
    leave_and_reopen(&mut h, &mut app, "Saved 2");
    assert_eq!(bits(&plane(&app)), bits(&was));
}

/// The tilt to `angles` on the app's view, as the Tilt section sends it.
fn tilt_to(h: &mut Headless, app: &mut App<MockSide>, angles: Angles) {
    let sim = &app.snapshot().sim;
    let from = View::read(sim);
    let class = crate::dispatch::Class::Tilt { from, to: angles };
    let edit = crate::dispatch::one(&sim.plane, class);
    app.set_field(edit);
    frames(h, app, 2);
}

#[test]
fn saved_latent_chart_untilted_tilts_and_saves_again() {
    let (mut app, mut h) = gui(Store::default());
    save(&mut h, &mut app);
    let (dim, live) = tilt_inputs(&mut h, &mut app);
    assert_eq!(dim, 0, "{live} live");
    let tree = app.keyboard.tree(app.view.mode);
    let want: Vec<&str> = (2..8).map(|i| mv::TILT_X_ENTRIES[i]).collect();
    assert_eq!(tree.children(Some(TILT_X)), want, "six targets");
    // Active over both maps' spans.
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        // As the radio sends it: a frame edit alone would keep the other map's basis, whose axes are no tilt's under this.
        if let Some(edit) = mv::tilt::map_edit(&app.snapshot().sim, map) {
            app.set_field(edit);
            frames(&mut h, &mut app, 1);
        }
        assert_eq!(label(&app).as_deref(), Some("Saved 1"), "{map:?}");
        let (lo, hi) = tilt_range(map);
        for i in 0..=12 {
            for j in 0..=12 {
                let at = |k: i32| lo + (hi - lo) * f64::from(k) / 12.0;
                let to = Angles {
                    tau1: at(i),
                    tau2: at(j),
                    gamma: 25.0,
                };
                tilt_to(&mut h, &mut app, to);
                assert_eq!(label(&app).as_deref(), Some("Saved 1"), "{map:?} {to:?}");
                // Whatever the platform's rounding leaves of a component that is 0.
                let sim = app.snapshot().sim.clone();
                for by in [0.0, -1.0, 2.0, -3.0] {
                    let rounded = |v: f64| if v.abs() < 1e-15 { by * v } else { v };
                    let mut sim = sim.clone();
                    sim.plane.q1 = sim.plane.q1.map(rounded);
                    sim.plane.q2 = sim.plane.q2.map(rounded);
                    let name = app.manifold.charts.label(&sim);
                    assert_eq!(name, Some("Saved 1"), "{map:?} {to:?} {by}");
                }
            }
        }
    }
    // The axes swapped, and a pair off the targets, are other charts.
    let sim = app.snapshot().sim.clone();
    let off = |q1: Latent, q2: Latent| {
        let mut sim = sim.clone();
        (sim.plane.q1, sim.plane.q2) = (q1, q2);
        assert_eq!(app.manifold.charts.label(&sim), Some("Saved 1"));
    };
    let (e, third) = (|i| mv::axis(i, 1.0), mv::axis(6, 0.5));
    off(e(0), add(&e(1), 1.0, &e(5)));
    rejects("the axes swapped", || off(e(1), e(0)));
    rejects("a pair off the targets", || {
        off(e(5), add(&e(5), 1.0, &third))
    });
    let to = Angles {
        tau1: 30.0,
        tau2: 20.0,
        gamma: 10.0,
    };
    tilt_to(&mut h, &mut app, to);
    let was = plane(&app);
    save(&mut h, &mut app);
    assert_eq!(label(&app).as_deref(), Some("Saved 2"));
    let shown = frame_texts(&mut h, &mut app);
    assert!(
        shown
            .iter()
            .any(|t| t.contains("from Saved 1 · Lean each axis: τ₁ 30.0°")),
        "{shown:?}"
    );
    leave_and_reopen(&mut h, &mut app, "Saved 2");
    let check = |p: &Plane| assert_eq!(bits(p), bits(&was));
    check(&plane(&app));
    rejects("a pair re-orthonormalised", || {
        check(&orthonormalised(&was))
    });
}

/// `z₀` moved along the pair and off it, the pair zoomed.
fn moved(h: &mut Headless, app: &mut App<MockSide>) {
    let p = plane(app);
    let off: Latent = [0.0, 0.0, 0.1, -0.2, 0.05, 0.0, 0.02, 0.0];
    let z0 = add(&add(&add(&p.z0, 0.3, &p.q1), -0.2, &p.q2), 1.0, &off);
    app.set_field(z0_edit(z0));
    app.set_field(basis_edit((p.q1.map(|v| v * 0.25), p.q2.map(|v| v * 0.25))));
    frames(h, app, 2);
}

#[test]
fn saved_latent_chart_home_returns_the_saved_view() {
    for tilted in [false, true] {
        let (mut app, mut h) = gui(Store::default());
        // Off the preset's home, so its home is not the saved one.
        moved(&mut h, &mut app);
        if tilted {
            tilt_view(&mut h, &mut app, TiltMap::Lean);
            let p = plane(&app);
            app.set_field(z0_edit(p.z0.map(|v| v + 0.01)));
            frames(&mut h, &mut app, 2);
        }
        save(&mut h, &mut app);
        let was = plane(&app);
        let sim = app.snapshot().sim.clone();
        let charts = &app.manifold.charts;
        assert!(!reset::home_enabled(charts.at(&sim)), "at home, dimmed");
        assert!(frame_texts(&mut h, &mut app).iter().any(|t| t == "× 1.00"));
        let leave = |h: &mut Headless, app: &mut App<MockSide>| {
            moved(h, app);
            if !tilted {
                let to = Angles {
                    tau1: 15.0,
                    tau2: -40.0,
                    gamma: 5.0,
                };
                tilt_to(h, app, to);
            }
            assert_eq!(label(app).as_deref(), Some("Saved 1"));
        };
        leave(&mut h, &mut app);
        let check = |p: &Plane| assert_eq!(bits(p), bits(&was), "tilted {tilted}");
        rejects("a view off home taken for it", || check(&plane(&app)));
        let by_preset = {
            let sim = &app.snapshot().sim;
            let mut after = sim.plane.clone();
            after = compass::after(&after, &reset::home(sim));
            after
        };
        rejects("a Home of the preset's values", || check(&by_preset));
        press(&mut h, &mut app, Key::Home, NONE);
        frames(&mut h, &mut app, 2);
        check(&plane(&app));
        assert_eq!(label(&app).as_deref(), Some("Saved 1"));
        // Each section's reset is its own: the other two keep their values, and the three compose to Home.
        leave(&mut h, &mut app);
        let values = |app: &App<MockSide>| {
            let sim = app.snapshot().sim.clone();
            SECTIONS.map(|s| reset::enabled(app.manifold.charts.at(&sim), s))
        };
        let live = if tilted {
            [true, false, true]
        } else {
            [true; 3]
        };
        assert_eq!(values(&app), live);
        for (i, section) in SECTIONS.into_iter().enumerate() {
            click_scope(&mut h, &mut app, section.scope());
            frames(&mut h, &mut app, 2);
            let mut want = live;
            want[..=i].fill(false);
            assert_eq!(values(&app), want, "{section:?}, tilted {tilted}");
        }
        let p = plane(&app);
        let far = |a: &Latent, b: &Latent| norm(&add(a, -1.0, b)) <= 1e-12;
        let near = |p: &Plane| {
            assert!(far(&p.z0, &was.z0) && far(&p.q1, &was.q1) && far(&p.q2, &was.q2));
        };
        near(&p);
        rejects("a slice's z₀ taken for home", || {
            let mut p = p.clone();
            p.z0[2] += 1e-6;
            near(&p)
        });
    }
}
