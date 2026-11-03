//! QA tests for TASK-M6-46, written from REQ-GUI-225 and REQ-GUI-226 and the ruling they port, R-467 (D2, D5, D7,
//! D8), with render_gui_spec §G1, §G2, §G6 and §G12 and gui_state_contract §2, not from the implementation:
//!
//! - REQ-GUI-225, the toggle (`qa_m6_46_debug_view_*`): View › Debug view is off at launch; reached from the keyboard
//!   (Tab to the top bar, Enter, → to View, Enter, ↓ past F3 hide, Enter) and clicked, each toggle sends one
//!   "SetField DebugView before → after", one undo entry, and Ctrl+Z restores it, the sim key untouched (nothing
//!   re-integrated); it is kept through F3 and present mode.
//! - REQ-GUI-225, the legend (`qa_m6_46_legend_*`): with the toggle off the outcome legend holds the nine classes and
//!   no "forbidden region" key and no "Debug" key; with it on the last key is "Debug", after the stain's, its three
//!   entries "forbidden (chart)" in the hatch, "decode or sim failure" in `debug_invalid`'s style (the hatch, R-467 F1)
//!   and "NaN / Inf" in a style of its own, each with a count, the forbidden count the flagged region's pixel count
//!   in the window, read off the presentation capture, and 0 with no region flagged in the window.
//! - REQ-GUI-225, the mass-simplex analogue (`qa_m6_46_flagged_pixels_*`): with the hook flagging a region past the
//!   shown rect and F3 off, the presentation capture is the clear colour at exactly the flagged pixels, the debug
//!   capture `debug_invalid` at exactly those pixels, every other pixel equal in both and to the unflagged capture; the
//!   same inside `[0,1]²` with the layer shown (D8: "a region reaching inside `[0,1]²` is drawn there too").
//! - REQ-GUI-226, the warning (`qa_m6_46_inside_region_warns_*`): a region inside `[0,1]²` logs exactly one warning,
//!   severity warn, source app, "decoder bug: chart <name> fails validity inside [0,1]² at pixel (<x>, <y>), (s, t) =
//!   (<s>, <t>); <n> pixels in this view", naming the mock's chart, the first flagged pixel in row order from the
//!   bottom-left and the count, which equals the clear pixels of the presentation capture; a repaint logs nothing more,
//!   a pan logs it once again; a region wholly past the square logs none.
//!
//! The flagged pixels are found from the region and the shown figure's rect (chart_decoder_contract Part 3: the shown
//! rect is `[0, 1]²`, `t` Y-up), each pixel judged at its centre; a pixel within one pixel of the region's edge may
//! fall either way, but it must fall the same way in both views. The hatch is the render contract's `debug_invalid`
//! (REQ-COL-055) through its CPU mirror. gui takes no dependency on validation (R-187), so each check runs its own
//! control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-46` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::sync::Arc;

use eframe::egui::{self, Key, Modifiers};
use engine::contract::canvas::hatch::{hatch, hatch_stripe};
use engine::contract::legend::{Legend, LegendEntry, LegendKey, SwatchStyle};
use engine::contract::log::{LogEntry, Severity, Source};
use engine::contract::set_field::{Edit, RenderField, SetField, SimField};
use engine::contract::sim_config::SimConfig;
use gui::app::{clear_colour, App};
use gui::capture::{self, render_frame, Shot, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::manifold_view::View;
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::canvas::MockCanvas;
use gui::mock::MockEngine;
use gui::side::MockSide;

const W: usize = SIZE[0] as usize;
const H: usize = SIZE[1] as usize;

/// The nine outcome classes as shown (R-444, R-449), in colour_composition §1.4's order.
const CLASSES: [&str; 9] = [
    "bounded / nothing yet",
    "collision 1–2",
    "collision 1–3",
    "collision 2–3",
    "triple collision",
    "escape body 1",
    "escape body 2",
    "escape body 3",
    "triple ejection",
];

/// The key R-467 removes from the presentation legends (render_gui_spec §G6).
const FORBIDDEN_REGION: &str = "forbidden region";

/// The debug view's key and its entries, in order (R-467 D5; render_gui_spec §G2, §G6).
const DEBUG: &str = "Debug";
const DEBUG_ENTRIES: [&str; 3] = ["forbidden (chart)", "decode or sim failure", "NaN / Inf"];

/// A region past the shown rect, left of it (R-467 D8's own example, `-0.5,-0.5,0,1.5`).
const PAST: [f32; 4] = [-0.5, -0.5, 0.0, 1.5];
/// A region inside `[0,1]²`.
const INSIDE: [f32; 4] = [0.4, 0.4, 0.6, 0.6];

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn canvas() -> Arc<MockCanvas> {
    Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"))
}

fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

/// The shown figure's rect in physical pixels, unrounded: `[x0, y0, x1, y1]`, rows from the top.
fn shown_f() -> [f64; 4] {
    let r = layout().figure;
    [r.min.x, r.min.y, r.max.x, r.max.y].map(|v| f64::from(v * PIXELS_PER_POINT))
}

/// The shown figure's rect in whole pixels.
fn shown_px() -> [usize; 4] {
    shown_f().map(|v| v.round() as usize)
}

fn px(rgba: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * W + x) * 4;
    rgba[i..i + 4].try_into().unwrap()
}

fn clear() -> [u8; 4] {
    clear_colour(&egui::Visuals::dark()).to_array()
}

/// The hatch at the capture's pixel `(x, y)`: `debug_invalid` at the pixel's centre (REQ-COL-055).
fn hatch_at(x: usize, y: usize) -> [u8; 4] {
    let [r, g, b] = hatch().colours[hatch_stripe([x as f64 + 0.5, y as f64 + 0.5])];
    [r, g, b, 255]
}

/// An app on the frozen mock drawing through `canvas`, settled.
fn app_on(canvas: Option<Arc<MockCanvas>>) -> (App<MockSide>, Headless) {
    let mut app = App::new(MockSide::new(MockEngine::frozen(), canvas), FORMAT);
    let mut h = headless();
    settle(&mut h, &mut app);
    (app, h)
}

/// Frames enough for an edit to reach the snapshot.
fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers) {
    for events in capture::key(key, modifiers) {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

fn set(h: &mut Headless, app: &mut App<MockSide>, edit: Edit) {
    app.set_field(SetField {
        edit,
        no_history: false,
    });
    settle(h, app);
}

fn debug_view(h: &mut Headless, app: &mut App<MockSide>, on: bool) {
    set(h, app, Edit::Render(RenderField::DebugView(on)));
}

/// Clicks the accessible node named `name`, at its rect's centre.
fn click(h: &mut Headless, app: &mut App<MockSide>, name: &str) {
    let out = h.frame(app, Vec::new());
    let rect = h
        .names(&out)
        .into_iter()
        .find(|n| n.name == name)
        .and_then(|n| n.rect)
        .unwrap_or_else(|| panic!("no node named `{name}` with a rect"));
    let ppp = f64::from(PIXELS_PER_POINT);
    let at = egui::pos2(
        ((rect[0] + rect[2]) / 2.0 / ppp) as f32,
        ((rect[1] + rect[3]) / 2.0 / ppp) as f32,
    );
    for events in capture::click(at) {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

// ── The toggle ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// The console's `SetField DebugView` lines, in order.
fn debug_lines(console: &[LogEntry]) -> Vec<String> {
    console
        .iter()
        .filter(|e| e.message.starts_with("SetField DebugView"))
        .map(|e| e.message.clone())
        .collect()
}

/// What a toggle must leave or change: the debug view, the undo depth, the `SetField DebugView` lines, the sim key.
#[derive(Clone)]
struct Before {
    on: bool,
    depth: u32,
    lines: Vec<String>,
    sim: SimConfig,
}

fn before(app: &App<MockSide>) -> Before {
    Before {
        on: app.snapshot().render.debug_view,
        depth: app.snapshot().history.undo_depth,
        lines: debug_lines(app.console()),
        sim: app.snapshot().sim.clone(),
    }
}

/// One toggle since `b`: the debug view flipped, exactly one new line "SetField DebugView b → !b" (gui_state_contract
/// §2), the undo depth one greater (R-69, as an Overlays ▾ toggle is), and the sim key unchanged, so nothing
/// re-integrates (a render key).
fn check_one_toggle(b: &Before, now: &Before) {
    assert_eq!(now.on, !b.on, "the debug view not toggled");
    assert_eq!(
        now.lines.len(),
        b.lines.len() + 1,
        "not one SetField DebugView: {:?}",
        now.lines
    );
    assert_eq!(
        now.lines.last().map(String::as_str),
        Some(format!("SetField DebugView {} → {}", b.on, !b.on).as_str()),
        "the SetField's log line"
    );
    assert_eq!(now.depth, b.depth + 1, "not one undo entry");
    assert_eq!(now.sim, b.sim, "the sim key changed: a re-integration");
}

/// The keys from launch to View › Debug view: Tab to the top bar, Enter, → to View, Enter (the menu open on its
/// first entry, F3 hide), ↓ to F3 fills by (R-468 E4), ↓ to Debug view; Enter then acts as its click (§G3).
const TO_ENTRY: [Key; 6] = [
    Key::Tab,
    Key::Enter,
    Key::ArrowRight,
    Key::Enter,
    Key::ArrowDown,
    Key::ArrowDown,
];

#[test]
fn qa_m6_46_debug_view_reached_from_the_keyboard() {
    let (mut app, mut h) = app_on(None);
    assert!(!app.snapshot().render.debug_view, "on at launch");
    let b = before(&app);
    for key in TO_ENTRY.into_iter().chain([Key::Enter]) {
        press(&mut h, &mut app, key, Modifiers::NONE);
    }
    let on = before(&app);
    check_one_toggle(&b, &on);
    // Ctrl+Z restores it.
    press(&mut h, &mut app, Key::Z, Modifiers::CTRL);
    assert!(
        !app.snapshot().render.debug_view,
        "Ctrl+Z did not restore it"
    );
    assert_eq!(app.snapshot().history.undo_depth, b.depth, "the undo depth");
    // Control: the same keys without ↓, Enter on F3 hide: the entry not reached, no toggle.
    let (mut app, mut h) = app_on(None);
    let b = before(&app);
    for key in TO_ENTRY[..4].iter().copied().chain([Key::Enter]) {
        press(&mut h, &mut app, key, Modifiers::NONE);
    }
    let now = before(&app);
    rejects("keys that stop on F3 hide", || check_one_toggle(&b, &now));
}

#[test]
fn qa_m6_46_debug_view_clicked_is_one_undoable_set_field_per_toggle() {
    let (mut app, mut h) = app_on(None);
    let b = before(&app);
    click(&mut h, &mut app, "View");
    click(&mut h, &mut app, "Debug view");
    let on = before(&app);
    check_one_toggle(&b, &on);
    click(&mut h, &mut app, "View");
    click(&mut h, &mut app, "Debug view");
    let off = before(&app);
    check_one_toggle(&on, &off);
    press(&mut h, &mut app, Key::Z, Modifiers::CTRL);
    assert!(app.snapshot().render.debug_view, "the first undo");
    press(&mut h, &mut app, Key::Z, Modifiers::CTRL);
    assert!(!app.snapshot().render.debug_view, "the second undo");
    assert_eq!(app.snapshot().history.undo_depth, b.depth);
    // Control: a toggle that adds no undo entry (marked "no history").
    let (mut quiet, mut h) = app_on(None);
    let b = before(&quiet);
    quiet.set_field(SetField {
        edit: Edit::Render(RenderField::DebugView(true)),
        no_history: true,
    });
    settle(&mut h, &mut quiet);
    let now = before(&quiet);
    rejects("a toggle that adds no undo entry", || {
        check_one_toggle(&b, &now)
    });
}

/// View lists Debug view after F3 hide (§G2), as a check entry: its accessible node sits below F3 hide's.
#[test]
fn qa_m6_46_debug_view_entry_follows_f3_hide() {
    let (mut app, mut h) = app_on(None);
    click(&mut h, &mut app, "View");
    let out = h.frame(&mut app, Vec::new());
    let names = h.names(&out);
    let top = |name: &str| {
        names
            .iter()
            .find(|n| n.name == name)
            .and_then(|n| n.rect)
            .unwrap_or_else(|| panic!("no `{name}` in the open View menu"))[1]
    };
    let check =
        |hide: f64, debug: f64| assert!(debug > hide, "Debug view at {debug}, F3 hide at {hide}");
    check(top("F3 hide"), top("Debug view"));
    rejects("the entry above F3 hide", || {
        check(top("Debug view"), top("F3 hide"))
    });
}

#[test]
fn qa_m6_46_debug_view_kept_through_f3_and_present_mode() {
    let (mut app, mut h) = app_on(None);
    debug_view(&mut h, &mut app, true);
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    assert!(app.snapshot().render.debug_view, "lost with F3 off");
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    assert!(app.snapshot().render.debug_view, "lost with F3 back");
    app.present = true;
    settle(&mut h, &mut app);
    assert!(app.snapshot().render.debug_view, "lost in present mode");
    press(&mut h, &mut app, Key::Escape, Modifiers::NONE);
    assert!(!app.present, "Esc left present mode on");
    assert!(
        app.snapshot().render.debug_view,
        "lost leaving present mode"
    );
}

// ── The legend ──────────────────────────────────────────────────────────────────────────────────────────────────────

fn entries(key: &LegendKey) -> Option<(&str, &[LegendEntry])> {
    match key {
        LegendKey::Categorical { title, entries } => Some((title, entries)),
        _ => None,
    }
}

/// The presentation legend (R-467): the outcome key's nine classes, in order; no "forbidden region" entry and no
/// "Debug" key anywhere.
fn check_presentation(legend: &Legend) {
    let (_, outcome) = entries(&legend.keys[0]).expect("the outcome key is categorical");
    let labels: Vec<&str> = outcome.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(labels, CLASSES, "the outcome key");
    for key in &legend.keys {
        if let Some((title, entries)) = entries(key) {
            assert_ne!(title, DEBUG, "a Debug key in the presentation legend");
            assert!(
                entries.iter().all(|e| e.label != FORBIDDEN_REGION),
                "a forbidden region key in the presentation legend"
            );
        }
    }
}

/// The debug view's legend (R-467 D5): the presentation keys first, then "Debug", last, its three entries in order,
/// each with a count, the forbidden one `forbidden` pixels of `figure`, its share theirs, the other two 0 on the mock;
/// the first two in the hatch's style (F1: `debug_invalid` is the hatch), the third a style of its own.
fn check_debug(legend: &Legend, forbidden: u64, figure: u64) {
    let n = legend.keys.len();
    assert!(n >= 2, "no Debug key after the stain's: {legend:?}");
    check_presentation(&Legend {
        keys: legend.keys[..n - 1].to_vec(),
    });
    let (title, debug) = entries(&legend.keys[n - 1]).expect("the Debug key is categorical");
    assert_eq!(title, DEBUG, "the last key's title");
    let labels: Vec<&str> = debug.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(labels, DEBUG_ENTRIES, "the Debug key's entries");
    assert_eq!(
        debug[0].swatch,
        SwatchStyle::Hatch,
        "forbidden (chart)'s style"
    );
    assert_eq!(
        debug[1].swatch,
        SwatchStyle::Hatch,
        "decode or sim failure's style"
    );
    assert!(
        !matches!(
            debug[2].swatch,
            SwatchStyle::Hatch | SwatchStyle::NotYet | SwatchStyle::Flat(_)
        ),
        "NaN / Inf's style is not its own: {:?}",
        debug[2].swatch
    );
    let counts: Vec<Option<u64>> = debug.iter().map(|e| e.count).collect();
    assert_eq!(counts, [Some(forbidden), Some(0), Some(0)], "the counts");
    let share = 100.0 * forbidden as f64 / figure as f64;
    assert!(
        (debug[0].share - share).abs() < 1e-9,
        "forbidden (chart)'s share {} is not {share}",
        debug[0].share
    );
}

/// The pixels of `shot` that are the clear colour.
fn clear_pixels(shot: &Shot) -> Vec<(usize, usize)> {
    let clear = clear();
    (0..H)
        .flat_map(|y| (0..W).map(move |x| (x, y)))
        .filter(|&(x, y)| px(&shot.rgba, x, y) == clear)
        .collect()
}

/// The legend with the debug view on counts, under "forbidden (chart)", the flagged region's pixels in the window,
/// which the presentation capture draws as the clear colour (F3 off: the figure is the whole window, and the stand-in
/// alone draws no clear pixel); 0 with the layer shown, the region being past the shown rect, and 0 with no region.
#[test]
fn qa_m6_46_legend_counts_the_flagged_pixels() {
    let canvas = canvas();
    let figure = (W * H) as u64;
    // No region, F3 off: no clear pixel, so the count read off the capture is the flagged region's alone.
    let (mut app, mut h) = app_on(Some(canvas.clone()));
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    let plain = render_frame(&canvas, &mut app, &mut h);
    assert!(
        clear_pixels(&plain).is_empty(),
        "the stand-in drew the clear colour"
    );
    check_presentation(&app.snapshot().legend);
    debug_view(&mut h, &mut app, true);
    let _ = render_frame(&canvas, &mut app, &mut h);
    settle(&mut h, &mut app);
    check_debug(&app.snapshot().legend, 0, figure);

    canvas.flag_invalid(PAST);
    let (mut app, mut h) = app_on(Some(canvas.clone()));
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    let shot = render_frame(&canvas, &mut app, &mut h);
    settle(&mut h, &mut app);
    let flagged = clear_pixels(&shot).len() as u64;
    assert!(flagged > 0, "the region flagged no pixel");
    check_presentation(&app.snapshot().legend);
    debug_view(&mut h, &mut app, true);
    let _ = render_frame(&canvas, &mut app, &mut h);
    settle(&mut h, &mut app);
    let legend = app.snapshot().legend.clone();
    check_debug(&legend, flagged, figure);
    // Controls: a legend that keeps the forbidden key; a count one off; the Debug key in the presentation legend.
    rejects(
        "a presentation legend that keeps the forbidden region key",
        || {
            let mut l = Legend {
                keys: legend.keys[..legend.keys.len() - 1].to_vec(),
            };
            if let LegendKey::Categorical { entries, .. } = &mut l.keys[0] {
                let mut e = entries[0].clone();
                e.label = FORBIDDEN_REGION.to_owned();
                e.swatch = SwatchStyle::Hatch;
                entries.push(e);
            }
            check_presentation(&l)
        },
    );
    rejects("a forbidden count one off", || {
        check_debug(&legend, flagged + 1, figure)
    });
    rejects("the Debug key in the presentation legend", || {
        check_presentation(&legend)
    });
    // The layer shown: the figure is the shown rect, which the region does not reach.
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    let _ = render_frame(&canvas, &mut app, &mut h);
    settle(&mut h, &mut app);
    let f = shown_px();
    let shown = ((f[2] - f[0]) * (f[3] - f[1])) as u64;
    let legend = app.snapshot().legend.clone();
    let (_, debug) = entries(legend.keys.last().unwrap()).unwrap();
    assert_eq!(
        debug[0].count,
        Some(0),
        "the region counted past the figure"
    );
    assert!(
        debug[0].share == 0.0 && shown > 0,
        "a share with nothing counted"
    );
    canvas.flag_invalid([0.0; 4]);
}

// ── The figure: the mass-simplex analogue ──────────────────────────────────────────────────────────────────────────

/// Where the pixel `(x, y)` stands against `region`: `Some(true)` flagged, `Some(false)` not, `None` within one pixel
/// of its edge. The field point is the pixel's centre, `s` across the shown rect `f`, `t` up it.
fn flagged_at(f: [f64; 4], region: [f32; 4], x: usize, y: usize) -> Option<bool> {
    let (fw, fh) = (f[2] - f[0], f[3] - f[1]);
    let s = (x as f64 + 0.5 - f[0]) / fw;
    let t = 1.0 - (y as f64 + 0.5 - f[1]) / fh;
    let (ds, dt) = (1.0 / fw, 1.0 / fh);
    let [s0, t0, s1, t1] = region.map(f64::from);
    let deep_in = s >= s0 + ds && s < s1 - ds && t >= t0 + dt && t < t1 - dt;
    let far_out = s < s0 - ds || s >= s1 + ds || t < t0 - dt || t >= t1 + dt;
    if deep_in {
        Some(true)
    } else if far_out {
        Some(false)
    } else {
        None
    }
}

/// Over the pixels `area` holds: the presentation capture `pres` is the clear colour at exactly the flagged pixels,
/// the debug capture `dbg` the hatch at exactly those, and every other pixel is `plain`'s in both; a pixel on the
/// region's edge is either, the same in both views. Returns how many pixels were flagged.
fn check_views(plain: &Shot, pres: &Shot, dbg: &Shot, region: [f32; 4], area: [usize; 4]) -> usize {
    let (clear, f) = (clear(), shown_f());
    let mut flagged = 0;
    for y in area[1]..area[3] {
        for x in area[0]..area[2] {
            let (p, d, o) = (
                px(&pres.rgba, x, y),
                px(&dbg.rgba, x, y),
                px(&plain.rgba, x, y),
            );
            let want = flagged_at(f, region, x, y);
            let is = match want {
                Some(true) => true,
                Some(false) => false,
                None => p == clear,
            };
            if is {
                flagged += 1;
                assert_eq!(p, clear, "presentation, flagged ({x}, {y})");
                assert_eq!(d, hatch_at(x, y), "debug view, flagged ({x}, {y})");
            } else {
                assert_eq!(p, o, "presentation, not flagged ({x}, {y})");
                assert_eq!(d, o, "debug view, not flagged ({x}, {y})");
            }
        }
    }
    flagged
}

/// The captures of the frozen mock through `canvas`: unflagged (presentation), and with `region` flagged in the
/// presentation and the debug view, the layer hidden by F3 or shown. With nothing flagged, the debug view draws the
/// figure, the pixels `area` holds, as the presentation view does (D5: "every system pixel exactly as the active stain
/// draws it"); with the layer shown the legend beside it gains the Debug key.
fn three(
    canvas: &Arc<MockCanvas>,
    region: [f32; 4],
    hide: bool,
    area: [usize; 4],
) -> (Shot, Shot, Shot) {
    let take = |debug: bool| {
        let (mut app, mut h) = app_on(Some(canvas.clone()));
        if debug {
            debug_view(&mut h, &mut app, true);
        }
        if hide {
            press(&mut h, &mut app, Key::F3, Modifiers::NONE);
        }
        render_frame(canvas, &mut app, &mut h)
    };
    canvas.flag_invalid([0.0; 4]);
    let plain = take(false);
    let plain_debug = take(true);
    for y in area[1]..area[3] {
        for x in area[0]..area[2] {
            assert_eq!(
                px(&plain.rgba, x, y),
                px(&plain_debug.rgba, x, y),
                "with nothing flagged the debug view changed the figure at ({x}, {y})"
            );
        }
    }
    canvas.flag_invalid(region);
    let (pres, dbg) = (take(false), take(true));
    canvas.flag_invalid([0.0; 4]);
    (plain, pres, dbg)
}

/// R-467 clause 7's second test, on the mock (REQ-GUI-225's verify): the hook flags a region past the shown rect,
/// F3 off (Extend): the presentation capture is the clear colour at exactly the flagged pixels and the debug capture
/// `debug_invalid` at exactly those, every other pixel equal in both.
#[test]
fn qa_m6_46_flagged_pixels_past_the_square_f3_off() {
    let canvas = canvas();
    let (plain, pres, dbg) = three(&canvas, PAST, true, [0, 0, W, H]);
    let n = check_views(&plain, &pres, &dbg, PAST, [0, 0, W, H]);
    assert!(n > 1000, "only {n} pixels flagged");
    // Controls: a presentation figure that hatches the flagged region; a debug view drawing them as the background;
    // the region not drawn at all.
    rejects("a presentation figure that hatches the region", || {
        check_views(&plain, &dbg, &dbg, PAST, [0, 0, W, H]);
    });
    rejects("a debug view drawing the background", || {
        check_views(&plain, &pres, &pres, PAST, [0, 0, W, H]);
    });
    rejects("the region not drawn", || {
        check_views(&plain, &plain, &dbg, PAST, [0, 0, W, H]);
    });
}

/// A region inside `[0,1]²`, the layer shown: its pixels draw as past it (D8), the clear colour in the presentation
/// view and the hatch in the debug view, over the shown rect, every other pixel of the rect unchanged. (Outside the
/// rect the footer shows the decoder-bug warning, and the debug view's legend its Debug key.)
#[test]
fn qa_m6_46_flagged_pixels_inside_the_square() {
    let canvas = canvas();
    let f = shown_px();
    let (plain, pres, dbg) = three(&canvas, INSIDE, false, f);
    let n = check_views(&plain, &pres, &dbg, INSIDE, f);
    assert!(n > 1000, "only {n} pixels flagged");
    rejects("a presentation figure that hatches the region", || {
        check_views(&plain, &dbg, &dbg, INSIDE, f);
    });
}

// ── The decoder-bug warning ─────────────────────────────────────────────────────────────────────────────────────────

/// The console's decoder-bug warnings: severity warn, source app (render_gui_spec §G12; R-467 D7).
fn warnings(console: &[LogEntry]) -> Vec<&LogEntry> {
    console
        .iter()
        .filter(|e| e.message.starts_with("decoder bug"))
        .collect()
}

/// The parts of a warning's message: the chart's name, the pixel, `(s, t)` and the count.
struct Warning {
    chart: String,
    pixel: [usize; 2],
    st: [f64; 2],
    n: u64,
}

/// Reads "decoder bug: chart <name> fails validity inside [0,1]² at pixel (<x>, <y>), (s, t) = (<s>, <t>); <n> pixels
/// in this view", the corpus's text (§G12), or panics.
fn parse(message: &str) -> Warning {
    let fail = || -> ! { panic!("not §G12's warning: {message:?}") };
    let rest = message
        .strip_prefix("decoder bug: chart ")
        .unwrap_or_else(|| fail());
    let (chart, rest) = rest
        .split_once(" fails validity inside [0,1]² at pixel (")
        .unwrap_or_else(|| fail());
    let (pixel, rest) = rest.split_once("), (s, t) = (").unwrap_or_else(|| fail());
    let (st, rest) = rest.split_once("); ").unwrap_or_else(|| fail());
    let n = rest
        .strip_suffix(" pixels in this view")
        .unwrap_or_else(|| fail());
    let number = |text: &str| -> f64 { text.trim().parse().unwrap_or_else(|_| fail()) };
    let pair = |text: &str| -> [f64; 2] {
        let (a, b) = text.split_once(", ").unwrap_or_else(|| fail());
        [number(a), number(b)]
    };
    let [x, y] = pair(pixel);
    Warning {
        chart: chart.to_owned(),
        pixel: [x as usize, y as usize],
        st: pair(st),
        n: n.parse().unwrap_or_else(|_| fail()),
    }
}

/// Checks one warning entry against the presentation capture `pres` of its view: severity warn, source app, the chart
/// `chart`, the count the clear pixels of the shown rect, and the pixel the first of them in row order from the
/// bottom-left (the lowest row on screen, then its leftmost), its `(x, y)` with `y` counted up from the bottom or down
/// from the top (§G12 names the order, not the frame), its `(s, t)` that pixel's centre to within a pixel.
fn check_warning(entry: &LogEntry, chart: &str, pres: &Shot, region: [f32; 4]) {
    assert_eq!(entry.severity, Severity::Warn, "the warning's severity");
    assert_eq!(entry.source, Source::App, "the warning's source");
    let w = parse(&entry.message);
    assert_eq!(w.chart, chart, "the chart named");
    let f = shown_px();
    let clear = clear();
    let shown: Vec<(usize, usize)> = (f[1]..f[3])
        .flat_map(|y| (f[0]..f[2]).map(move |x| (x, y)))
        .filter(|&(x, y)| px(&pres.rgba, x, y) == clear)
        .collect();
    assert_eq!(
        w.n,
        shown.len() as u64,
        "the count against the capture's flagged pixels"
    );
    let bottom = shown.iter().map(|p| p.1).max().expect("a flagged pixel");
    let left = shown
        .iter()
        .filter(|p| p.1 == bottom)
        .map(|p| p.0)
        .min()
        .unwrap();
    assert_eq!(w.pixel[0], left, "the first pixel's x");
    assert!(
        w.pixel[1] == H - 1 - bottom || w.pixel[1] == bottom,
        "the first pixel's y {} is row {bottom} from the top, {} from the bottom",
        w.pixel[1],
        H - 1 - bottom
    );
    let ff = shown_f();
    let (ds, dt) = (1.0 / (ff[2] - ff[0]), 1.0 / (ff[3] - ff[1]));
    let [s0, t0, ..] = region.map(f64::from);
    assert!(
        w.st[0] >= s0 - 1e-4 && w.st[0] <= s0 + ds + 1e-4,
        "the first pixel's s {} is not the region's left edge {s0}",
        w.st[0]
    );
    assert!(
        w.st[1] >= t0 - 1e-4 && w.st[1] <= t0 + dt + 1e-4,
        "the first pixel's t {} is not the region's bottom edge {t0}",
        w.st[1]
    );
}

/// A region inside `[0,1]²`: one warning per view, naming the mock's chart, the first pixel and the count; a repaint
/// without a view change logs nothing more, a pan logs it once again (REQ-GUI-226; R-467 D7).
#[test]
fn qa_m6_46_inside_region_warns_once_per_view() {
    let canvas = canvas();
    canvas.flag_invalid(INSIDE);
    let (mut app, mut h) = app_on(Some(canvas.clone()));
    let pres = render_frame(&canvas, &mut app, &mut h);
    settle(&mut h, &mut app);
    let chart = View::of(&app.snapshot().sim.plane).name();
    let logged: Vec<LogEntry> = warnings(app.console()).into_iter().cloned().collect();
    assert_eq!(logged.len(), 1, "not one warning: {logged:?}");
    check_warning(&logged[0], &chart, &pres, INSIDE);
    // Repaints, and the debug view toggled, change no view: no more warnings.
    for _ in 0..3 {
        let _ = render_frame(&canvas, &mut app, &mut h);
    }
    debug_view(&mut h, &mut app, true);
    debug_view(&mut h, &mut app, false);
    settle(&mut h, &mut app);
    assert_eq!(
        warnings(app.console()).len(),
        1,
        "a repaint logged the warning again"
    );
    // Controls: a warning per frame; a warning naming another pixel than the first.
    rejects("a warning per frame", || {
        let mut per_frame = logged.clone();
        per_frame.extend(logged.iter().cloned());
        assert_eq!(warnings(&per_frame).len(), 1);
    });
    rejects("a warning naming the pixel right of the first", || {
        let w = parse(&logged[0].message);
        let mut e = logged[0].clone();
        e.message = e.message.replacen(
            &format!("pixel ({}, ", w.pixel[0]),
            &format!("pixel ({}, ", w.pixel[0] + 1),
            1,
        );
        check_warning(&e, &chart, &pres, INSIDE)
    });
    rejects("a warning with the count one off", || {
        let w = parse(&logged[0].message);
        let mut e = logged[0].clone();
        e.message = e.message.replace(
            &format!("; {} pixels", w.n),
            &format!("; {} pixels", w.n + 1),
        );
        check_warning(&e, &chart, &pres, INSIDE)
    });
    // A pan, an edit of z₀ along q₁: a new view, logged once again; repaints add nothing.
    let plane = app.snapshot().sim.plane.clone();
    let z0 = std::array::from_fn(|i| plane.z0[i] + 0.05 * plane.q1[i]);
    set(&mut h, &mut app, Edit::Sim(SimField::Z0(z0)));
    let panned = render_frame(&canvas, &mut app, &mut h);
    for _ in 0..3 {
        let _ = render_frame(&canvas, &mut app, &mut h);
    }
    settle(&mut h, &mut app);
    let all: Vec<LogEntry> = warnings(app.console()).into_iter().cloned().collect();
    assert_eq!(
        all.len(),
        2,
        "a pan did not log it exactly once again: {all:?}"
    );
    let chart = View::of(&app.snapshot().sim.plane).name();
    check_warning(&all[1], &chart, &panned, INSIDE);
    canvas.flag_invalid([0.0; 4]);
}

/// A region wholly past the square logs no warning, with the layer shown, F3 off and in the debug view.
#[test]
fn qa_m6_46_past_region_warns_none() {
    let canvas = canvas();
    canvas.flag_invalid(PAST);
    let (mut app, mut h) = app_on(Some(canvas.clone()));
    let _ = render_frame(&canvas, &mut app, &mut h);
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    let _ = render_frame(&canvas, &mut app, &mut h);
    debug_view(&mut h, &mut app, true);
    let _ = render_frame(&canvas, &mut app, &mut h);
    settle(&mut h, &mut app);
    let check =
        |console: &[LogEntry]| assert!(warnings(console).is_empty(), "a warning: {console:?}");
    check(app.console());
    // Control: the same check on a region inside the square.
    canvas.flag_invalid(INSIDE);
    let (mut inside, mut h) = app_on(Some(canvas.clone()));
    let _ = render_frame(&canvas, &mut inside, &mut h);
    settle(&mut h, &mut inside);
    rejects("a region inside the square", || check(inside.console()));
    canvas.flag_invalid([0.0; 4]);
}
