//! `f3_fills_window_mock` (REQ-GUI-179; R-406; render_gui_spec §G1): with F3 hiding the egui layer the figure fills
//! the window. No pixel of the window is the clear colour; over the shown figure's rect the capture is pixel-identical
//! to the shown one; everywhere it equals the stand-in drawn at the shown layout's placement over the whole window,
//! more of the field at the same scale, never the shown view stretched. With tile bounds on, the fake quads' bounds
//! cover the window too, the extra quads at the shown quads' size. F3 again restores the shown capture pixel for
//! pixel. Neither toggle sends a `SetField` or adds an undo entry: the state is `ViewUI`'s (gui_state_contract §2).
//! Each check also runs on an input it must reject (R-176, through `rejects`; gui takes no dependency on validation,
//! R-187). These run on the GPU, on `PRIN_GPU_BACKEND`'s backend (R-206).

use std::sync::Arc;

use eframe::egui::{self, Key, Modifiers};
use eframe::egui_wgpu;
use engine::contract::interface::EngineInterface;
use engine::contract::render_state::Overlays;
use engine::contract::set_field::{Edit, RenderField, SetField};

use super::support::{headless, mock_app, press, rejects};
use crate::app::{clear_colour, App, FigureCallback};
use crate::capture::{self, shoot, Shot, Step, FORMAT, PIXELS_PER_POINT, SIZE};
use crate::headless::Headless;
use crate::layout::{Layout, Placement};
use crate::mock::canvas::{FigureCanvas, MockCanvas};
use crate::mock::quads::{self, UNIT};
use crate::mock::MockEngine;
use crate::side::MockSide;

pub(super) fn canvas() -> Arc<MockCanvas> {
    Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"))
}

/// The capture's layout.
pub(super) fn layout() -> Layout {
    Layout::new(
        Headless::new(SIZE, PIXELS_PER_POINT).screen(),
        PIXELS_PER_POINT,
    )
}

/// `r` in the capture's whole pixels: `[x0, y0, x1, y1)`.
pub(super) fn px(r: egui::Rect) -> [usize; 4] {
    [r.min.x, r.min.y, r.max.x, r.max.y].map(|v| (v * PIXELS_PER_POINT).round() as usize)
}

/// The shown figure's rect in the capture's pixels.
pub(super) fn shown_px() -> [usize; 4] {
    px(layout().figure)
}

pub(super) fn pixel(rgba: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * SIZE[0] as usize + x) * 4;
    rgba[i..i + 4].try_into().unwrap()
}

/// The clear colour, as the capture writes it.
pub(super) fn clear() -> [u8; 4] {
    clear_colour(&egui::Visuals::dark()).to_array()
}

/// `a` and `b` are identical over `rect`.
pub(super) fn check_same_over(a: &[u8], b: &[u8], rect: [usize; 4], what: &str) {
    for y in rect[1]..rect[3] {
        for x in rect[0]..rect[2] {
            assert_eq!(
                pixel(a, x, y),
                pixel(b, x, y),
                "{what}: differs at ({x}, {y})"
            );
        }
    }
}

/// No pixel of `rgba` is the clear colour: the figure covers the window.
pub(super) fn check_covers(rgba: &[u8], what: &str) {
    let clear = clear();
    for y in 0..SIZE[1] as usize {
        for x in 0..SIZE[0] as usize {
            assert_ne!(
                pixel(rgba, x, y),
                clear,
                "{what}: the clear colour at ({x}, {y})"
            );
        }
    }
}

/// `rgba` with everything outside `rect` the clear colour: a figure left in its rect.
pub(super) fn left_in(rgba: &[u8], rect: [usize; 4]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    let clear = clear();
    for y in 0..SIZE[1] as usize {
        for x in 0..SIZE[0] as usize {
            if !((rect[0]..rect[2]).contains(&x) && (rect[1]..rect[3]).contains(&y)) {
                let i = (y * SIZE[0] as usize + x) * 4;
                out[i..i + 4].copy_from_slice(&clear);
            }
        }
    }
    out
}

/// The stand-in of `mock`'s chart drawn by the figure canvas over the whole window at `placement`, with the marks over
/// the window's span, offscreen, alone: what the figure with the layer hidden must be.
pub(super) fn stand_in(
    canvas: &Arc<MockCanvas>,
    mock: &MockEngine,
    placement: Placement,
) -> Vec<u8> {
    canvas.set_chart(mock.plane());
    canvas.set_placement(placement, mock.marks_over(placement.span()));
    let window = layout().window;
    let callback = egui_wgpu::Callback::new_paint_callback(
        window,
        FigureCallback {
            canvas: Arc::new(FigureCanvas(canvas.clone())),
            format: FORMAT,
        },
    );
    let primitive = egui::ClippedPrimitive {
        clip_rect: window,
        primitive: egui::epaint::Primitive::Callback(callback),
    };
    let clear = clear_colour(&egui::Visuals::dark());
    capture::render(&**canvas, &Default::default(), &[primitive], clear)
}

/// A mock with tile bounds on.
pub(super) fn tile_bounds_on(mock: &mut MockEngine) {
    mock.set_field(SetField {
        edit: Edit::Render(RenderField::Overlays(Overlays {
            tile_bounds: true,
            ..Default::default()
        })),
        no_history: false,
    });
}

#[test]
fn f3_fills_window_mock_covers_the_window_at_the_shown_placement() {
    let canvas = canvas();
    let on = shoot(canvas.clone(), &[]);
    let off = shoot(canvas.clone(), &[Step::F3]);
    let back = shoot(canvas.clone(), &[Step::F3, Step::F3]);
    let rect = shown_px();
    check_covers(&off.rgba, "F3 off");
    check_same_over(&on.rgba, &off.rgba, rect, "the shown rect, F3 on and off");
    let placed = stand_in(&canvas, &MockEngine::frozen(), layout().placement());
    check_same_over(
        &off.rgba,
        &placed,
        [0, 0, 2160, 1350],
        "F3 off and the stand-in placed",
    );
    assert_eq!(
        on.rgba, back.rgba,
        "F3 twice is not the shown capture again"
    );
    // The shown view stretched to the window is not the shown view over its rect.
    let stretched = stand_in(&canvas, &MockEngine::frozen(), Placement::IDENTITY);
    rejects("the shown view stretched to the window", || {
        check_same_over(&on.rgba, &stretched, rect, "the shown rect, stretched")
    });
    // A figure left in its rect does not cover the window; nor does the shown capture, its chrome aside.
    rejects("a figure left in its rect", || {
        check_covers(&left_in(&off.rgba, rect), "left in its rect")
    });
    rejects("F3 off against F3 on outside the shown rect", || {
        check_same_over(&on.rgba, &off.rgba, [0, 0, 2160, 1350], "the window")
    });
}

/// The placement is the shown figure's rect in the window, and its span the window's share of the field: the shown
/// rect is `[0, 1]²`, the window more of it each side, at the same scale.
#[test]
fn f3_fills_window_mock_placement_and_span() {
    let l = layout();
    let p = l.placement();
    let (w, f) = (l.window, l.figure);
    let near = |a: f64, b: f64| (a - b).abs() < 1e-5;
    assert!(near(f64::from(p.half[0]), f64::from(f.width() / w.width())));
    assert!(near(
        f64::from(p.half[1]),
        f64::from(f.height() / w.height())
    ));
    let span = p.span();
    // s at the window's left edge: the figure's left edge is (figure.min.x − 0) / figure.width to its left.
    let want = [
        -f64::from(f.min.x - w.min.x) / f64::from(f.width()),
        -f64::from(w.max.y - f.max.y) / f64::from(f.height()),
        1.0 + f64::from(w.max.x - f.max.x) / f64::from(f.width()),
        1.0 + f64::from(f.min.y - w.min.y) / f64::from(f.height()),
    ];
    for (got, want) in span.iter().zip(want) {
        assert!(near(*got, want), "span {span:?}, want {want}");
    }
    assert!(span[0] < 0.0 && span[1] < 0.0 && span[2] > 1.0 && span[3] > 1.0);
    // The identity spans the unit square exactly, and a rect placed in itself is the identity.
    assert_eq!(Placement::IDENTITY.span(), UNIT);
    let own = Placement::of(f, f);
    assert!(own.centre.iter().all(|c| c.abs() < 1e-6) && own.half == [1.0, 1.0]);
    assert_eq!(l.figure_rect(true), f);
    assert_eq!(l.figure_rect(false), w);
    rejects("the window's span taken as the unit square", || {
        assert_eq!(l.placement().span(), UNIT)
    });
    // A rect placed off-centre, in the right half of the viewport and its top half: centre (0.5, 0.5), half 0.5.
    let viewport = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(4.0, 4.0));
    let quarter = egui::Rect::from_min_max(egui::pos2(2.0, 0.0), egui::pos2(4.0, 2.0));
    let q = Placement::of(quarter, viewport);
    assert_eq!((q.centre, q.half), ([0.5, 0.5], [0.5, 0.5]));
    assert_eq!(q.span(), [-1.0, -1.0, 1.0, 1.0]);
}

#[test]
fn f3_fills_window_mock_tile_bounds_cover_the_window() {
    let canvas = canvas();
    let on = shoot(canvas.clone(), &[Step::Alt1]);
    let off = shoot(canvas.clone(), &[Step::Alt1, Step::F3]);
    let rect = shown_px();
    check_covers(&off.rgba, "F3 off, tile bounds on");
    check_same_over(&on.rgba, &off.rgba, rect, "the shown rect with tile bounds");
    let mut mock = MockEngine::frozen();
    tile_bounds_on(&mut mock);
    let placed = stand_in(&canvas, &mock, layout().placement());
    check_same_over(
        &off.rgba,
        &placed,
        [0, 0, 2160, 1350],
        "F3 off with tile bounds",
    );
    // The bounds are drawn past the shown rect: left of it, the capture is not the bare stand-in.
    let bare = stand_in(&canvas, &MockEngine::frozen(), layout().placement());
    let left = [0, 0, rect[0], 1350];
    rejects("the extra quads' bounds left out", || {
        check_same_over(&off.rgba, &bare, left, "left of the shown rect")
    });
    // The extra quads are the shown quads' tree, sized against the shown figure: the shown quads are among them,
    // and away from the frontier the extra ones are as wide as the shown ones.
    let plane = mock.plane().clone();
    let shown = quads::quads(&plane);
    let all = quads::quads_over(&plane, layout().placement().span());
    assert!(
        all.len() > shown.len(),
        "{} quads over the window, {} shown",
        all.len(),
        shown.len()
    );
    for q in &shown {
        assert!(all.contains(q), "the shown quad {q:?} is gone");
    }
    let width = |q: &[f64; 4]| q[2] - q[0];
    let widest = |qs: &[[f64; 4]]| qs.iter().map(width).fold(0.0, f64::max);
    let extra: Vec<_> = all.iter().filter(|q| !shown.contains(q)).copied().collect();
    assert_eq!(widest(&extra), widest(&shown), "the extra quads' size");
    // Control: quads sized against the window, the plane's span stretched to it, are wider.
    let mut stretched = plane.clone();
    let span = layout().placement().span();
    let k = (span[2] - span[0]).max(span[3] - span[1]);
    stretched.q1 = stretched.q1.map(|v| v * k);
    stretched.q2 = stretched.q2.map(|v| v * k);
    rejects("quads sized against the window", || {
        assert_eq!(widest(&quads::quads(&stretched)), widest(&shown))
    });
}

/// Past the root the plane is covered by copies of the root, so the window's quads cover the whole window at the shown
/// quads' size; a copy's quads stay clear of the shown view, which keeps exactly [`quads::quads`]'s; and the copies
/// stop [`quads::MAX_ROOT_COPIES`] roots out.
#[test]
fn f3_fills_window_mock_quads_copied_past_the_root() {
    use crate::mock::quads::{touches, MAX_ROOT_COPIES, ROOT};
    let plane = MockEngine::frozen().plane().clone();
    let span = layout().placement().span();
    let shown = quads::quads(&plane);
    let all = quads::quads_over(&plane, span);
    // The window in the plane's (u, v): the mock's unit basis, centred on z₀'s (0.18, 0.41).
    let (cu, cv) = (plane.z0[0], plane.z0[1]);
    let window = [
        cu + 2.0 * span[0] - 1.0,
        cv + 2.0 * span[1] - 1.0,
        cu + 2.0 * span[2] - 1.0,
        cv + 2.0 * span[3] - 1.0,
    ];
    assert!(
        window[2] > ROOT,
        "the default window reaches past the root: {window:?}"
    );
    // Every point of the window, on a fine grid, lies in a quad.
    let covered = |qs: &[[f64; 4]], u: f64, v: f64| {
        qs.iter()
            .any(|q| q[0] <= u && u <= q[2] && q[1] <= v && v <= q[3])
    };
    for i in 0..=100 {
        for j in 0..=100 {
            let u = window[0] + (window[2] - window[0]) * f64::from(i) / 100.0;
            let v = window[1] + (window[3] - window[1]) * f64::from(j) / 100.0;
            assert!(covered(&all, u, v), "({u}, {v}) in no quad");
        }
    }
    rejects("the root alone", || {
        let central: Vec<_> = all.iter().filter(|q| q[2] <= ROOT).copied().collect();
        assert!(covered(&central, window[2] - 1e-3, cv), "past the root")
    });
    // Past the root the quads are the shown ones' size: the widest copy quad is as wide as the widest shown quad.
    let width = |q: &[f64; 4]| q[2] - q[0];
    let copies: Vec<_> = all.iter().filter(|q| q[2] > ROOT).copied().collect();
    assert!(!copies.is_empty());
    let widest = |qs: &[[f64; 4]]| qs.iter().map(width).fold(0.0, f64::max);
    assert_eq!(widest(&copies), widest(&shown));
    // The shown view's quads are the root's alone: every quad touching the shown view is a shown quad.
    let shown_view = [cu - 1.0, cv - 1.0, cu + 1.0, cv + 1.0];
    for q in all.iter().filter(|q| touches(q, &shown_view)) {
        assert!(shown.contains(q), "{q:?} touches the shown view");
    }
    // Over a span far wider than any window, the copies stop MAX_ROOT_COPIES roots out each way.
    let far_quads = quads::quads_over(&plane, [-20.0, -20.0, 21.0, 21.0]);
    let reach = (2.0 * f64::from(MAX_ROOT_COPIES) + 1.0) * ROOT;
    let max_u = far_quads
        .iter()
        .map(|q| q[2])
        .fold(f64::NEG_INFINITY, f64::max);
    let min_v = far_quads.iter().map(|q| q[1]).fold(f64::INFINITY, f64::min);
    let min_u = far_quads.iter().map(|q| q[0]).fold(f64::INFINITY, f64::min);
    let max_v = far_quads
        .iter()
        .map(|q| q[3])
        .fold(f64::NEG_INFINITY, f64::max);
    assert_eq!((min_u, min_v, max_u, max_v), (-reach, -reach, reach, reach));
    // The copies cover the plane: no two quads overlap, and every quad is the shown quads' size or finer.
    for (k, a) in far_quads.iter().enumerate() {
        assert!(width(a) > 0.0 && width(a) <= widest(&shown), "{a:?}");
        for b in &far_quads[k + 1..] {
            assert!(!quads::overlaps(a, b), "{a:?} and {b:?} overlap");
        }
    }
    // Zoomed far out, the shown view covers copies, and keeps the root's quads only, as before.
    let mut far = plane.clone();
    far.q1 = far.q1.map(|v| v * 100.0);
    far.q2 = far.q2.map(|v| v * 100.0);
    assert_eq!(quads::quads(&far), quads::quads_over(&far, UNIT));
    assert!(quads::quads(&far)
        .iter()
        .all(|q| q[2] <= ROOT && q[0] >= -ROOT));
    // `touches` at its edges: touching on each side counts, a hair apart does not.
    let a = [0.0, 0.0, 1.0, 1.0];
    for b in [
        [1.0, 0.0, 2.0, 1.0],
        [-1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0, 2.0],
        [0.0, -1.0, 1.0, 0.0],
    ] {
        assert!(touches(&a, &b), "{b:?} touches");
    }
    let e = 1e-9;
    for b in [
        [1.0 + e, 0.0, 2.0, 1.0],
        [-1.0, 0.0, -e, 1.0],
        [0.0, 1.0 + e, 1.0, 2.0],
        [0.0, -1.0, 1.0, -e],
    ] {
        assert!(!touches(&a, &b), "{b:?} is apart");
    }
}

/// The span reaches as far as it says each way, no farther: a quad past the window's span is not in view, one just
/// inside it is; and over [`UNIT`] the span changes nothing (`quads` is `quads_over` the unit square).
#[test]
fn f3_fills_window_mock_quads_over_a_span() {
    let plane = MockEngine::frozen().plane().clone();
    assert_eq!(quads::quads(&plane), quads::quads_over(&plane, UNIT));
    assert_eq!(quads::bounds(&plane), quads::bounds_over(&plane, UNIT));
    // Widening the span leftward only adds quads left of the shown ones, and each side's edge moves its own side.
    let shown = quads::quads(&plane);
    let min_u = |qs: &[[f64; 4]]| qs.iter().map(|q| q[0]).fold(f64::INFINITY, f64::min);
    let max_u = |qs: &[[f64; 4]]| qs.iter().map(|q| q[2]).fold(f64::NEG_INFINITY, f64::max);
    let min_v = |qs: &[[f64; 4]]| qs.iter().map(|q| q[1]).fold(f64::INFINITY, f64::min);
    let max_v = |qs: &[[f64; 4]]| qs.iter().map(|q| q[3]).fold(f64::NEG_INFINITY, f64::max);
    let left = quads::quads_over(&plane, [-0.5, 0.0, 1.0, 1.0]);
    assert!(min_u(&left) < min_u(&shown) && max_u(&left) == max_u(&shown));
    let right = quads::quads_over(&plane, [0.0, 0.0, 1.5, 1.0]);
    assert!(max_u(&right) > max_u(&shown) && min_u(&right) == min_u(&shown));
    let down = quads::quads_over(&plane, [0.0, -0.5, 1.0, 1.0]);
    assert!(min_v(&down) < min_v(&shown) && max_v(&down) == max_v(&shown));
    let up = quads::quads_over(&plane, [0.0, 0.0, 1.0, 1.5]);
    assert!(max_v(&up) > max_v(&shown) && min_v(&up) == min_v(&shown));
    rejects("a leftward span taken as the shown one", || {
        assert_eq!(left, shown)
    });
    // The marks follow: over the window's span, more bounds; tile bounds off, none.
    let mut mock = MockEngine::frozen();
    assert!(mock.marks_over(layout().placement().span()).is_empty());
    tile_bounds_on(&mut mock);
    assert_eq!(mock.marks(), mock.marks_over(UNIT));
    assert!(mock.marks_over(layout().placement().span()).len() > mock.marks().len());
}

/// The undo depth and the `SetField`s the mock logged, and the chart, after `app`'s frames so far.
fn state(app: &App<MockSide>) -> (u32, usize, engine::contract::sim_config::Plane) {
    let set_fields = app
        .console()
        .iter()
        .filter(|e| e.message.starts_with("SetField"))
        .count();
    (
        app.snapshot().history.undo_depth,
        set_fields,
        app.snapshot().sim.plane.clone(),
    )
}

/// Frames enough for a snapshot to be read after an edit.
fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..3 {
        let _ = h.frame(app, Vec::new());
    }
}

#[test]
fn f3_fills_window_mock_toggle_sends_no_set_field() {
    let mut app = mock_app();
    let mut h = headless();
    settle(&mut h, &mut app);
    let before = state(&app);
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    settle(&mut h, &mut app);
    assert!(!app.shown, "F3 hid the layer");
    assert_eq!(
        state(&app),
        before,
        "F3 off: the undo depth, the SetFields and the chart"
    );
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    settle(&mut h, &mut app);
    assert!(app.shown, "F3 again showed it");
    assert_eq!(state(&app), before, "F3 on again");
    // Control: an edit is seen by the same check.
    let mut z0 = app.snapshot().sim.plane.z0;
    z0[0] += 0.25;
    app.set_field(SetField {
        edit: Edit::Sim(engine::contract::set_field::SimField::Z0(z0)),
        no_history: false,
    });
    settle(&mut h, &mut app);
    rejects("a pan, which is an edit", || {
        assert_eq!(state(&app), before)
    });
}

/// `draws` rendered offscreen in one pass and one submission on `canvas`'s device, each canvas into its viewport
/// `[x, y, w, h]` of a 64 × 32 target: the pixels, RGBA8 row-major.
fn offscreen(
    canvas: &MockCanvas,
    draws: &[(&dyn engine::contract::canvas::Canvas, [f32; 4])],
) -> Vec<u8> {
    use engine::contract::canvas::Canvas;
    let context = canvas.context();
    let (device, queue) = (&context.device, &context.queue);
    let extent = wgpu::Extent3d {
        width: 64,
        height: 32,
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::COPY_SRC),
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    // 64 pixels of 4 bytes: one row is 256 bytes, the copy's alignment.
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256 * 32,
        usage: wgpu::BufferUsages::MAP_READ.union(wgpu::BufferUsages::COPY_DST),
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations::default(),
            })],
            ..Default::default()
        });
        for (c, [x, y, w, h]) in draws {
            pass.set_viewport(*x, *y, *w, *h, 0.0, 1.0);
            c.draw_figure(&mut pass, FORMAT);
        }
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(32),
            },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map failed"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll failed");
    let bytes = slice.get_mapped_range().expect("readback range").to_vec();
    readback.unmap();
    bytes
}

/// The right half, `x` in `[32, 64)`, of an [`offscreen`] render.
fn right_half(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks(256)
        .flat_map(|row| row[128..].to_vec())
        .collect()
}

/// The figure's placement never reaches the Chart builder's quick render or the Stain editor's preview: drawn beside
/// a placed figure in the same pass, flagged region and all, each is what it is alone, the field over its whole
/// viewport; the placed figure is not.
#[test]
fn f3_fills_window_mock_placement_does_not_leak() {
    use crate::mock::canvas::PreviewCanvas;
    let canvas = canvas();
    let mock = MockEngine::frozen();
    canvas.set_chart(mock.plane());
    canvas.set_marks(Vec::new());
    let left = [0.0, 0.0, 32.0, 32.0];
    let right = [32.0, 0.0, 32.0, 32.0];
    let figure = FigureCanvas(canvas.clone());
    let preview = PreviewCanvas(canvas.clone());
    let quick_alone = right_half(&offscreen(&canvas, &[(&*canvas, right)]));
    let preview_alone = right_half(&offscreen(&canvas, &[(&preview, right)]));
    // The quick render is the stand-in over its whole viewport: the figure placed at the identity, drawn.
    canvas.set_placement(Placement::IDENTITY, Vec::new());
    let identity = right_half(&offscreen(&canvas, &[(&figure, right)]));
    assert!(
        quick_alone == identity,
        "the quick render is not the field over its viewport"
    );
    assert!(
        identity.chunks(4).any(|p| p[..3] != [0, 0, 0]),
        "nothing drawn"
    );
    // The figure placed off-centre, half the viewport, its left past the shown rect flagged invalid.
    let shown = egui::Rect::from_min_max(egui::pos2(8.0, 8.0), egui::pos2(24.0, 24.0));
    let viewport = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(32.0, 32.0));
    canvas.set_placement(Placement::of(shown, viewport), Vec::new());
    canvas.flag_invalid([-1.0, -1.0, 0.0, 2.0]);
    let quick = right_half(&offscreen(&canvas, &[(&figure, left), (&*canvas, right)]));
    let previewed = right_half(&offscreen(&canvas, &[(&figure, left), (&preview, right)]));
    assert!(
        quick == quick_alone,
        "the quick render took the figure's placement"
    );
    assert!(
        previewed == preview_alone,
        "the preview took the figure's placement"
    );
    // Control: the placed figure in the same place is another picture.
    let placed = right_half(&offscreen(&canvas, &[(&figure, right)]));
    rejects("the placed figure as the quick render", || {
        assert!(placed == quick_alone, "placed")
    });
    assert_eq!(canvas.placement(), Placement::of(shown, viewport));
}

/// Segments of the figure, `[(s, t), (s, t)]`.
type Segments = Vec<[[f64; 2]; 2]>;

/// The bounds the old rule drew: every kept quad's edges whole, a copy's quads dropped where they touch the shown view.
fn dropped_on_touch(plane: &engine::contract::sim_config::Plane, span: [f64; 4]) -> Segments {
    let mut lines = Vec::new();
    for q in quads::quads_over(plane, span) {
        let c = [(q[0], q[1]), (q[2], q[1]), (q[2], q[3]), (q[0], q[3])];
        let p: Vec<[f64; 2]> = c
            .iter()
            .map(|c| quads::on_figure(plane, *c).expect("a figure point"))
            .collect();
        for i in 0..4 {
            lines.push([p[i], p[(i + 1) % 4]]);
        }
    }
    lines
}

/// Every column of the strips left and right of the shown figure, and every row of the strips under and over it,
/// crosses a bound beside the shown figure: a column `s` at some `t` in `[0, 1]`, a row `t` at some `s` in `[0, 1]`.
fn check_strips_bounded(lines: &Segments, span: [f64; 4], what: &str) {
    let edge = quads::CLIP_MARGIN + 1e-3;
    let crosses = |axis: usize, c: f64| {
        lines.iter().any(|[a, b]| {
            let (lo, hi) = (a[axis].min(b[axis]), a[axis].max(b[axis]));
            if !(lo <= c && c <= hi) || lo == hi {
                return false;
            }
            let f = (c - a[axis]) / (b[axis] - a[axis]);
            let other = a[1 - axis] + (b[1 - axis] - a[1 - axis]) * f;
            (0.0..=1.0).contains(&other)
        })
    };
    let steps = |from: f64, to: f64| {
        let n = ((to - from) / 0.004).floor() as usize;
        (0..=n).map(move |i| from + 0.004 * i as f64)
    };
    for (axis, from, to) in [
        (0, span[0], -edge),
        (0, 1.0 + edge, span[2]),
        (1, span[1], -edge),
        (1, 1.0 + edge, span[3]),
    ] {
        for c in steps(from, to) {
            assert!(
                crosses(axis, c),
                "{what}: {} {c} crosses no bound beside the figure",
                ["column", "row"][axis]
            );
        }
    }
}

/// The views the clipping is checked on: grid-aligned, `z₀` at (0, 0), where root quads touch the shown view without
/// overlapping it (the code review's repro); and past the root, `u` 1.5, and 8.18 (the gui review's repro).
const VIEWS: [(f64, f64); 3] = [(0.0, 0.0), (1.5, 0.41), (8.18, 0.41)];

/// The mock's plane with `z₀`'s first two coordinates `(u, v)`.
fn plane_at((u, v): (f64, f64)) -> engine::contract::sim_config::Plane {
    let mut plane = MockEngine::frozen().plane().clone();
    plane.z0[0] = u;
    plane.z0[1] = v;
    plane
}

/// No bound reaches into the shown figure but the shown quads' own: every segment wholly in `[0, 1]²` is one of
/// [`quads::bounds`].
fn check_only_shown_inside(lines: &Segments, shown: &Segments, what: &str) {
    for l in lines {
        let inside = l
            .iter()
            .all(|p| (0.0..=1.0).contains(&p[0]) && (0.0..=1.0).contains(&p[1]));
        assert!(
            !inside || shown.contains(l),
            "{what}: {l:?} inside the shown figure"
        );
    }
}

/// Grid-aligned and past the root, the bounds cover the strips of the window beside the shown figure (a quad
/// straddling its edge is bounded outside it), and none reaches into the shown figure but the shown quads' own (a
/// quad touching its edge is never drawn along it a second time). Controls: the rule that dropped a touching copy
/// leaves the strips bare past the root; the rule that drew every kept quad whole draws a touching root quad's edge
/// along the grid-aligned figure's edge.
#[test]
fn f3_fills_window_mock_bounds_beside_the_figure_past_the_root() {
    let span = layout().placement().span();
    for view in VIEWS {
        let plane = plane_at(view);
        let lines = quads::bounds_over(&plane, span);
        check_strips_bounded(&lines, span, &format!("{view:?}"));
        check_only_shown_inside(&lines, &quads::bounds(&plane), &format!("{view:?}"));
    }
    let past = plane_at(VIEWS[2]);
    let old = dropped_on_touch(&past, span);
    rejects(
        "the bounds dropped where a copy touches the shown view",
        || check_strips_bounded(&old, span, "dropped"),
    );
    let grid = plane_at(VIEWS[0]);
    let old = dropped_on_touch(&grid, span);
    rejects("a touching root quad drawn whole", || {
        check_only_shown_inside(&old, &quads::bounds(&grid), "whole")
    });
}

/// `outside` at its edges: a segment wholly inside the margin's square has no part outside; one crossing it keeps
/// both ends; one with an end inside keeps one piece; one wholly outside, or along a side past the margin, is kept
/// whole; one along the margin's side is inside.
#[test]
fn f3_fills_window_mock_outside_clips_at_the_margin() {
    use crate::mock::quads::outside;
    let m = 0.1;
    let same = |got: Vec<[[f64; 2]; 2]>, want: Vec<[[f64; 2]; 2]>| {
        let flat =
            |v: &[[[f64; 2]; 2]]| -> Vec<f64> { v.iter().flatten().flatten().copied().collect() };
        assert_eq!(got.len(), want.len(), "{got:?} against {want:?}");
        for (g, w) in flat(&got).iter().zip(flat(&want)) {
            assert!((g - w).abs() < 1e-12, "{got:?} against {want:?}");
        }
    };
    assert!(outside([[0.2, 0.5], [0.8, 0.5]], m).is_empty());
    same(
        outside([[-1.0, 0.5], [2.0, 0.5]], m),
        vec![[[-1.0, 0.5], [-0.1, 0.5]], [[1.1, 0.5], [2.0, 0.5]]],
    );
    same(
        outside([[0.5, -1.0], [0.5, 0.5]], m),
        vec![[[0.5, -1.0], [0.5, -0.1]]],
    );
    same(
        outside([[0.5, 0.5], [0.5, 2.0]], m),
        vec![[[0.5, 1.1], [0.5, 2.0]]],
    );
    let whole = [[-1.0, -1.0], [2.0, -1.0]];
    same(outside(whole, m), vec![whole]);
    for past in [
        [[-0.11, 0.0], [-0.11, 1.0]],
        [[1.11, 0.0], [1.11, 1.0]],
        [[0.0, 1.11], [1.0, 1.11]],
    ] {
        same(outside(past, m), vec![past]);
    }
    assert!(outside([[-0.1, 0.0], [-0.1, 1.0]], m).is_empty());
    assert!(outside([[1.1, 0.0], [1.1, 1.0]], m).is_empty());
    // A diagonal crossing a corner: both ends kept, the middle gone.
    let diag = outside([[-1.0, -1.0], [2.0, 2.0]], m);
    assert_eq!(diag.len(), 2);
    assert!((diag[0][1][0] + 0.1).abs() < 1e-12 && (diag[1][0][0] - 1.1).abs() < 1e-12);
    // A segment passing beside the square, clear of it on the diagonal, is whole.
    let beside = [[1.5, -1.0], [3.0, 0.5]];
    same(outside(beside, m), vec![beside]);
    rejects("a crossing segment kept whole", || {
        assert_eq!(outside([[-1.0, 0.5], [2.0, 0.5]], m).len(), 1)
    });
}

/// The app on the mock at the view `(u, v)` with tile bounds on, captured with the layer shown and then hidden.
fn shown_and_hidden(canvas: &Arc<MockCanvas>, view: (f64, f64)) -> (Shot, Shot) {
    let mut mock = MockEngine::frozen();
    let mut z0 = mock.plane().z0;
    (z0[0], z0[1]) = view;
    mock.set_field(SetField {
        edit: Edit::Sim(engine::contract::set_field::SimField::Z0(z0)),
        no_history: false,
    });
    tile_bounds_on(&mut mock);
    let mut app = App::new(MockSide::new(mock, Some(canvas.clone())), FORMAT);
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let on = capture::render_frame(canvas, &mut app, &mut h);
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    let _ = h.frame(&mut app, Vec::new());
    let off = capture::render_frame(canvas, &mut app, &mut h);
    (on, off)
}

/// The stand-in at `view` drawn alone over the window with `marks` as its tile bounds.
fn stand_in_with(canvas: &Arc<MockCanvas>, view: (f64, f64), lines: &Segments) -> Vec<u8> {
    let mut mock = MockEngine::frozen();
    let mut z0 = mock.plane().z0;
    (z0[0], z0[1]) = view;
    mock.set_field(SetField {
        edit: Edit::Sim(engine::contract::set_field::SimField::Z0(z0)),
        no_history: false,
    });
    let marks = lines
        .iter()
        .map(|[a, b]| crate::mock::canvas::Line {
            ends: [a.map(|v| v as f32), b.map(|v| v as f32)],
            rgba: crate::mock::TILE_BOUNDS_RGBA,
        })
        .collect();
    canvas.set_chart(mock.plane());
    canvas.set_placement(layout().placement(), marks);
    let window = layout().window;
    let callback = egui_wgpu::Callback::new_paint_callback(
        window,
        FigureCallback {
            canvas: Arc::new(FigureCanvas(canvas.clone())),
            format: FORMAT,
        },
    );
    let primitive = egui::ClippedPrimitive {
        clip_rect: window,
        primitive: egui::epaint::Primitive::Callback(callback),
    };
    capture::render(
        &**canvas,
        &Default::default(),
        &[primitive],
        clear_colour(&egui::Visuals::dark()),
    )
}

/// Grid-aligned and past the root, with tile bounds on, the two F3 states draw the shown figure identically, and as
/// the shown quads' bounds alone draw it (no edge drawn twice there); and F3 off bounds the strips beside it.
/// Controls: a rect grown by a pixel differs; the old whole-quad rule draws the grid-aligned figure's edge twice.
#[test]
fn f3_fills_window_mock_views_identical_over_the_rect() {
    let canvas = canvas();
    let rect = shown_px();
    let span = layout().placement().span();
    for view in VIEWS {
        let (on, off) = shown_and_hidden(&canvas, view);
        check_same_over(
            &on.rgba,
            &off.rgba,
            rect,
            &format!("{view:?}, tile bounds on"),
        );
        let alone = stand_in_with(&canvas, view, &quads::bounds(&plane_at(view)));
        check_same_over(
            &on.rgba,
            &alone,
            rect,
            &format!("{view:?}, the shown quads alone"),
        );
        rejects("a rect grown by a pixel", || {
            check_same_over(
                &on.rgba,
                &off.rgba,
                [rect[0] - 1, rect[1], rect[2], rect[3]],
                "grown",
            )
        });
        // Beside the figure, left of it, every column crosses a bound with F3 off.
        let bare = stand_in_with(&canvas, view, &Vec::new());
        for x in 0..rect[0] - 4 {
            assert!(
                (rect[1]..rect[3]).any(|y| pixel(&off.rgba, x, y) != pixel(&bare, x, y)),
                "{view:?}: column {x} beside the figure has no bound"
            );
        }
    }
    let old = stand_in_with(
        &canvas,
        VIEWS[0],
        &dropped_on_touch(&plane_at(VIEWS[0]), span),
    );
    let (on, _) = shown_and_hidden(&canvas, VIEWS[0]);
    rejects("the touching root quads' edges drawn whole", || {
        check_same_over(&on.rgba, &old, rect, "whole")
    });
}
