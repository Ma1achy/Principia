//! QA tests for TASK-M6-47, written from REQ-GUI-229 and the ruling it ports, R-468 (E1 to E6), with R-472's X2c,
//! render_gui_spec §G1 and §G2 and gui_state_contract §2, not from the implementation:
//!
//! - The row (`qa_m6_47_row_*`): View holds F3 hide, then one row of Extend and Fit, then Debug view, the options radio
//!   buttons with the chosen one marked; reached from the keyboard, moving chooses nothing and Enter on an option
//!   chooses it; no `SetField`, no undo entry; it acts at the next F3.
//! - Fit's pixels (`qa_m6_47_fit_*`), in a window wider than tall, one taller than wide, and one with an odd margin:
//!   what is drawn is one square, its side the shorter side within one pixel, centred within one pixel, every other
//!   pixel the clear colour; and the square holds the view's `[0, 1]²` at one scale. That is read off the pixels: each
//!   `(s, t)` of the square is compared with the same `(s, t)` of the Extend capture, whose field rect is the shown
//!   layout's, against the wrong readings (stretched, cropped, mirrored). No placement of the implementation's is used.
//! - Fit shows nothing past the square (R-468 E2), in the debug view too, and a hook's region inside the square lands
//!   where `(s, t)` says, the background in the presentation view and the hatch in the debug view (R-467).
//! - The quads (`qa_m6_47_quads_*`), through the app on a window where `k ≥ 2`: Fit's deepest level is greater than
//!   Extend's and the shown layout's; those already computed are kept (R-468 E3); the shown layout's and Extend's tile
//!   bounds are the same after a trip through Fit.
//! - The round trip (`qa_m6_47_f3_twice_*`), in both modes, from a view that is not the launch view and with a splitter
//!   moved: the layout's rects, `(z₀, q₁, q₂)` bit for bit, the capture pixel-identical, no `SetField`, the undo depth
//!   unchanged.
//! - Present mode in Fit equals F3 in Fit, and follows the mode; Extend at each launch, the choice kept for the session.
//!
//! The formula is the one REQ-GUI-229's statement and verify state in words, "finer" at `k ≥ 2`: R-468 E3's written
//! `frac` is not asserted. gui takes no dependency on validation (R-187), so each check runs its own control (R-176)
//! through [`rejects`].
// The file name `qa_TASK-M6-47` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::sync::Arc;

use eframe::egui::{self, Key, Modifiers};
use engine::contract::canvas::hatch::{hatch, hatch_stripe};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::view_ui::F3Fill;
use gui::app::{clear_colour, App};
use gui::capture::{self, render_frame, shoot, Step, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::headless::Headless;
use gui::layout::{Layout, Splitter};
use gui::mock::canvas::{MockCanvas, NO_INVALID};
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};

/// The capture's buffer is always [`SIZE`]; a smaller window is drawn in its top-left corner.
const STRIDE: usize = SIZE[0] as usize;

/// A window wider than tall, not the artboard; one taller than wide; and one with an odd margin, in pixels.
const WIDE: [u32; 2] = [1500, 900];
const TALL: [u32; 2] = [810, 1350];
const ODD: [u32; 2] = [1501, 900];

/// A rect `[x0, y0, x1, y1)` in pixels.
type Px = [usize; 4];

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

fn clear() -> [u8; 4] {
    clear_colour(&egui::Visuals::dark()).to_array()
}

fn px(rgba: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * STRIDE + x) * 4;
    rgba[i..i + 4].try_into().unwrap()
}

/// Frames enough for an edit to reach the snapshot and the canvas.
fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key) {
    for events in capture::key(key, Modifiers::NONE) {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

/// An app on the frozen mock in a window of `size`, drawing through `canvas`, settled, the fill as given.
fn app_at(canvas: &Arc<MockCanvas>, size: [u32; 2], fill: F3Fill) -> (App<MockSide>, Headless) {
    let mut app = App::new(
        MockSide::new(MockEngine::frozen(), Some(canvas.clone())),
        FORMAT,
    );
    let mut h = Headless::new(size, PIXELS_PER_POINT);
    app.view.f3_fill = fill;
    settle(&mut h, &mut app);
    (app, h)
}

/// The capture in a window of `size` with the layer hidden by F3 in `fill`.
fn hidden(canvas: &Arc<MockCanvas>, size: [u32; 2], fill: F3Fill) -> Vec<u8> {
    let mut app = App::new(
        MockSide::new(MockEngine::frozen(), Some(canvas.clone())),
        FORMAT,
    );
    let mut h = Headless::new(size, PIXELS_PER_POINT);
    app.view.f3_fill = fill;
    // Hidden from the first frame: a narrow window's shown layer is under its least size.
    app.shown = false;
    settle(&mut h, &mut app);
    render_frame(canvas, &mut app, &mut h).rgba
}

/// The whole-pixel bounding box, in a window of `size`, of the pixels that are not the clear colour.
fn drawn_box(rgba: &[u8], size: [u32; 2]) -> Px {
    let clear = clear();
    let mut b = [usize::MAX, usize::MAX, 0, 0];
    for y in 0..size[1] as usize {
        for x in 0..size[0] as usize {
            if px(rgba, x, y) != clear {
                b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
            }
        }
    }
    b
}

/// REQ-GUI-229's geometry: everything drawn is one square, its side the window's shorter side within one pixel,
/// centred within one pixel. Being the bounding box of what is not the clear colour, it has every other pixel clear.
fn check_square(rgba: &[u8], size: [u32; 2]) -> Px {
    let b = drawn_box(rgba, size);
    let (w, h) = (b[2] - b[0], b[3] - b[1]);
    let short = size[0].min(size[1]) as usize;
    assert!(w.abs_diff(short) <= 1, "width {w} in {size:?}: {b:?}");
    assert!(h.abs_diff(short) <= 1, "height {h} in {size:?}: {b:?}");
    assert!(
        b[0].abs_diff(size[0] as usize - b[2]) <= 1,
        "not centred across in {size:?}: {b:?}"
    );
    assert!(
        b[1].abs_diff(size[1] as usize - b[3]) <= 1,
        "not centred down in {size:?}: {b:?}"
    );
    b
}

/// The window pixel of `(s, t)` in a rect `[x0, y0, x1, y1]`, `t` Y-up.
fn at(rect: [f64; 4], s: f64, t: f64) -> (usize, usize) {
    let (s, t) = (s.clamp(0.0, 0.999), t.clamp(0.001, 1.0));
    (
        (rect[0] + s * (rect[2] - rect[0])).floor() as usize,
        (rect[3] - t * (rect[3] - rect[1])).floor() as usize,
    )
}

/// The mean difference, on a grid inside the square `q` of `fit`, between the pixel at `(s, t)` read through `read` and
/// the Extend capture's pixel at `(s, t)` in the field's rect `field`.
fn disagreement(
    fit: &[u8],
    extend: &[u8],
    q: Px,
    field: [f64; 4],
    read: impl Fn(f64, f64) -> (f64, f64),
) -> f64 {
    const N: usize = 48;
    let square = q.map(|v| v as f64);
    let mut sum = 0.0;
    for i in 0..N {
        for j in 0..N {
            let (s, t) = ((i as f64 + 0.5) / N as f64, (j as f64 + 0.5) / N as f64);
            let (a, b) = read(s, t);
            let (fx, fy) = at(square, a, b);
            let (ex, ey) = at(field, s, t);
            let (p, e) = (px(fit, fx, fy), px(extend, ex, ey));
            sum += (0..3).map(|c| f64::from(p[c].abs_diff(e[c]))).sum::<f64>();
        }
    }
    sum / (N * N) as f64
}

/// The square `q` of `fit` holds the view's `[0, 1]²` at one scale: its `(s, t)` agrees with the Extend capture's better
/// than any wrong reading of it, or the unscaled field, does. `aspect` is the window's longer side over its shorter, for the stretched reading:
/// a figure stretched to `W × H` has, in the centred square, `(s, t)` further from the middle by `aspect` along that side.
fn check_holds_the_view(fit: &[u8], extend: &[u8], q: Px, field: [f64; 4], size: [u32; 2]) {
    let aspect = f64::from(size[0].max(size[1])) / f64::from(size[0].min(size[1]));
    let across = size[0] > size[1];
    let right = disagreement(fit, extend, q, field, |s, t| (s, t));
    let wrong: [(&str, f64); 5] = [
        (
            "mirrored in s",
            disagreement(fit, extend, q, field, |s, t| (1.0 - s, t)),
        ),
        (
            "mirrored in t",
            disagreement(fit, extend, q, field, |s, t| (s, 1.0 - t)),
        ),
        (
            "the field's middle half only",
            disagreement(fit, extend, q, field, |s, t| (2.0 * s - 0.5, 2.0 * t - 0.5)),
        ),
        (
            "a larger span than the view's",
            disagreement(fit, extend, q, field, |s, t| {
                (0.25 + s / 2.0, 0.25 + t / 2.0)
            }),
        ),
        (
            "stretched to the window",
            disagreement(fit, extend, q, field, |s, t| {
                if across {
                    (0.5 + (s - 0.5) * aspect, t)
                } else {
                    (s, 0.5 + (t - 0.5) * aspect)
                }
            }),
        ),
    ];
    // And better than the window's own pixels at the shown scale, Extend's capture read in the same square: a smooth
    // field agrees with itself nearby, so the readings above alone would pass that picture.
    let unscaled = disagreement(extend, extend, q, field, |s, t| (s, t));
    assert!(
        right < unscaled,
        "the shown scale: {unscaled} against the view's {right}"
    );
    for (name, d) in wrong {
        assert!(right < d, "{name}: {d} against the view's {right}");
    }
}

/// The field's rect in pixels in a window of `size`, the shown layout's (R-468 E2's `w`).
fn field_px(size: [u32; 2]) -> [f64; 4] {
    let screen = Headless::new(size, PIXELS_PER_POINT).screen();
    let r = Layout::new(screen, PIXELS_PER_POINT).field;
    [r.min.x, r.min.y, r.max.x, r.max.y].map(|v| f64::from(v * PIXELS_PER_POINT))
}

/// The keys from launch to the row: Tab to the top bar, Enter, → to View, Enter (on F3 hide), ↓ to the row.
const TO_ROW: [Key; 5] = [
    Key::Tab,
    Key::Enter,
    Key::ArrowRight,
    Key::Enter,
    Key::ArrowDown,
];

/// [`TO_ROW`] as a capture's steps, then Enter on to Extend, → to Fit, Enter.
const CHOOSE_FIT: [Step; 8] = [
    Step::Tab,
    Step::Enter,
    Step::ArrowRight,
    Step::Enter,
    Step::ArrowDown,
    Step::Enter,
    Step::ArrowRight,
    Step::Enter,
];

fn fit_then(more: &[Step]) -> Vec<Step> {
    CHOOSE_FIT.iter().chain(more).copied().collect()
}

// ── The row ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// The accessible nodes of a frame: name, rect in points, role and whether toggled.
fn nodes(
    h: &mut Headless,
    app: &mut App<MockSide>,
) -> Vec<(
    String,
    egui::accesskit::Role,
    Option<egui::accesskit::Toggled>,
)> {
    let output = h.frame(app, Vec::new());
    let update = output.platform_output.accesskit_update.expect("a tree");
    update
        .nodes
        .iter()
        .filter_map(|(_, n)| n.label().map(|l| (l.to_string(), n.role(), n.toggled())))
        .collect()
}

/// A node's rect in pixels, by its accessible name.
fn rect_of(h: &mut Headless, app: &mut App<MockSide>, name: &str) -> [f64; 4] {
    let out = h.frame(app, Vec::new());
    h.names(&out)
        .into_iter()
        .find(|n| n.name == name)
        .and_then(|n| n.rect)
        .unwrap_or_else(|| panic!("no node named `{name}` with a rect"))
}

fn click(h: &mut Headless, app: &mut App<MockSide>, name: &str) {
    let r = rect_of(h, app, name);
    let ppp = f64::from(PIXELS_PER_POINT);
    let centre = egui::pos2(
        ((r[0] + r[2]) / 2.0 / ppp) as f32,
        ((r[1] + r[3]) / 2.0 / ppp) as f32,
    );
    for events in capture::click(centre) {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

/// View's rows in order, as drawn, the options on one row (R-468 E4).
fn check_order(hide: [f64; 4], extend: [f64; 4], fit: [f64; 4], debug: [f64; 4]) {
    assert!(extend[1] >= hide[3] - 1.0, "the row after F3 hide");
    assert!(debug[1] >= extend[3] - 1.0, "the row before Debug view");
    let middle = |r: [f64; 4]| (r[1] + r[3]) / 2.0;
    assert!(
        (middle(extend) - middle(fit)).abs() < (extend[3] - extend[1]),
        "one row"
    );
    assert!(fit[0] >= extend[2] - 1.0, "Extend, then Fit");
}

#[test]
fn qa_m6_47_row_is_after_f3_hide_two_radios_extend_marked() {
    let canvas = canvas();
    let (mut app, mut h) = app_at(&canvas, SIZE, F3Fill::default());
    assert_eq!(app.view.f3_fill, F3Fill::Extend, "Extend at launch");
    click(&mut h, &mut app, "View");
    let (hide, extend, fit, debug) = (
        rect_of(&mut h, &mut app, "F3 hide"),
        rect_of(&mut h, &mut app, "Extend"),
        rect_of(&mut h, &mut app, "Fit"),
        rect_of(&mut h, &mut app, "Debug view"),
    );
    check_order(hide, extend, fit, debug);
    rejects("the row and Debug view swapped", || {
        check_order(hide, debug, fit, extend)
    });
    let marked = |list: &[(
        String,
        egui::accesskit::Role,
        Option<egui::accesskit::Toggled>,
    )],
                  name: &str| {
        let n = list.iter().find(|n| n.0 == name).expect(name);
        assert_eq!(n.1, egui::accesskit::Role::RadioButton, "{name} is a radio");
        n.2 == Some(egui::accesskit::Toggled::True)
    };
    let list = nodes(&mut h, &mut app);
    assert!(marked(&list, "Extend") && !marked(&list, "Fit"));
    // Clicked, the mark moves, the layer stays shown and the view does not change: it acts at the next F3.
    let placed = canvas.placement();
    let depth = app.snapshot().history.undo_depth;
    click(&mut h, &mut app, "Fit");
    assert_eq!(app.view.f3_fill, F3Fill::Fit);
    assert!(app.shown);
    assert_eq!(canvas.placement(), placed, "the row acts at the next F3");
    assert_eq!(app.snapshot().history.undo_depth, depth, "an undo entry");
    press(&mut h, &mut app, Key::F3);
    assert_ne!(canvas.placement(), placed, "F3 acts on it");
}

#[test]
fn qa_m6_47_row_keyboard_enter_chooses_moving_does_not() {
    let canvas = canvas();
    let (mut app, mut h) = app_at(&canvas, SIZE, F3Fill::default());
    let (sim, depth, lines) = (
        app.snapshot().sim.clone(),
        app.snapshot().history.undo_depth,
        app.console().len(),
    );
    for key in TO_ROW {
        press(&mut h, &mut app, key);
    }
    press(&mut h, &mut app, Key::Enter);
    assert_eq!(app.view.f3_fill, F3Fill::Extend, "Enter on the row itself");
    press(&mut h, &mut app, Key::ArrowRight);
    assert_eq!(app.view.f3_fill, F3Fill::Extend, "moving to Fit");
    press(&mut h, &mut app, Key::Enter);
    assert_eq!(app.view.f3_fill, F3Fill::Fit, "Enter on Fit");
    press(&mut h, &mut app, Key::ArrowLeft);
    assert_eq!(app.view.f3_fill, F3Fill::Fit, "moving back to Extend");
    press(&mut h, &mut app, Key::Enter);
    assert_eq!(app.view.f3_fill, F3Fill::Extend, "Enter on Extend");
    assert_eq!(app.snapshot().sim, sim, "the view changed");
    assert_eq!(app.snapshot().history.undo_depth, depth, "an undo entry");
    let set_fields = app.console()[lines..]
        .iter()
        .filter(|e| e.message.starts_with("SetField"))
        .count();
    assert_eq!(set_fields, 0, "a SetField");
    // Control: without the ↓, Enter and → act on F3 hide and on View's neighbours, and Fit is not chosen.
    let (mut other, mut h) = app_at(&canvas, SIZE, F3Fill::default());
    for key in &TO_ROW[..4] {
        press(&mut h, &mut other, *key);
    }
    for key in [Key::ArrowRight, Key::Enter] {
        press(&mut h, &mut other, key);
    }
    rejects("keys that never reach the row", || {
        assert_eq!(other.view.f3_fill, F3Fill::Fit)
    });
}

// ── Fit's pixels ────────────────────────────────────────────────────────────────────────────────────────────────────

/// The acceptance's own capture, the one `mock_f3_fit` records: from launch, by the keyboard, then F3.
#[test]
fn qa_m6_47_fit_capture_case_from_the_keyboard() {
    let canvas = canvas();
    let fit = shoot(canvas.clone(), &fit_then(&[Step::F3]));
    let b = check_square(&fit.rgba, SIZE);
    assert_eq!(b, [405, 0, 1755, 1350], "1350 of 2160, centred");
    let extend = shoot(canvas.clone(), &[Step::F3]);
    check_holds_the_view(&fit.rgba, &extend.rgba, b, field_px(SIZE), SIZE);
    // Controls: Extend's capture is no Fit, on either check, and F3 without the choice is Extend.
    rejects("Extend's capture as a square", || {
        check_square(&extend.rgba, SIZE);
    });
    rejects("Extend's capture as holding the view", || {
        check_holds_the_view(&extend.rgba, &extend.rgba, b, field_px(SIZE), SIZE)
    });
}

#[test]
fn qa_m6_47_fit_wider_taller_and_an_odd_margin() {
    let canvas = canvas();
    let cases: [([u32; 2], Px); 3] = [
        (WIDE, [300, 0, 1200, 900]),
        (TALL, [0, 270, 810, 1080]),
        // 601 pixels of margin: 300 and 301, either way round.
        (ODD, [0, 0, 0, 0]),
    ];
    for (size, expected) in cases {
        let fit = hidden(&canvas, size, F3Fill::Fit);
        let b = check_square(&fit, size);
        if expected != [0; 4] {
            assert_eq!(b, expected, "{size:?}");
        }
        let extend = hidden(&canvas, size, F3Fill::Extend);
        check_holds_the_view(&fit, &extend, b, field_px(size), size);
        rejects("Extend's window taken as Fit's", || {
            check_square(&extend, size);
        });
        rejects("a square that spans the longer side", || {
            let long = size[0].max(size[1]) as usize;
            let (w, h) = (b[2] - b[0], b[3] - b[1]);
            assert!(w.abs_diff(long) <= 1 && h.abs_diff(long) <= 1);
        });
    }
}

/// R-468 E2: everything outside the square, past `[0, 1]²` included, is the canvas background, whatever the hook flags
/// there and in the debug view too.
#[test]
fn qa_m6_47_fit_shows_nothing_past_the_square_in_the_debug_view() {
    let canvas = canvas();
    let (mut app, mut h) = app_at(&canvas, SIZE, F3Fill::Fit);
    canvas.flag_invalid([-0.5, -0.5, 0.0, 1.5]);
    app.set_field(SetField {
        edit: Edit::Render(engine::contract::set_field::RenderField::DebugView(true)),
        no_history: false,
    });
    settle(&mut h, &mut app);
    press(&mut h, &mut app, Key::F3);
    let debug = render_frame(&canvas, &mut app, &mut h).rgba;
    canvas.flag_invalid(NO_INVALID);
    let b = check_square(&debug, SIZE);
    assert_eq!(b, [405, 0, 1755, 1350], "a hatch past the square");
    // Control: Extend, the same hook and view: the hatch is there, so the square is not.
    let (mut other, mut h) = app_at(&canvas, SIZE, F3Fill::Extend);
    canvas.flag_invalid([-0.5, -0.5, 0.0, 1.5]);
    other.set_field(SetField {
        edit: Edit::Render(engine::contract::set_field::RenderField::DebugView(true)),
        no_history: false,
    });
    settle(&mut h, &mut other);
    press(&mut h, &mut other, Key::F3);
    let extend = render_frame(&canvas, &mut other, &mut h).rgba;
    canvas.flag_invalid(NO_INVALID);
    rejects("Extend's debug capture", || {
        check_square(&extend, SIZE);
    });
}

/// A hook's region inside `[0, 1]²` lands in the square where `(s, t)` says: the background in the presentation view
/// and the hatch in the debug view (R-467, read with R-468 E2).
#[test]
fn qa_m6_47_fit_region_inside_the_square_lands_at_its_s_t() {
    let canvas = canvas();
    let region = [0.25_f32, 0.5, 0.5, 0.75];
    let square = [405.0, 0.0, 1755.0, 1350.0];
    let shot = |debug: bool| {
        let (mut app, mut h) = app_at(&canvas, SIZE, F3Fill::Fit);
        canvas.flag_invalid(region);
        if debug {
            app.set_field(SetField {
                edit: Edit::Render(engine::contract::set_field::RenderField::DebugView(true)),
                no_history: false,
            });
            settle(&mut h, &mut app);
        }
        press(&mut h, &mut app, Key::F3);
        let rgba = render_frame(&canvas, &mut app, &mut h).rgba;
        canvas.flag_invalid(NO_INVALID);
        rgba
    };
    let (presentation, debug) = (shot(false), shot(true));
    let clear = clear();
    let is_hatch = |rgba: &[u8], x: usize, y: usize| {
        let [r, g, b] = hatch().colours[hatch_stripe([x as f64 + 0.5, y as f64 + 0.5])];
        px(rgba, x, y) == [r, g, b, 255]
    };
    let inside = at(square, 0.375, 0.625);
    assert_eq!(px(&presentation, inside.0, inside.1), clear, "the region");
    assert!(is_hatch(&debug, inside.0, inside.1), "the region, debug");
    // Below it in t and right of it in s, the field: so the region is at `(s, t)`, `t` Y-up, not mirrored.
    for (s, t) in [(0.375, 0.375), (0.625, 0.625)] {
        let (x, y) = at(square, s, t);
        assert_ne!(px(&presentation, x, y), clear, "({s}, {t})");
        assert!(!is_hatch(&debug, x, y), "({s}, {t}), debug");
    }
    rejects("the region's pixel read as outside it", || {
        assert_ne!(px(&presentation, inside.0, inside.1), clear)
    });
}

// ── The quads ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// The deepest level the quads reach, as the narrowest quad's width: the smaller the deeper.
fn narrowest(quads: &[gui::mock::quads::Quad]) -> f64 {
    quads
        .iter()
        .map(|q| q[2] - q[0])
        .fold(f64::INFINITY, f64::min)
}

/// A window taller than wide, large enough for the shown layer's panels (no pixels are read from it).
const TALL_SHOWN: [u32; 2] = [1620, 2700];

/// `k ≥ 2`: a window taller than wide gives it, its shorter side over the shown figure's width.
fn tall_k() -> f64 {
    let f = field_px(TALL_SHOWN);
    f64::from(TALL_SHOWN[0]) / (f[2] - f[0])
}

#[test]
fn qa_m6_47_quads_finer_in_fit_at_k_2_kept_and_extend_unchanged() {
    assert!(tall_k() >= 2.0, "the window's k: {}", tall_k());
    let canvas = canvas();
    let (mut fit, mut h) = app_at(&canvas, TALL_SHOWN, F3Fill::Fit);
    let shown: Vec<_> = fit.side().engine().integrated().to_vec();
    assert!(!shown.is_empty());
    press(&mut h, &mut fit, Key::F3);
    let in_fit: Vec<_> = fit.side().engine().integrated().to_vec();
    // Finer: the deepest level is greater, about log₂ k, so at least one level at `k ≥ 2`.
    assert!(
        narrowest(&in_fit) <= narrowest(&shown) / 2.0,
        "Fit's narrowest {} against the shown {}",
        narrowest(&in_fit),
        narrowest(&shown)
    );
    // Those already computed are kept (R-468 E3).
    for q in &shown {
        assert!(in_fit.contains(q), "a computed quad dropped: {q:?}");
    }
    // Extend, in the same window: more quads at the shown size, none finer.
    let (mut extend, mut h) = app_at(&canvas, TALL_SHOWN, F3Fill::Extend);
    press(&mut h, &mut extend, Key::F3);
    let in_extend: Vec<_> = extend.side().engine().integrated().to_vec();
    assert_eq!(
        narrowest(&in_extend),
        narrowest(&shown),
        "Extend refined the quads"
    );
    for q in &shown {
        assert!(in_extend.contains(q), "Extend dropped {q:?}");
    }
    assert!(narrowest(&in_fit) < narrowest(&in_extend));
    // Controls: quads left at the shown scale, and a rule that coarsens, are not finer.
    rejects("quads left at the shown scale", || {
        assert!(narrowest(&shown) <= narrowest(&shown) / 2.0)
    });
    let plane = extend.side().engine().plane().clone();
    let coarser = gui::mock::quads::quads_at(&plane, [0.0, 0.0, 1.0, 1.0], 1.0 / tall_k());
    rejects("a split rule dividing by k", || {
        assert!(narrowest(&coarser) <= narrowest(&shown) / 2.0)
    });
}

/// The tile-bounds capture, shown and in Extend, is the same before and after a trip through Fit, whose finer bounds
/// must not stay; and Fit's bounds are finer than the shown layout's, seen on the pixels.
#[test]
fn qa_m6_47_quads_tile_bounds_unchanged_after_fit() {
    let canvas = canvas();
    let shown = shoot(canvas.clone(), &[Step::Alt1]);
    let back = {
        let (mut app, mut h) = app_at(&canvas, SIZE, F3Fill::default());
        for events in capture::key(Key::Num1, Modifiers::ALT) {
            let _ = h.frame(&mut app, events);
        }
        settle(&mut h, &mut app);
        let before = render_frame(&canvas, &mut app, &mut h).rgba;
        assert!(before == shown.rgba, "the tile bounds, shown");
        app.view.f3_fill = F3Fill::Fit;
        press(&mut h, &mut app, Key::F3);
        let fit = render_frame(&canvas, &mut app, &mut h).rgba;
        assert!(fit != before, "Fit drew the shown capture");
        press(&mut h, &mut app, Key::F3);
        render_frame(&canvas, &mut app, &mut h).rgba
    };
    assert!(
        back == shown.rgba,
        "Fit's bounds stayed in the shown layout"
    );
    // Control: the same comparison on Extend's capture, which holds more bounds around the rect.
    let off = shoot(canvas.clone(), &[Step::Alt1, Step::F3]);
    rejects("the shown capture against Extend's", || {
        assert!(back == off.rgba)
    });
}

// ── F3 twice ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// What F3 twice must return: the layout's rects, the view's bits, the undo depth, the applied edits and the capture.
#[derive(Clone, PartialEq, Debug)]
struct Everything {
    rects: [egui::Rect; 4],
    view: Vec<u64>,
    depth: u32,
    edits: u64,
    rgba: Vec<u8>,
}

fn everything(canvas: &MockCanvas, app: &mut App<MockSide>, h: &mut Headless) -> Everything {
    let l = Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits);
    let p = &app.snapshot().sim.plane;
    let view = [&p.z0, &p.q1, &p.q2]
        .into_iter()
        .flatten()
        .map(|v| v.to_bits())
        .collect();
    let depth = app.snapshot().history.undo_depth;
    let edits = app.side().engine().edits_applied();
    let rgba = render_frame(canvas, app, h).rgba;
    Everything {
        rects: [l.figure, l.manifold_view, l.trajectory, l.footer],
        view,
        depth,
        edits,
        rgba,
    }
}

/// F3 twice from a view moved off the launch view and a splitter moved off its default, in either mode.
#[test]
fn qa_m6_47_f3_twice_returns_everything_from_a_moved_view() {
    let canvas = canvas();
    for fill in [F3Fill::Extend, F3Fill::Fit] {
        let (mut app, mut h) = app_at(&canvas, SIZE, fill);
        let mut z0 = app.snapshot().sim.plane.z0;
        z0[0] += 0.125;
        app.set_field(SetField {
            edit: Edit::Sim(SimField::Z0(z0)),
            no_history: false,
        });
        let at = app.splits.get(Splitter::ManifoldFigure) + 0.03;
        app.splits.set(Splitter::ManifoldFigure, at);
        settle(&mut h, &mut app);
        let before = everything(&canvas, &mut app, &mut h);
        press(&mut h, &mut app, Key::F3);
        let hidden = everything(&canvas, &mut app, &mut h);
        assert!(hidden.rgba != before.rgba, "{fill:?}: F3 hid nothing");
        assert_eq!(hidden.view, before.view, "{fill:?}: F3 moved the view");
        assert_eq!(hidden.depth, before.depth, "{fill:?}: an undo entry");
        assert_eq!(hidden.edits, before.edits, "{fill:?}: a SetField");
        if fill == F3Fill::Fit {
            check_square(&hidden.rgba, SIZE);
        }
        press(&mut h, &mut app, Key::F3);
        let back = everything(&canvas, &mut app, &mut h);
        assert!(back == before, "{fill:?}: F3 twice changed something");
        assert_eq!(app.view.f3_fill, fill, "the mode kept through F3");
        // Control: a Fit that edits the zoom, a SetField, fails on the same comparison.
        let mut z0 = app.snapshot().sim.plane.z0;
        z0[1] += 0.0625;
        app.set_field(SetField {
            edit: Edit::Sim(SimField::Z0(z0)),
            no_history: false,
        });
        settle(&mut h, &mut app);
        let edited = everything(&canvas, &mut app, &mut h);
        rejects("an edit between the two presses", || {
            assert!(edited == before)
        });
    }
}

/// Fit's square does not follow the splitters: the view's square, scaled to the window's shorter side, whatever the
/// shown figure's rect was dragged to.
#[test]
fn qa_m6_47_fit_same_with_a_splitter_moved() {
    let canvas = canvas();
    let plain = hidden(&canvas, SIZE, F3Fill::Fit);
    let (mut app, mut h) = app_at(&canvas, SIZE, F3Fill::Fit);
    let at = app.splits.get(Splitter::ManifoldFigure) + 0.05;
    app.splits.set(Splitter::ManifoldFigure, at);
    settle(&mut h, &mut app);
    press(&mut h, &mut app, Key::F3);
    let moved = render_frame(&canvas, &mut app, &mut h).rgba;
    assert!(moved == plain, "Fit read the splitters");
    let extend = hidden(&canvas, SIZE, F3Fill::Extend);
    rejects("Extend against Fit", || assert!(extend == plain));
}

// ── Present mode and the session ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn qa_m6_47_present_follows_the_mode() {
    let canvas = canvas();
    let presented = |fill| {
        let (mut app, mut h) = app_at(&canvas, SIZE, fill);
        app.present = true;
        settle(&mut h, &mut app);
        render_frame(&canvas, &mut app, &mut h).rgba
    };
    let (present_fit, present_extend) = (presented(F3Fill::Fit), presented(F3Fill::Extend));
    let (f3_fit, f3_extend) = (
        hidden(&canvas, SIZE, F3Fill::Fit),
        hidden(&canvas, SIZE, F3Fill::Extend),
    );
    assert!(
        present_fit == f3_fit,
        "present mode in Fit against F3 in Fit"
    );
    assert!(present_extend == f3_extend, "and in Extend");
    check_square(&present_fit, SIZE);
    rejects("present mode ignoring the mode", || {
        assert!(present_fit == f3_extend)
    });
}

/// Extend at each launch: a new app is Extend after a session that chose Fit, and the settings the first would save
/// hold no fill (E4: "not kept across sessions"); within the session the choice stays.
#[test]
fn qa_m6_47_extend_at_each_launch_kept_for_the_session() {
    let canvas = canvas();
    let (mut first, mut h) = app_at(&canvas, SIZE, F3Fill::default());
    first.view.f3_fill = F3Fill::Fit;
    press(&mut h, &mut first, Key::F3);
    press(&mut h, &mut first, Key::F3);
    assert_eq!(first.view.f3_fill, F3Fill::Fit, "kept for the session");
    let saved = gui::settings::Settings::default().to_json().to_lowercase();
    assert!(!saved.contains("fill") && !saved.contains("fit"), "{saved}");
    let relaunched = App::new(
        MockSide::new(MockEngine::frozen(), Some(canvas.clone())),
        FORMAT,
    );
    assert_eq!(relaunched.view.f3_fill, F3Fill::Extend, "Extend at launch");
    rejects("a mode that persists across a relaunch", || {
        assert_eq!(first.view.f3_fill, F3Fill::Extend)
    });
}
