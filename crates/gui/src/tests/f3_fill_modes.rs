//! `f3_fill_modes` (REQ-GUI-229; R-468; render_gui_spec §G1, §G2): View's row F3 fills by: Extend / Fit, reached from
//! the keyboard, Extend at each launch and `ViewUI`'s, no `SetField`. Extend is R-406's fill. Fit scales the view's
//! square to the window's shorter side, centred, never stretched, the rest of the window the clear colour, the fake
//! quads split at the new screen scale, finer, the shown layout's and Extend's unchanged. F3 twice, in either mode,
//! returns the layout, the view and the capture. Present mode follows the mode. Each check also runs on an input it
//! must reject (R-176, through `rejects`; gui takes no dependency on validation, R-187). The captures run on the GPU, on
//! `PRIN_GPU_BACKEND`'s backend (R-206).

use std::sync::Arc;

use eframe::egui::{self, Key, Modifiers};
use eframe::egui_wgpu;
use engine::contract::view_ui::F3Fill;

use super::f3_fallback_mock::{check_background, check_hatched, debug_then};
use super::f3_fills_window_mock::{
    canvas, check_covers, check_same_over, clear, layout, pixel, shown_px, stand_in, tile_bounds_on,
};
use super::support::{click, focus, headless, mock_app, press, rejects};
use crate::app::{clear_colour, App, FigureCallback};
use crate::capture::{self, shoot, Step, FORMAT, PIXELS_PER_POINT, SIZE};
use crate::explore::top_bar::{self, DEBUG_VIEW, FILL_EXTEND, FILL_FIT, FILL_ROW, HIDE};
use crate::headless::Headless;
use crate::layout::{Layout, Placement};
use crate::mock::canvas::{FigureCanvas, MockCanvas, NO_INVALID};
use crate::mock::quads::{self, UNIT};
use crate::mock::MockEngine;
use crate::side::{EngineSide, FigureView, MockSide};

/// The steps that choose Fit from the keyboard: Tab to the top bar, Enter, → to View, Enter on to F3 hide, ↓ to the
/// row, Enter on to Extend, → to Fit, Enter.
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

/// [`CHOOSE_FIT`], then F3.
fn fit_then_f3() -> Vec<Step> {
    CHOOSE_FIT.iter().chain(&[Step::F3]).copied().collect()
}

/// The keys of [`CHOOSE_FIT`] up to the row.
const TO_ROW: [Key; 5] = [
    Key::Tab,
    Key::Enter,
    Key::ArrowRight,
    Key::Enter,
    Key::ArrowDown,
];

fn frames(h: &mut Headless, app: &mut App<MockSide>, n: usize) {
    for _ in 0..n {
        let _ = h.frame(app, Vec::new());
    }
}

/// A rect `[x0, y0, x1, y1)` in pixels.
type Px = [usize; 4];

/// The whole-pixel bounding box of the pixels of `rgba` (`size` wide and high) that are not the clear colour.
fn drawn_box(rgba: &[u8], size: [usize; 2]) -> Px {
    let clear = clear();
    let mut b = [usize::MAX, usize::MAX, 0, 0];
    for y in 0..size[1] {
        for x in 0..size[0] {
            let i = (y * size[0] + x) * 4;
            if rgba[i..i + 4] != clear {
                b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
            }
        }
    }
    b
}

/// Fit's pixels in a window of `size`: everything drawn is one square, its side the window's shorter side within a
/// pixel, centred within a pixel. Returns it.
fn check_fit(rgba: &[u8], size: [usize; 2]) -> Px {
    let b = drawn_box(rgba, size);
    let (w, h) = (b[2] - b[0], b[3] - b[1]);
    let short = size[0].min(size[1]);
    assert!(w.abs_diff(short) <= 1, "width {w} of {size:?}: {b:?}");
    assert!(h.abs_diff(short) <= 1, "height {h} of {size:?}: {b:?}");
    assert!(w.abs_diff(h) <= 1, "not square: {b:?}");
    assert!(
        b[0].abs_diff(size[0] - b[2]) <= 1,
        "not centred across: {b:?}"
    );
    assert!(
        b[1].abs_diff(size[1] - b[3]) <= 1,
        "not centred down: {b:?}"
    );
    b
}

/// A mock app on a headless window of `size`, frozen, with the fill `fill` and the layer hidden by F3 or in present
/// mode, rendered at `size`.
fn hidden_capture(
    canvas: &Arc<MockCanvas>,
    size: [u32; 2],
    fill: F3Fill,
    present: bool,
) -> Vec<u8> {
    let side = MockSide::new(MockEngine::frozen(), Some(canvas.clone()));
    let mut app = App::new(side, FORMAT);
    let mut h = Headless::new(size, PIXELS_PER_POINT);
    app.view.f3_fill = fill;
    // Hidden from the first frame: the layer shown has its own least window, and this one may be under it.
    if present {
        app.present = true;
    } else {
        app.shown = false;
    }
    frames(&mut h, &mut app, 2);
    let output = h.frame(&mut app, Vec::new());
    let primitives = h.ctx.tessellate(output.shapes, PIXELS_PER_POINT);
    let colour = clear_colour(&h.ctx.global_style().visuals);
    capture::render_at(&**canvas, &h.textures, &primitives, colour, size)
}

/// The stand-in of `mock` at `placement`, drawn by the figure canvas over the capture's window, clipped to `clip`, with
/// the marks over `span`.
fn stand_in_clipped(
    canvas: &Arc<MockCanvas>,
    mock: &MockEngine,
    placement: Placement,
    clip: egui::Rect,
    span: [f64; 4],
) -> Vec<u8> {
    canvas.set_chart(mock.plane());
    canvas.set_placement(placement, mock.marks_over(span));
    let window = layout().window;
    let callback = egui_wgpu::Callback::new_paint_callback(
        window,
        FigureCallback {
            canvas: Arc::new(FigureCanvas(canvas.clone())),
            format: FORMAT,
        },
    );
    let primitive = egui::ClippedPrimitive {
        clip_rect: clip,
        primitive: egui::epaint::Primitive::Callback(callback),
    };
    let colour = clear_colour(&egui::Visuals::dark());
    capture::render(&**canvas, &Default::default(), &[primitive], colour)
}

/// The deepest level of `quads`, read from the narrowest quad's width, the root being `2 ROOT` wide.
fn deepest(quads: &[quads::Quad]) -> u32 {
    let narrowest = quads
        .iter()
        .map(|q| q[2] - q[0])
        .fold(f64::INFINITY, f64::min);
    (2.0 * quads::ROOT / narrowest).log2().round() as u32
}

/// The bits of the view `(z₀, q₁, q₂)`.
fn view_bits(app: &App<MockSide>) -> Vec<u64> {
    let p = &app.snapshot().sim.plane;
    [&p.z0, &p.q1, &p.q2]
        .into_iter()
        .flatten()
        .map(|v| v.to_bits())
        .collect()
}

/// The undo depth and the edits the mock has applied.
fn edits(app: &mut App<MockSide>) -> (u32, u64) {
    let depth = app.snapshot().history.undo_depth;
    (depth, app.side().engine().edits_applied())
}

#[test]
fn f3_fill_modes_row_is_in_the_view_menu_and_marked() {
    let mut tree = crate::keyboard::scopes::ScopeTree::new();
    top_bar::register(&mut tree);
    assert_eq!(
        tree.children(Some("view")),
        [HIDE.0, FILL_ROW.0, DEBUG_VIEW.0, top_bar::RESET_LAYOUT.0]
    );
    assert_eq!(tree.children(Some(FILL_ROW.0)), [FILL_EXTEND.0, FILL_FIT.0]);
    let mut app = mock_app();
    let mut h = headless();
    frames(&mut h, &mut app, 2);
    click(&mut h, &mut app, "View");
    let rect = |app: &App<MockSide>, id| app.keyboard.place_of(id).map(|p| p.rect).unwrap();
    let (hide, row, debug) = (
        rect(&app, HIDE.0),
        rect(&app, FILL_ROW.0),
        rect(&app, DEBUG_VIEW.0),
    );
    assert!(row.min.y >= hide.max.y - 0.5, "{row:?} after {hide:?}");
    assert!(debug.min.y >= row.max.y - 0.5, "{debug:?} after {row:?}");
    let (extend, fit) = (rect(&app, FILL_EXTEND.0), rect(&app, FILL_FIT.0));
    assert!(row.contains_rect(extend.shrink(1.0)) && row.contains_rect(fit.shrink(1.0)));
    assert!(fit.min.x >= extend.max.x - 0.5, "one row, Extend then Fit");
    // The two options are radio buttons, the chosen one marked.
    let marked = |h: &mut Headless, app: &mut App<MockSide>, name: &str| {
        let output = h.frame(app, Vec::new());
        let update = output.platform_output.accesskit_update.expect("a tree");
        let (_, node) = update
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some(name))
            .expect(name);
        assert_eq!(node.role(), egui::accesskit::Role::RadioButton, "{name}");
        node.toggled() == Some(egui::accesskit::Toggled::True)
    };
    assert!(marked(&mut h, &mut app, FILL_EXTEND.1), "Extend at launch");
    assert!(!marked(&mut h, &mut app, FILL_FIT.1));
    app.view.f3_fill = F3Fill::Fit;
    frames(&mut h, &mut app, 1);
    assert!(!marked(&mut h, &mut app, FILL_EXTEND.1));
    assert!(marked(&mut h, &mut app, FILL_FIT.1), "Fit chosen");
    rejects("the row after Debug view", || {
        assert!(row.min.y >= debug.max.y - 0.5)
    });
}

#[test]
fn f3_fill_modes_keyboard_reaches_the_row_and_enter_chooses() {
    let mut app = mock_app();
    let mut h = headless();
    frames(&mut h, &mut app, 2);
    let (view, before) = (view_bits(&app), edits(&mut app));
    for key in TO_ROW {
        press(&mut h, &mut app, key, Modifiers::NONE);
    }
    assert_eq!(focus(&app), ["top_bar", "view", FILL_ROW.0]);
    assert_eq!(
        app.view.f3_fill,
        F3Fill::Extend,
        "the row itself chooses nothing"
    );
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    assert_eq!(focus(&app), ["top_bar", "view", FILL_ROW.0, FILL_EXTEND.0]);
    press(&mut h, &mut app, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(
        focus(&app),
        ["top_bar", "view", FILL_ROW.0, FILL_EXTEND.0],
        "a row: ↓ stays"
    );
    press(&mut h, &mut app, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(focus(&app), ["top_bar", "view", FILL_ROW.0, FILL_FIT.0]);
    assert_eq!(app.view.f3_fill, F3Fill::Extend, "moving chooses nothing");
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    assert_eq!(app.view.f3_fill, F3Fill::Fit, "Enter on Fit");
    press(&mut h, &mut app, Key::ArrowLeft, Modifiers::NONE);
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    assert_eq!(app.view.f3_fill, F3Fill::Extend, "Enter on Extend");
    // Never an edit, and the options' places exist while the menu is open on them.
    assert_eq!(edits(&mut app), before, "a SetField or an undo entry");
    assert_eq!(view_bits(&app), view);
    assert!(app.keyboard.place_of(FILL_FIT.0).is_some());
    // Control: the keys that stop on F3 hide do not reach the row.
    let mut other = mock_app();
    let mut h = headless();
    frames(&mut h, &mut other, 2);
    for key in &TO_ROW[..4] {
        press(&mut h, &mut other, *key, Modifiers::NONE);
    }
    rejects("keys that stop on F3 hide", || {
        assert_eq!(focus(&other), ["top_bar", "view", FILL_ROW.0])
    });
}

#[test]
fn f3_fill_modes_click_chooses_and_acts_at_the_next_f3() {
    let canvas = canvas();
    let side = MockSide::new(MockEngine::frozen(), Some(canvas.clone()));
    let mut app = App::new(side, FORMAT);
    let mut h = headless();
    frames(&mut h, &mut app, 2);
    let before = edits(&mut app);
    click(&mut h, &mut app, "View");
    click(&mut h, &mut app, FILL_FIT.1);
    assert_eq!(app.view.f3_fill, F3Fill::Fit);
    assert!(app.shown, "chosen with the layer shown; nothing hides yet");
    assert_eq!(
        canvas.placement(),
        layout().placement(),
        "acts at the next F3"
    );
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    frames(&mut h, &mut app, 1);
    let square = layout().fit_rect(PIXELS_PER_POINT);
    assert_eq!(canvas.placement(), Placement::of(square, layout().window));
    assert_eq!(edits(&mut app), before, "a SetField or an undo entry");
}

/// `top_bar::activate`'s two arms, and that nothing else chooses.
#[test]
fn f3_fill_modes_activate_arms() {
    let chosen = |id| {
        let mut actions = crate::app::Actions::default();
        top_bar::activate(id, &mut actions);
        actions.f3_fill
    };
    assert_eq!(chosen(FILL_EXTEND.0), Some(F3Fill::Extend));
    assert_eq!(chosen(FILL_FIT.0), Some(F3Fill::Fit));
    assert_eq!(chosen(FILL_ROW.0), None);
    assert_eq!(chosen(HIDE.0), None);
}

#[test]
fn f3_fill_modes_extend_is_today_s_fill() {
    let canvas = canvas();
    let on = shoot(canvas.clone(), &[]);
    let off = shoot(canvas.clone(), &[Step::F3]);
    check_same_over(&on.rgba, &off.rgba, shown_px(), "the shown rect, Extend");
    check_covers(&off.rgba, "Extend covers the window");
    let placed = stand_in(&canvas, &MockEngine::frozen(), layout().placement());
    assert_eq!(
        off.rgba, placed,
        "Extend is the stand-in at the shown placement"
    );
    // Fit, then back to Extend, from the keyboard: the same capture.
    let mut back: Vec<Step> = CHOOSE_FIT.to_vec();
    back.extend([Step::ArrowLeft, Step::Enter, Step::F3]);
    assert_eq!(
        shoot(canvas.clone(), &back).rgba,
        off.rgba,
        "Fit, then Extend"
    );
    // Control: Fit is not Extend over the shown rect.
    let fit = shoot(canvas.clone(), &fit_then_f3());
    rejects("Fit taken as Extend", || {
        check_same_over(&on.rgba, &fit.rgba, shown_px(), "the shown rect, Fit")
    });
}

/// Under Extend the hook's region is the background in the presentation view and the hatch in the debug view
/// (R-468 clause 4; R-467).
#[test]
fn f3_fill_modes_extend_hook_background_and_hatch() {
    let canvas = canvas();
    let region = [-0.4, 0.2, -0.1, 0.8];
    canvas.flag_invalid(region);
    let presentation = shoot(canvas.clone(), &[Step::F3]);
    let debug = shoot(canvas.clone(), &debug_then(&[Step::F3]));
    canvas.flag_invalid(NO_INVALID);
    assert!(check_background(&presentation.rgba, region) > 0);
    assert!(check_hatched(&debug.rgba, region) > 0);
    rejects("the hatch in the presentation view", || {
        check_hatched(&presentation.rgba, region);
    });
    rejects("the background in the debug view", || {
        check_background(&debug.rgba, region);
    });
}

#[test]
fn f3_fill_modes_fit_wider_than_tall() {
    let canvas = canvas();
    let size = SIZE.map(|v| v as usize);
    let fit = shoot(canvas.clone(), &fit_then_f3());
    let b = check_fit(&fit.rgba, size);
    assert_eq!(b, [405, 0, 1755, 1350], "1350 pixels, centred in 2160");
    // Every pixel of the square is the stand-in at Fit's placement, one scale on both axes, `(s, t)` 1 : 1.
    let l = layout();
    let square = l.fit_rect(PIXELS_PER_POINT);
    let p = Placement::of(square, l.window);
    assert!(
        (f64::from(p.half[0]) * 2160.0 - f64::from(p.half[1]) * 1350.0).abs() < 1e-3,
        "one scale: {p:?}"
    );
    let mut mock = MockEngine::frozen();
    mock.set_fit(Some(l.fit_scale()));
    let placed = stand_in_clipped(&canvas, &mock, p, square, UNIT);
    assert_eq!(fit.rgba, placed, "Fit is the stand-in at its placement");
    let edges = [0, 0, 2160, 1350];
    // Controls: stretched to W × H; the stretched one covers the window and fails the square, and differs inside it.
    let stretched = stand_in(&canvas, &MockEngine::frozen(), Placement::IDENTITY);
    rejects("a Fit stretched to W × H", || {
        check_fit(&stretched, size);
    });
    rejects("a Fit stretched to W × H, pixels", || {
        check_same_over(&fit.rgba, &stretched, b, "the square, stretched")
    });
    // A square scaled to the longer side, letterboxed against the wrong one: it overflows the window's height.
    let longer = egui::Rect::from_center_size(square.center(), egui::Vec2::splat(l.window.width()));
    let wrong = stand_in_clipped(
        &canvas,
        &mock,
        Placement::of(longer, l.window),
        longer.intersect(l.window),
        UNIT,
    );
    rejects("a square scaled to the longer side", || {
        check_fit(&wrong, size);
    });
    // The shown placement left in the square.
    let shown = stand_in_clipped(&canvas, &mock, l.placement(), square, UNIT);
    rejects("the field left at the shown scale in the square", || {
        check_same_over(&fit.rgba, &shown, edges, "the shown scale")
    });
    let off_centre = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), square.size());
    let left = stand_in_clipped(
        &canvas,
        &mock,
        Placement::of(off_centre, l.window),
        off_centre,
        UNIT,
    );
    rejects("a square off centre", || {
        check_fit(&left, size);
    });
    // The rest of the window is the clear colour, not the figure: the corner pixel.
    assert_eq!(pixel(&fit.rgba, 0, 0), clear());
    assert_eq!(pixel(&fit.rgba, 2159, 1349), clear());
}

#[test]
fn f3_fill_modes_fit_taller_than_wide() {
    let canvas = canvas();
    let size = [810, 1350];
    let rgba = hidden_capture(&canvas, size, F3Fill::Fit, false);
    let b = check_fit(&rgba, size.map(|v| v as usize));
    assert_eq!(b, [0, 270, 810, 1080], "810 pixels, centred in 1350");
    let extend = hidden_capture(&canvas, size, F3Fill::Extend, false);
    rejects("Extend's window taken as Fit's", || {
        check_fit(&extend, size.map(|v| v as usize));
    });
}

/// The square's geometry at each boundary: wider, taller, exactly square, and a half pixel to round.
#[test]
fn f3_fill_modes_fit_rect_and_scale() {
    let at = |w: f32, h: f32, ppp: f32| {
        Layout::new(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(w, h)),
            ppp,
        )
    };
    let square = |l: &Layout, ppp| {
        let r: egui::Rect = l.fit_rect(ppp);
        [r.min.x, r.min.y, r.max.x, r.max.y]
    };
    assert_eq!(
        square(&at(1000.0, 600.0, 1.0), 1.0),
        [200.0, 0.0, 800.0, 600.0]
    );
    assert_eq!(
        square(&at(600.0, 1000.0, 1.0), 1.0),
        [0.0, 200.0, 600.0, 800.0]
    );
    assert_eq!(
        square(&at(700.0, 700.0, 1.0), 1.0),
        [0.0, 0.0, 700.0, 700.0]
    );
    // 201 pixels of room: the corner on a whole pixel (100.5 rounds to 101), the side kept.
    assert_eq!(
        square(&at(801.0, 600.0, 1.0), 1.0),
        [101.0, 0.0, 701.0, 600.0]
    );
    // At two pixels per point, a corner on a half point is a whole pixel.
    assert_eq!(
        square(&at(801.0, 600.0, 2.0), 2.0),
        [100.5, 0.0, 700.5, 600.0]
    );
    // k: the window's shorter side over the shown figure's width, either way round.
    let wide = at(2000.0, 1000.0, 1.0);
    assert_eq!(wide.fit_scale(), 1000.0 / f64::from(wide.field.width()));
    let tall = at(1000.0, 2000.0, 1.0);
    assert_eq!(tall.fit_scale(), 1000.0 / f64::from(tall.field.width()));
    // A window too small for a figure asks for no limitless refinement: the floor at one point.
    let mut small = at(1000.0, 600.0, 1.0);
    small.field.set_width(0.0);
    assert_eq!(small.fit_scale(), 600.0);
    small.field.set_width(1.0);
    assert_eq!(small.fit_scale(), 600.0);
    small.field.set_width(2.0);
    assert_eq!(small.fit_scale(), 300.0);
    rejects("the longer side's scale", || {
        assert_eq!(wide.fit_scale(), 2000.0 / f64::from(wide.field.width()))
    });
}

/// `drawn`'s arms: shown in either mode, and hidden in Extend, are the figure as it was; hidden in Fit is the square.
#[test]
fn f3_fill_modes_drawn_arms() {
    let l = layout();
    let ppp = PIXELS_PER_POINT;
    for fill in [F3Fill::Extend, F3Fill::Fit] {
        let shown = l.drawn(true, fill, ppp);
        assert_eq!(shown.clip, l.figure);
        assert_eq!(shown.placement, l.placement());
        assert_eq!(shown.span, l.span(true));
        assert_eq!(shown.fit, None);
    }
    let extend = l.drawn(false, F3Fill::Extend, ppp);
    assert_eq!(extend.clip, l.window);
    assert_eq!(extend.placement, l.placement());
    assert_eq!(extend.span, l.span(false));
    assert_eq!(extend.fit, None);
    let fit = l.drawn(false, F3Fill::Fit, ppp);
    let square = l.fit_rect(ppp);
    assert_eq!(fit.clip, square);
    assert_eq!(fit.placement, Placement::of(square, l.window));
    assert_eq!(fit.span, UNIT);
    assert_eq!(fit.fit, Some(l.fit_scale()));
    // The figure's view in pixels: Fit's clip is the square; `of` is Extend's.
    let view = FigureView::of_drawn(&l, &fit, ppp, [1, 2, 3]);
    assert_eq!(view.clip, [405, 0, 1755, 1350]);
    assert_eq!(view.size, SIZE);
    assert_eq!(view.placement, fit.placement);
    assert_eq!(view.background, [1, 2, 3]);
    let of = FigureView::of(&l, false, ppp, [1, 2, 3]);
    assert_eq!(of.clip, [0, 0, 2160, 1350]);
    assert_eq!(of.placement, l.placement());
    rejects("Fit's view taken as Extend's", || {
        assert_eq!(view.clip, of.clip)
    });
}

/// The fake quads refine only under Fit: `k = 1` is the shown layout's tree bit for bit, `k ≥ 2` goes deeper.
#[test]
fn f3_fill_modes_quads_finer_only_in_fit() {
    let mock = MockEngine::frozen();
    let plane = mock.plane().clone();
    let shown = quads::quads(&plane);
    assert_eq!(quads::quads_at(&plane, UNIT, 1.0), shown);
    let span = layout().placement().span();
    assert_eq!(
        quads::quads_at(&plane, span, 1.0),
        quads::quads_over(&plane, span),
        "Extend's quads"
    );
    assert_eq!(
        quads::bounds_at(&plane, span, 1.0),
        quads::bounds_over(&plane, span)
    );
    for k in [2.0, 3.0, 4.0] {
        let finer = quads::quads_at(&plane, UNIT, k);
        assert!(
            deepest(&finer) > deepest(&shown),
            "k = {k}: level {} against {}",
            deepest(&finer),
            deepest(&shown)
        );
        assert!(finer.len() > shown.len(), "k = {k}");
    }
    // About log₂ k levels finer: four times the scale, two levels, the frontier's quads below the depth cap.
    assert_eq!(
        deepest(&quads::quads_at(&plane, UNIT, 4.0)),
        deepest(&shown) + 2
    );
    // The capture's k, under 2, still refines somewhere.
    let k = layout().fit_scale();
    assert!(k > 1.0 && k < 2.0, "the capture's k: {k}");
    assert!(quads::quads_at(&plane, UNIT, k).len() > shown.len());
    // Controls: quads left at the shown scale; a rule dividing by `k`, which coarsens (R-468 E3's formula as written,
    // flagged on the PR); and a rule that refines the shown layout's quads.
    rejects("quads left at the shown scale", || {
        assert!(deepest(&quads::quads_at(&plane, UNIT, 1.0)) > deepest(&shown))
    });
    rejects("a rule dividing the quad's width by k", || {
        assert!(deepest(&quads::quads_at(&plane, UNIT, 1.0 / 2.0)) > deepest(&shown))
    });
    rejects("a split rule that refines the shown layout's quads", || {
        assert_eq!(quads::quads_at(&plane, UNIT, 2.0), shown)
    });
}

/// The capture draws the bounds at Fit's scale; `mock_tile_bounds` and Extend's stay at the shown scale.
#[test]
fn f3_fill_modes_tile_bounds_finer_in_fit_only() {
    let canvas = canvas();
    let l = layout();
    let square = l.fit_rect(PIXELS_PER_POINT);
    let p = Placement::of(square, l.window);
    let mut steps = vec![Step::Alt1];
    steps.extend(fit_then_f3());
    let fit = shoot(canvas.clone(), &steps);
    let mut mock = MockEngine::frozen();
    tile_bounds_on(&mut mock);
    mock.set_fit(Some(l.fit_scale()));
    let finer = stand_in_clipped(&canvas, &mock, p, square, UNIT);
    assert_eq!(fit.rgba, finer, "the bounds at Fit's scale");
    mock.set_fit(None);
    let coarse = stand_in_clipped(&canvas, &mock, p, square, UNIT);
    rejects("quads left at the shown scale", || {
        assert_eq!(fit.rgba, coarse)
    });
    // The shown capture and Extend's, tile bounds on, are at the shown scale: the mock_tile_bounds capture is as it was.
    let extend = shoot(canvas.clone(), &[Step::Alt1, Step::F3]);
    let placed = stand_in(&canvas, &mock, l.placement());
    assert_eq!(extend.rgba, placed, "Extend's bounds");
    let shown = shoot(canvas.clone(), &[Step::Alt1]);
    check_same_over(&shown.rgba, &extend.rgba, shown_px(), "the shown bounds");
}

/// The engine side: the span of the marks, and that Fit integrates what its scale asks, keeping what is held.
#[test]
fn f3_fill_modes_mock_integrates_at_fit_and_keeps() {
    let mut mock = MockEngine::frozen();
    let p = layout().placement();
    assert_eq!(mock.figure_span(p), p.span(), "outside Fit, the viewport's");
    mock.show_span(UNIT);
    let shown: Vec<_> = mock.integrated().to_vec();
    assert_eq!(shown, quads::quads(mock.plane()));
    mock.show_span(UNIT);
    assert_eq!(mock.integrated().len(), shown.len(), "the same span again");
    mock.set_fit(Some(2.0));
    assert_eq!(mock.figure_span(p), UNIT, "in Fit, the view's square");
    mock.show_span(UNIT);
    let after = mock.integrated();
    assert!(after.len() > shown.len(), "the same span at a finer scale");
    assert_eq!(&after[..shown.len()], &shown[..], "what is held is kept");
    let held = after.len();
    mock.set_fit(None);
    mock.show_span(UNIT);
    assert_eq!(
        mock.integrated().len(),
        held,
        "back: nothing new, nothing lost"
    );
    assert_eq!(mock.marks_over(UNIT), MockEngine::frozen().marks_over(UNIT));
    rejects("a span taken as the viewport's in Fit", || {
        let mut fit = MockEngine::frozen();
        fit.set_fit(Some(2.0));
        assert_eq!(fit.figure_span(p), p.span())
    });
}

/// F3 twice in `fill` on a mock app, `while_hidden` running with the layer hidden: the edits, the view, the layout rects
/// and the capture all as they were.
fn check_round_trip(
    canvas: &Arc<MockCanvas>,
    fill: F3Fill,
    while_hidden: impl FnOnce(&mut App<MockSide>),
) {
    let side = MockSide::new(MockEngine::frozen(), Some(canvas.clone()));
    let mut app = App::new(side, FORMAT);
    let mut h = headless();
    app.view.f3_fill = fill;
    frames(&mut h, &mut app, 3);
    let (view, count) = (view_bits(&app), edits(&mut app));
    let (rects, place) = (
        Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits),
        canvas.placement(),
    );
    let before = capture::render_frame(canvas, &mut app, &mut h);
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    while_hidden(&mut app);
    frames(&mut h, &mut app, 2);
    let hidden = capture::render_frame(canvas, &mut app, &mut h);
    assert_ne!(hidden.rgba, before.rgba, "hidden differs from shown");
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    frames(&mut h, &mut app, 2);
    let back = capture::render_frame(canvas, &mut app, &mut h);
    // The edits first, so a Fit that sends a `SetField` fails here and not on what it drew.
    assert_eq!(
        edits(&mut app),
        count,
        "{fill:?}: a SetField or an undo entry"
    );
    assert_eq!(view_bits(&app), view, "{fill:?}: the view");
    assert_eq!(
        back.rgba, before.rgba,
        "{fill:?}: the capture after F3 twice"
    );
    let after = Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits);
    assert_eq!(
        (
            after.figure,
            after.manifold_view,
            after.trajectory,
            after.footer
        ),
        (
            rects.figure,
            rects.manifold_view,
            rects.trajectory,
            rects.footer
        ),
        "{fill:?}: the layout"
    );
    assert_eq!(canvas.placement(), place, "{fill:?}: the placement");
}

#[test]
fn f3_fill_modes_f3_twice_returns_everything() {
    let canvas = canvas();
    check_round_trip(&canvas, F3Fill::Extend, |_| {});
    check_round_trip(&canvas, F3Fill::Fit, |_| {});
    // Control: a Fit that edits the zoom, one `SetField` halving the basis.
    rejects("a Fit that edits the zoom (a SetField)", || {
        check_round_trip(&canvas, F3Fill::Fit, |app| {
            let plane = app.snapshot().sim.plane.clone();
            let half = |q: &engine::contract::sim_config::Latent| q.map(|v| v * 0.5);
            app.set_field(crate::explore::manifold_view::basis_edit((
                half(&plane.q1),
                half(&plane.q2),
            )));
        })
    });
}

#[test]
fn f3_fill_modes_present_follows_the_mode() {
    let canvas = canvas();
    for fill in [F3Fill::Extend, F3Fill::Fit] {
        assert_eq!(
            hidden_capture(&canvas, SIZE, fill, true),
            hidden_capture(&canvas, SIZE, fill, false),
            "present mode against F3 in {fill:?}"
        );
    }
    let fit = hidden_capture(&canvas, SIZE, F3Fill::Fit, true);
    check_fit(&fit, SIZE.map(|v| v as usize));
    let extend = hidden_capture(&canvas, SIZE, F3Fill::Extend, true);
    rejects("present mode ignoring the mode", || assert_eq!(fit, extend));
}

/// The settings file `text` does not name the fill.
fn check_mode_not_saved(text: &str) {
    assert!(!text.contains("f3_fill") && !text.contains("Fit"), "{text}");
}

/// Extend at each launch: the choice is `ViewUI`'s, kept for the session and not across one (R-468 E4). The relaunch is
/// real: a new app reading the settings file the first saved, which a moved splitter shows it read.
#[test]
fn f3_fill_modes_extend_at_each_launch() {
    let check = |fill: F3Fill| assert_eq!(fill, F3Fill::Extend);
    assert_eq!(crate::app::initial_view().f3_fill, F3Fill::Extend);
    let dir = std::env::temp_dir().join(format!("principia-gui-f3-fill-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let launch = |path: &std::path::Path| {
        App::with_settings(
            MockSide::new(MockEngine::frozen(), None),
            FORMAT,
            crate::settings::Store::at(path),
        )
    };
    let path = dir.join("settings.json");
    let mut session = launch(&path);
    let mut h = headless();
    frames(&mut h, &mut session, 2);
    check(session.view.f3_fill);
    session.view.f3_fill = F3Fill::Fit;
    session
        .splits
        .set(crate::layout::Splitter::ManifoldFigure, 0.3);
    press(&mut h, &mut session, Key::F3, Modifiers::NONE);
    press(&mut h, &mut session, Key::F3, Modifiers::NONE);
    assert_eq!(session.view.f3_fill, F3Fill::Fit, "kept for the session");
    session.save_settings();
    let saved = std::fs::read_to_string(&path).expect("the settings saved");
    check_mode_not_saved(&saved);
    let relaunched = launch(&path);
    assert_eq!(relaunched.splits, session.splits, "the settings were read");
    check(relaunched.view.f3_fill);
    // Controls: a relaunch that came back in Fit; a settings file that holds the mode; a relaunch from a file that was
    // never saved, which shows the check on the splitter can fail.
    rejects("a mode that persists across a relaunch", || {
        check(session.view.f3_fill)
    });
    rejects("a settings file that holds the mode", || {
        check_mode_not_saved(&format!("{saved}\"f3_fill\": \"Fit\""))
    });
    rejects("a relaunch from nothing saved", || {
        assert_eq!(launch(&dir.join("unsaved.json")).splits, session.splits)
    });
    let _ = std::fs::remove_dir_all(&dir);
}
