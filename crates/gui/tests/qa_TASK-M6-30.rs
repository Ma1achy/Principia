//! QA tests for TASK-M6-30, written from the requirement and its rulings, not from the implementation:
//!
//! - REQ-GUI-179 (R-406, R-407, R-390; render_gui_spec §G1): "On the mock engine, with the egui layer hidden by F3 and
//!   in present mode, the figure must fill the whole window, the stand-in showing more of the field at the same scale
//!   in every direction, each point of the field at the screen position it has in the shown layout, never the shown
//!   view stretched; past the shown figure's rect the stand-in must extend as an affine chart, with R-407's hatch
//!   fallback (a pixel the mock flags invalid hatched as forbidden); showing the layer, or Esc from present mode, must
//!   return the figure to its rect in the shown layout; and neither must send a SetField or add an undo entry."
//! - R-406: "the tile-bounds overlay ... is drawn by the mock into the figure, so it covers the extra quads too"; the
//!   task's Goal: "The mock's fake quads, and their bounds when the tile-bounds overlay is on (TASK-M6-27), cover the
//!   window too, the extra quads at the shown quads' size."
//!
//! The chart is chart_decoder_contract Part 3's, `z(s, t) = z₀ + (2s − 1) q₁ + (2t − 1) q₂`, `t` Y-up, the shown
//! figure's rect being `[0, 1]²`. "Affine past the rect, at the same scale, each point where the shown layout puts
//! it" is checked without reading the mock's placement: the field the F3-off capture draws `d` pixels past the shown
//! rect is the field the shown layout draws inside it once the view is panned by `d` pixels (a `SetField` of `z₀`).
//! The hatch is the render contract's `debug_invalid` (REQ-COL-055), through its CPU mirror. gui takes no dependency
//! on validation (R-187), so each check runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-30` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::sync::Arc;

use eframe::egui::{self, Event, Key, Modifiers};
use engine::contract::canvas::hatch::{hatch, hatch_stripe};
use engine::contract::canvas::Canvas;
use engine::contract::render_state::Overlays;
use engine::contract::set_field::{Edit, RenderField, SetField, SimField};
use engine::contract::sim_config::Plane;
use engine::contract::view_ui::{Mode, Window};
use gui::app::{clear_colour, App};
use gui::capture::{self, render_frame, Shot, Step, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::canvas::MockCanvas;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};

const W: usize = SIZE[0] as usize;
const H: usize = SIZE[1] as usize;

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

/// The shown figure's rect in the capture's whole pixels, `[x0, y0, x1, y1)`.
fn shown_px() -> [usize; 4] {
    let r = layout().figure;
    [r.min.x, r.min.y, r.max.x, r.max.y].map(|v| (v * PIXELS_PER_POINT).round() as usize)
}

fn inside(r: [usize; 4], x: usize, y: usize) -> bool {
    (r[0]..r[2]).contains(&x) && (r[1]..r[3]).contains(&y)
}

fn px(rgba: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * W + x) * 4;
    rgba[i..i + 4].try_into().unwrap()
}

fn clear() -> [u8; 4] {
    clear_colour(&egui::Visuals::dark()).to_array()
}

/// An app on the frozen mock, drawing through `canvas` (`None`: no figure), and its headless context, settled.
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

fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key) {
    for events in capture::key(key, Modifiers::NONE) {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

fn edit(h: &mut Headless, app: &mut App<MockSide>, field: SimField) {
    app.set_field(SetField {
        edit: Edit::Sim(field),
        no_history: false,
    });
    settle(h, app);
}

fn tile_bounds_on(h: &mut Headless, app: &mut App<MockSide>) {
    app.set_field(SetField {
        edit: Edit::Render(RenderField::Overlays(Overlays {
            tile_bounds: true,
            ..Default::default()
        })),
        no_history: false,
    });
    settle(h, app);
}

fn add(a: &[f64; 8], k: f64, b: &[f64; 8]) -> [f64; 8] {
    std::array::from_fn(|i| a[i] + k * b[i])
}

/// The undo depth, the `SetField`s the contract's log holds and the chart: what a view-only toggle must leave alone.
fn state(app: &App<MockSide>) -> (u32, usize, Plane) {
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

/// `draws`, each canvas into its viewport `[x, y, w, h]` (whole pixels), on a `size` target first cleared to black,
/// in one pass on `device`'s canvas: the pixels, RGBA8 row-major, unpadded.
fn offscreen(on: &MockCanvas, size: [u32; 2], draws: &[(&dyn Canvas, [u32; 4])]) -> Vec<u8> {
    let context = on.context();
    let (device, queue) = (&context.device, &context.queue);
    let extent = wgpu::Extent3d {
        width: size[0],
        height: size[1],
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("qa offscreen"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::COPY_SRC),
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let row = size[0] * 4;
    let padded = row.div_ceil(256) * 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("qa readback"),
        size: u64::from(padded * size[1]),
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
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        for (c, [x, y, w, h]) in draws {
            pass.set_viewport(*x as f32, *y as f32, *w as f32, *h as f32, 0.0, 1.0);
            c.draw_figure(&mut pass, FORMAT);
        }
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(size[1]),
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
        .chunks(padded as usize)
        .flat_map(|r| r[..row as usize].to_vec())
        .collect()
}

// --- Affine past the shown rect, at the same scale, in every direction ----------------------------------------------

/// Over `rect`, `shown` at `(x, y)` against `off` at `(x − dx, y − dy)`: how many pixels differ by more than `TOL` in a
/// channel, how many were compared, and how many of those lie outside the shown rect in `off`.
fn shifted_gap(shown: &[u8], off: &[u8], rect: [usize; 4], d: [i64; 2]) -> (usize, usize, usize) {
    let figure = shown_px();
    let (mut bad, mut compared, mut past) = (0, 0, 0);
    for y in rect[1]..rect[3] {
        for x in rect[0]..rect[2] {
            let (ox, oy) = (x as i64 - d[0], y as i64 - d[1]);
            if ox < 0 || oy < 0 || ox >= W as i64 || oy >= H as i64 {
                continue;
            }
            let (ox, oy) = (ox as usize, oy as usize);
            compared += 1;
            if !inside(figure, ox, oy) {
                past += 1;
            }
            let (a, b) = (px(shown, x, y), px(off, ox, oy));
            if (0..3).any(|c| a[c].abs_diff(b[c]) > TOL) {
                bad += 1;
            }
        }
    }
    (bad, compared, past)
}

/// The per-channel allowance for f32 rounding of a different `z₀` in the stand-in's smooth noise.
const TOL: u8 = 2;

/// At most one compared pixel in this many may differ by more than [`TOL`]: the stand-in's bands are `fract(4n)`, so
/// a pixel on a band's edge flips colour under f32 rounding (measured: 1 pixel in about 540 000). A pan a pixel short
/// moves every band edge, some percent of the pixels.
const FLIPS: usize = 10_000;

fn check_shifted(shown: &[u8], off: &[u8], rect: [usize; 4], d: [i64; 2], what: &str) -> usize {
    let (bad, compared, past) = shifted_gap(shown, off, rect, d);
    assert!(
        bad * FLIPS <= compared,
        "{what}: the field past the shown rect is not the field panned into it ({bad} of {compared} pixels differ)"
    );
    past
}

/// The F3-off capture past the shown rect is the field the shown layout draws inside it once panned there: for each
/// of the four sides, a pan of `z₀` by the strip's width in pixels (`2 d / width · q₁`, or `q₂` for height, `t` Y-up)
/// brings the strip into the shown rect, and the shown capture there equals the F3-off capture `d` pixels away. So
/// the extra field is the same chart, affine, at the same scale, each point where the shown layout puts it; at the
/// default basis and zoomed in ×2. The allowance ([`TOL`], [`FLIPS`]) is f32 rounding of a different `z₀`, not a
/// scale or an offset: a pan one pixel short, or the shown view stretched to the window, fails.
#[test]
fn qa_f3_off_past_the_rect_is_the_field_panned_into_it() {
    let canvas = canvas();
    let f = shown_px();
    let (fw, fh) = ((f[2] - f[0]) as f64, (f[3] - f[1]) as f64);
    // Each strip fits in the shown rect, so one pan brings all of it in.
    assert!(
        f[0] as f64 <= fw
            && (W - f[2]) as f64 <= fw
            && f[1] as f64 <= fh
            && (H - f[3]) as f64 <= fh
    );
    for zoom in [1.0, 0.5] {
        let (mut app, mut h) = app_on(Some(canvas.clone()));
        let plane = app.snapshot().sim.plane.clone();
        let (q1, q2) = (plane.q1.map(|v| v * zoom), plane.q2.map(|v| v * zoom));
        if zoom != 1.0 {
            edit(&mut h, &mut app, SimField::Basis { q1, q2 });
        }
        let z0 = app.snapshot().sim.plane.z0;
        press(&mut h, &mut app, Key::F3);
        assert!(!app.shown);
        let off = render_frame(&canvas, &mut app, &mut h);
        // The strips: left, right, top, bottom; `d` the shift in pixels from the F3-off capture to the panned shown
        // one, and the strip's area.
        let sides: [(&str, [i64; 2], [f64; 8], usize); 4] = [
            (
                "left",
                [f[0] as i64, 0],
                add(&z0, -2.0 * f[0] as f64 / fw, &q1),
                f[0] * (f[3] - f[1]),
            ),
            (
                "right",
                [-((W - f[2]) as i64), 0],
                add(&z0, 2.0 * (W - f[2]) as f64 / fw, &q1),
                (W - f[2]) * (f[3] - f[1]),
            ),
            (
                "top",
                [0, f[1] as i64],
                add(&z0, 2.0 * f[1] as f64 / fh, &q2),
                f[1] * (f[2] - f[0]),
            ),
            (
                "bottom",
                [0, -((H - f[3]) as i64)],
                add(&z0, -2.0 * (H - f[3]) as f64 / fh, &q2),
                (H - f[3]) * (f[2] - f[0]),
            ),
        ];
        for (side, d, panned, strip) in sides {
            let (mut shown_app, mut sh) = app_on(Some(canvas.clone()));
            if zoom != 1.0 {
                edit(&mut sh, &mut shown_app, SimField::Basis { q1, q2 });
            }
            edit(&mut sh, &mut shown_app, SimField::Z0(panned));
            assert!(shown_app.shown);
            let shown = render_frame(&canvas, &mut shown_app, &mut sh);
            let what = format!("zoom {zoom}, {side}");
            let past = check_shifted(&shown.rgba, &off.rgba, f, d, &what);
            assert_eq!(past, strip, "{what}: the whole strip compared");
            // Control: a pan one pixel short is another picture.
            let short = [d[0] - d[0].signum(), d[1] - d[1].signum()];
            rejects(&format!("{what}, a pixel short"), || {
                check_shifted(&shown.rgba, &off.rgba, f, short, "short");
            });
        }
        // Control: the shown view stretched to the window (the field's [0, 1]² over the whole window) is not the F3-off
        // capture past the rect.
        let stretched = {
            let c: &MockCanvas = &canvas;
            c.set_chart(&app.snapshot().sim.plane);
            c.set_marks(Vec::new());
            offscreen(&canvas, SIZE, &[(c, [0, 0, SIZE[0], SIZE[1]])])
        };
        rejects(&format!("zoom {zoom}, the shown view stretched"), || {
            check_shifted(&stretched, &off.rgba, [0, 0, f[0], H], [0, 0], "stretched");
        });
    }
}

// --- The placement stays the figure's ---------------------------------------------------------------------------------

/// `rect` (whole pixels) of the capture, shrunk by `inset` (left, top, right, bottom), against `reference`, a
/// `rect`-sized render drawn at the origin, within the f32 rounding [`TOL`] and [`FLIPS`] allow for a viewport placed
/// elsewhere on the target.
fn check_region(rgba: &[u8], rect: [usize; 4], reference: &[u8], inset: [usize; 4], what: &str) {
    let rw = rect[2] - rect[0];
    let (mut bad, mut compared) = (0, 0);
    for y in rect[1] + inset[1]..rect[3] - inset[3] {
        for x in rect[0] + inset[0]..rect[2] - inset[2] {
            let i = ((y - rect[1]) * rw + (x - rect[0])) * 4;
            let (a, b) = (px(rgba, x, y), &reference[i..i + 4]);
            compared += 1;
            if (0..3).any(|c| a[c].abs_diff(b[c]) > TOL) {
                bad += 1;
            }
        }
    }
    assert!(
        compared > 0 && bad * FLIPS <= compared,
        "{what}: {bad} of {compared} pixels differ from the reference"
    );
}

/// The quick render's inset: two pixels each side, and on the right the width of egui's floating scroll bar, which
/// the Chart builder's scroll area draws over its content's right edge.
const QUICK_INSET: [usize; 4] = [2, 2, 24, 2];

/// The depth, in pixels, of the shading egui's scroll area draws over its content at the bottom edge of its view.
const SCROLL_FADE: usize = 64;

fn whole(r: egui::Rect) -> [usize; 4] {
    [r.min.x, r.min.y, r.max.x, r.max.y].map(|v| (v * PIXELS_PER_POINT).round() as usize)
}

/// The figure's placement, and a region flagged invalid past it, never reach the Chart builder's quick render or the
/// Stain editor's preview, drawn in the same session after the figure: each is the field over its own viewport, as the
/// canvas draws it alone (render_gui_spec §G7: "square, as the view"; Part II §1's preview of the figure).
#[test]
fn qa_placement_never_reaches_the_quick_render_or_the_preview() {
    let canvas = canvas();
    let (mut app, mut h) = app_on(Some(canvas.clone()));
    // The figure drawn hidden and shown, a region left of it flagged invalid: the placed draw's state is all set.
    canvas.flag_invalid([-10.0, -10.0, 0.0, 10.0]);
    press(&mut h, &mut app, Key::F3);
    let _ = render_frame(&canvas, &mut app, &mut h);
    press(&mut h, &mut app, Key::F3);
    app.view.windows.open.push(Window::ChartBuilder);
    settle(&mut h, &mut app);
    // The builder's content is taller than its window: scroll until the quick render is in view.
    let frame = layout().manifold_view;
    let in_view = |app: &App<MockSide>| {
        app.windows
            .chart
            .previews
            .is_some_and(|p| frame.shrink(4.0).contains_rect(p[1]))
    };
    let _ = h.frame(&mut app, vec![Event::PointerMoved(frame.center())]);
    for _ in 0..40 {
        if in_view(&app) {
            break;
        }
        let _ = h.frame(
            &mut app,
            vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -2.0),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
            }],
        );
        settle(&mut h, &mut app);
    }
    assert!(in_view(&app), "the quick render never scrolled into view");
    let shot = render_frame(&canvas, &mut app, &mut h);
    let quick = app
        .windows
        .chart
        .previews
        .expect("the Chart builder drew its previews")[1];
    let q = whole(quick);
    let size = [(q[2] - q[0]) as u32, (q[3] - q[1]) as u32];
    assert!(
        size[0] >= 32 && size[1] >= 32,
        "the quick render is {size:?}"
    );
    // The reference: the engine side's canvas for the quick render, drawn alone over a viewport of its size.
    let plain = app.side().canvas().expect("the mock's canvas");
    let reference = offscreen(&canvas, size, &[(&*plain, [0, 0, size[0], size[1]])]);
    // Only the rows clear of the scroll area's bottom edge.
    let bottom = whole(frame)[3] - SCROLL_FADE;
    let mut inset = QUICK_INSET;
    inset[3] = inset[3].max(q[3].saturating_sub(bottom));
    assert!(
        q[3] - inset[3] >= q[1] + 64,
        "fewer than 64 rows of the quick render in view"
    );
    check_region(&shot.rgba, q, &reference, inset, "the quick render");
    // Control: the figure's placed draw over the same viewport is another picture.
    let placed = app
        .side()
        .figure(layout().placement())
        .expect("the figure canvas");
    let wrong = offscreen(&canvas, size, &[(&*placed, [0, 0, size[0], size[1]])]);
    rejects("the placed figure as the quick render", || {
        check_region(&shot.rgba, q, &wrong, inset, "placed")
    });
    // The Stain editor's preview, after the placed figure was drawn hidden and shown.
    app.view.windows.open.clear();
    press(&mut h, &mut app, Key::F3);
    let _ = render_frame(&canvas, &mut app, &mut h);
    press(&mut h, &mut app, Key::F3);
    let _ = render_frame(&canvas, &mut app, &mut h);
    app.view.mode = Mode::Stain;
    settle(&mut h, &mut app);
    let stain = render_frame(&canvas, &mut app, &mut h);
    let p = whole(app.stain.preview.expect("the Stain page drew its preview"));
    let psize = [(p[2] - p[0]) as u32, (p[3] - p[1]) as u32];
    assert!(psize[0] >= 32 && psize[1] >= 32, "the preview is {psize:?}");
    let preview = app.side().preview().expect("the mock's preview");
    let reference = offscreen(&canvas, psize, &[(&*preview, [0, 0, psize[0], psize[1]])]);
    check_region(&stain.rgba, p, &reference, [2; 4], "the Stain preview");
    // Control: the preview with the field at a placement, its [0, 1]² the middle half of its viewport (drawn over a
    // viewport twice its size, cut to its middle), is another picture.
    let big = [2 * psize[0], 2 * psize[1]];
    let wide = offscreen(&canvas, big, &[(&*preview, [0, 0, big[0], big[1]])]);
    let (bw, (ox, oy)) = (
        big[0] as usize,
        ((psize[0] / 2) as usize, (psize[1] / 2) as usize),
    );
    let middle: Vec<u8> = (0..psize[1] as usize)
        .flat_map(|y| (0..psize[0] as usize * 4).map(move |i| (y, i)))
        .map(|(y, i)| wide[((y + oy) * bw + ox) * 4 + i])
        .collect();
    rejects("the preview at a placement", || {
        check_region(&stain.rgba, p, &middle, [2; 4], "placed preview")
    });
}

// --- The tile bounds cover the window --------------------------------------------------------------------------------

/// Pixels that differ between two captures.
fn diff(a: &[u8], b: &[u8]) -> Vec<bool> {
    a.chunks(4)
        .zip(b.chunks(4))
        .map(|(p, q)| p[..3] != q[..3])
        .collect()
}

/// For each column of `d` (a `W × H` mask) inside `cols`, whether it crosses a horizontal line of the mask (a run of at
/// least `RUN` set pixels along the row through it), and for each row inside `rows`, whether it crosses a vertical one.
const RUN: usize = 6;

fn crossings(d: &[bool], cols: [usize; 2], rows: [usize; 2]) -> (Vec<usize>, Vec<usize>) {
    let at = |x: usize, y: usize| d[y * W + x];
    let run_x = |x: usize, y: usize| {
        let (mut a, mut b) = (x, x);
        while a > 0 && at(a - 1, y) {
            a -= 1;
        }
        while b + 1 < W && at(b + 1, y) {
            b += 1;
        }
        b - a + 1
    };
    let run_y = |x: usize, y: usize| {
        let (mut a, mut b) = (y, y);
        while a > 0 && at(x, a - 1) {
            a -= 1;
        }
        while b + 1 < H && at(x, b + 1) {
            b += 1;
        }
        b - a + 1
    };
    let bare_cols = (cols[0]..cols[1])
        .filter(|&x| !(rows[0]..rows[1]).any(|y| at(x, y) && run_x(x, y) >= RUN))
        .collect();
    let bare_rows = (rows[0]..rows[1])
        .filter(|&y| !(cols[0]..cols[1]).any(|x| at(x, y) && run_y(x, y) >= RUN))
        .collect();
    (bare_cols, bare_rows)
}

/// With tile bounds on and F3 off, the fake quads' bounds cover the window, the extra quads at the shown quads' size
/// (the task's Goal; R-406: the overlay "covers the extra quads too"): every column of the window crosses a quad's
/// top or bottom edge and every row a quad's side, as every column and row of the shown rect does with the layer
/// shown, the window being larger than the largest shown quad. A figure whose bounds stop short of the window's edge
/// leaves columns or rows crossing none.
#[test]
fn qa_f3_off_tile_bounds_cover_the_window() {
    let canvas = canvas();
    let f = shown_px();
    let shot = |hide: bool, bounds: bool| {
        let (mut app, mut h) = app_on(Some(canvas.clone()));
        if bounds {
            tile_bounds_on(&mut h, &mut app);
        }
        if hide {
            press(&mut h, &mut app, Key::F3);
        }
        render_frame(&canvas, &mut app, &mut h)
    };
    // The shown layout: inside the shown rect every column and row crosses a bound.
    let shown = diff(&shot(false, true).rgba, &shot(false, false).rgba);
    let (cols, rows) = crossings(&shown, [f[0], f[2]], [f[1], f[3]]);
    assert!(
        cols.is_empty() && rows.is_empty(),
        "the shown quads leave columns {} and rows {} bare",
        cols.len(),
        rows.len()
    );
    // F3 off: over the whole window.
    let off_on = shot(true, true);
    let off = diff(&off_on.rgba, &shot(true, false).rgba);
    let (cols, rows) = crossings(&off, [0, W], [0, H]);
    let span = |v: &[usize]| v.first().zip(v.last()).map(|(a, b)| [*a, *b + 1]);
    assert!(
        cols.is_empty() && rows.is_empty(),
        "with F3 off the tile bounds leave {} columns bare ({:?}) and {} rows bare ({:?}), of a {W} × {H} window",
        cols.len(),
        span(&cols),
        rows.len(),
        span(&rows)
    );
    // Control: the bounds left in the shown rect leave the window's columns bare.
    let left: Vec<bool> = (0..W * H)
        .map(|i| off[i] && inside(f, i % W, i / W))
        .collect();
    rejects("the bounds left in the shown rect", || {
        let (cols, rows) = crossings(&left, [0, W], [0, H]);
        assert!(cols.is_empty() && rows.is_empty());
    });
}

// --- R-407's fallback -----------------------------------------------------------------------------------------------

/// The hatch's colour at the capture's pixel `(x, y)`: `debug_invalid` at the pixel's centre (REQ-COL-055).
fn hatch_at(x: usize, y: usize) -> [u8; 4] {
    let [r, g, b] = hatch().colours[hatch_stripe([x as f64 + 0.5, y as f64 + 0.5])];
    [r, g, b, 255]
}

/// Every pixel outside `rect` is the hatch.
fn check_hatched_outside(rgba: &[u8], rect: [usize; 4]) {
    for y in 0..H {
        for x in 0..W {
            if !inside(rect, x, y) {
                assert_eq!(px(rgba, x, y), hatch_at(x, y), "not hatched at ({x}, {y})");
            }
        }
    }
}

/// With the mock's test hook flagging the whole plane invalid, F3 off draws every pixel past the shown rect with the
/// hatch, never the stand-in or the clear colour, and the shown rect as the shown layout draws it, so the two states
/// stay pixel-identical over it (REQ-GUI-179; R-407's fallback is past `[0, 1]²`); the shown layout is unchanged by
/// the flag. A flag on one side hatches that side only.
#[test]
fn qa_f3_off_flagged_pixels_are_hatched_past_the_rect_only() {
    let canvas = canvas();
    let f = shown_px();
    let take = |hide: bool| {
        let (mut app, mut h) = app_on(Some(canvas.clone()));
        if hide {
            press(&mut h, &mut app, Key::F3);
        }
        render_frame(&canvas, &mut app, &mut h)
    };
    let (plain_on, plain_off) = (take(false), take(true));
    canvas.flag_invalid([-100.0, -100.0, 100.0, 100.0]);
    let (on, off) = (take(false), take(true));
    assert_eq!(
        on.rgba, plain_on.rgba,
        "the shown layout changed under the flag"
    );
    check_hatched_outside(&off.rgba, f);
    let clear = clear();
    assert!(
        off.rgba.chunks(4).all(|p| p != clear),
        "the clear colour with the plane flagged"
    );
    for y in f[1]..f[3] {
        for x in f[0]..f[2] {
            assert_eq!(
                px(&off.rgba, x, y),
                px(&plain_on.rgba, x, y),
                "the shown rect at ({x}, {y})"
            );
        }
    }
    // Both stripes drawn, so it is the hatch and not a flat colour.
    let colours = hatch().colours;
    for c in colours {
        assert!(off.rgba.chunks(4).any(|p| p[..3] == c), "no {c:?} stripe");
    }
    rejects("the stand-in past the rect", || {
        check_hatched_outside(&plain_off.rgba, f)
    });
    // A flag right of the shown rect only, past the window's right edge: the left strip stays the stand-in.
    canvas.flag_invalid([1.0, -100.0, 100.0, 100.0]);
    let right = take(true);
    for y in 0..H {
        for x in 0..f[0] {
            assert_eq!(
                px(&right.rgba, x, y),
                px(&plain_off.rgba, x, y),
                "the left strip at ({x}, {y})"
            );
        }
        for x in f[2] + 1..W {
            assert_eq!(
                px(&right.rgba, x, y),
                hatch_at(x, y),
                "the right strip at ({x}, {y})"
            );
        }
    }
    rejects("the left strip hatched", || {
        for y in 0..H {
            for x in 0..f[0] {
                assert_eq!(px(&off.rgba, x, y), px(&plain_off.rgba, x, y));
            }
        }
    });
    canvas.flag_invalid([0.0; 4]);
}

// --- Present mode, and the layer shown again --------------------------------------------------------------------------

/// The steps of the screenshot case `01_main/mock_present`, as checked in.
fn present_steps() -> Vec<Step> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/screenshot/01_main/cases.json"
    );
    let cases: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("01_main's cases"))
            .expect("JSON");
    let case = cases["cases"]
        .as_array()
        .expect("a list")
        .iter()
        .find(|c| c["name"] == "mock_present")
        .expect("a mock_present case");
    case["surface"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| Step::parse(s.as_str().expect("a name")).expect("a step"))
        .collect()
}

/// The checked-in `mock_present` case enters present mode, whose capture is the F3-off one: the figure fills the
/// window, no pixel the clear colour, no chrome named. Esc returns the capture before present mode pixel for pixel.
#[test]
fn qa_present_case_fills_the_window_as_f3_off() {
    let canvas = canvas();
    let steps = present_steps();
    let present = capture::shoot(canvas.clone(), &steps);
    let f3_off = capture::shoot(canvas.clone(), &[Step::F3]);
    let clear = clear();
    assert!(
        present.rgba.chunks(4).all(|p| p != clear),
        "the clear colour in present mode"
    );
    assert!(
        present.rgba == f3_off.rgba,
        "present mode is not the F3-off capture"
    );
    assert!(present.names.is_empty(), "chrome named in present mode");
    let before = capture::shoot(canvas.clone(), &steps[..steps.len() - 1]);
    let back = capture::shoot(
        canvas.clone(),
        &[steps.as_slice(), &[Step::Escape]].concat(),
    );
    assert!(
        back.rgba == before.rgba,
        "Esc did not restore the capture before present mode"
    );
    rejects("the capture before present mode as present mode", || {
        assert!(before.rgba == present.rgba)
    });
}

/// With the layer shown, the figure is in its rect and nowhere else: outside the shown rect the capture is the capture
/// of the same app drawing no figure, before F3 and after F3 twice. With it hidden, it fills the window. Neither
/// toggle, nor present mode in and out, sends a SetField, adds an undo entry or moves the chart.
#[test]
fn qa_layer_shown_returns_the_figure_to_its_rect_and_edits_nothing() {
    let canvas = canvas();
    let f = shown_px();
    let (mut bare_app, mut bh) = app_on(None);
    let bare = render_frame(&canvas, &mut bare_app, &mut bh);
    let (mut app, mut h) = app_on(Some(canvas.clone()));
    let before = state(&app);
    let check_outside = |a: &Shot, b: &Shot, what: &str| {
        for y in 0..H {
            for x in 0..W {
                if !inside(f, x, y) {
                    assert_eq!(px(&a.rgba, x, y), px(&b.rgba, x, y), "{what} at ({x}, {y})");
                }
            }
        }
    };
    let first = render_frame(&canvas, &mut app, &mut h);
    check_outside(&first, &bare, "shown, before F3");
    press(&mut h, &mut app, Key::F3);
    let hidden = render_frame(&canvas, &mut app, &mut h);
    assert_eq!(state(&app), before, "F3 off edited");
    press(&mut h, &mut app, Key::F3);
    let again = render_frame(&canvas, &mut app, &mut h);
    assert_eq!(state(&app), before, "F3 on again edited");
    check_outside(&again, &bare, "shown, after F3 twice");
    assert!(
        again.rgba == first.rgba,
        "F3 twice is not the first capture"
    );
    // Control: the hidden layer against the same app drawing no figure, its layer hidden too, differs outside the rect.
    press(&mut bh, &mut bare_app, Key::F3);
    let bare_hidden = render_frame(&canvas, &mut bare_app, &mut bh);
    rejects("the hidden layer's figure kept to its rect", || {
        check_outside(&hidden, &bare_hidden, "hidden")
    });
    // Present mode, in and out, by its own flag's keys: Enter on Present, Esc.
    for step in present_steps() {
        let key = match step {
            Step::Tab => Key::Tab,
            Step::Enter => Key::Enter,
            Step::ArrowRight => Key::ArrowRight,
            other => panic!("an unexpected step {other:?}"),
        };
        press(&mut h, &mut app, key);
    }
    assert!(
        app.present,
        "the mock_present case's keys entered present mode"
    );
    assert_eq!(state(&app), before, "present mode edited");
    press(&mut h, &mut app, Key::Escape);
    assert!(!app.present, "Esc left present mode");
    assert_eq!(state(&app), before, "Esc from present mode edited");
    // Control: a pan, an edit, is seen by the same check.
    let z0 = app.snapshot().sim.plane.z0;
    let q1 = app.snapshot().sim.plane.q1;
    edit(&mut h, &mut app, SimField::Z0(add(&z0, 0.1, &q1)));
    rejects("a pan", || assert_eq!(state(&app), before));
}

// --- Panned and grid-aligned views -----------------------------------------------------------------------------------

/// The views the fill is checked at, besides the mock's initial one: `(u, v)`, `z₀`'s first two coordinates (the
/// chart's `q₁`, `q₂` being the mock's unit `z_α`, `z_β`), and the zoom, a factor on the basis. Grid-aligned, the
/// quads' edges on the shown rect's edges: `z₀` at the origin; and zoomed in ×2 with the view `[0, 1]²` in `(u, v)`.
/// Panned: across the root's edge, and past it.
const VIEWS: [(&str, (f64, f64), f64); 4] = [
    ("grid-aligned", (0.0, 0.0), 1.0),
    ("grid-aligned, zoomed in", (0.5, 0.5), 0.5),
    ("panned", (1.5, 0.41), 1.0),
    ("panned past the root", (-5.3, 3.7), 1.0),
];

/// The app at `view`, with the tile bounds on or off, the layer shown or hidden.
fn at_view(
    canvas: &Arc<MockCanvas>,
    (_, (u, v), zoom): (&str, (f64, f64), f64),
    bounds: bool,
) -> (App<MockSide>, Headless) {
    let (mut app, mut h) = app_on(Some(canvas.clone()));
    let plane = app.snapshot().sim.plane.clone();
    if zoom != 1.0 {
        let (q1, q2) = (plane.q1.map(|x| x * zoom), plane.q2.map(|x| x * zoom));
        edit(&mut h, &mut app, SimField::Basis { q1, q2 });
    }
    let mut z0 = plane.z0;
    (z0[0], z0[1]) = (u, v);
    edit(&mut h, &mut app, SimField::Z0(z0));
    if bounds {
        tile_bounds_on(&mut h, &mut app);
    }
    (app, h)
}

/// The capture at `view`: the layer shown, or hidden by F3.
fn shot_at(
    canvas: &Arc<MockCanvas>,
    view: (&str, (f64, f64), f64),
    bounds: bool,
    hide: bool,
) -> Shot {
    let (mut app, mut h) = at_view(canvas, view, bounds);
    if hide {
        press(&mut h, &mut app, Key::F3);
    }
    render_frame(canvas, &mut app, &mut h)
}

/// `a` and `b` are the same pixel for pixel over `rect`.
fn check_same_over(a: &[u8], b: &[u8], rect: [usize; 4], what: &str) {
    for y in rect[1]..rect[3] {
        for x in rect[0]..rect[2] {
            assert_eq!(px(a, x, y), px(b, x, y), "{what} at ({x}, {y})");
        }
    }
}

/// At grid-aligned and panned views, with the tile bounds on (REQ-GUI-179; R-406, the overlay "covers the extra quads
/// too"): over the shown rect the F3-off capture is pixel-identical to the F3-on one; F3 again restores the shown
/// capture pixel for pixel; and with F3 off the bounds cover the window, every column crossing a quad's top or bottom
/// edge and every row a quad's side. Controls: the shown capture without the bounds fails the check over the rect
/// (where the shown figure has bounds), and the bounds left in the shown rect fail the coverage check.
#[test]
fn qa_f3_off_bounds_at_panned_and_grid_aligned_views() {
    let canvas = canvas();
    let f = shown_px();
    for view in VIEWS {
        let name = view.0;
        let (mut app, mut h) = at_view(&canvas, view, true);
        let on = render_frame(&canvas, &mut app, &mut h);
        press(&mut h, &mut app, Key::F3);
        assert!(!app.shown);
        let off = render_frame(&canvas, &mut app, &mut h);
        press(&mut h, &mut app, Key::F3);
        let back = render_frame(&canvas, &mut app, &mut h);
        assert!(
            on.rgba == back.rgba,
            "{name}: F3 twice is not the shown capture"
        );
        check_same_over(
            &on.rgba,
            &off.rgba,
            f,
            &format!("{name}, F3 off against on"),
        );
        let bare_on = shot_at(&canvas, view, false, false);
        let inside_bounds = diff(&on.rgba, &bare_on.rgba);
        if (0..W * H).any(|i| inside_bounds[i] && inside(f, i % W, i / W)) {
            rejects(
                &format!("{name}, the shown capture without its bounds"),
                || check_same_over(&bare_on.rgba, &off.rgba, f, "bare"),
            );
        }
        let bounds = diff(&off.rgba, &shot_at(&canvas, view, false, true).rgba);
        let (cols, rows) = crossings(&bounds, [0, W], [0, H]);
        assert!(
            cols.is_empty() && rows.is_empty(),
            "{name}: with F3 off the tile bounds leave {} columns and {} rows bare",
            cols.len(),
            rows.len()
        );
        let left: Vec<bool> = (0..W * H)
            .map(|i| bounds[i] && inside(f, i % W, i / W))
            .collect();
        rejects(
            &format!("{name}, the bounds left in the shown rect"),
            || {
                let (cols, rows) = crossings(&left, [0, W], [0, H]);
                assert!(cols.is_empty() && rows.is_empty());
            },
        );
    }
}

/// How far, in pixels, either side of the shown rect's edge a bound line is checked unbroken.
const SEAM: usize = 4;

/// Of the bound lines (a mask, as [`diff`] gives) crossing the shown rect's edge `rect` (a line [`RUN`] pixels long on
/// each side of the edge, [`SEAM`] pixels clear of it outside): how many there are, and the ones broken in the `SEAM`
/// pixels outside the edge, each as `(side, x, y)` of its first unset pixel.
fn seams(d: &[bool], rect: [usize; 4]) -> (usize, Vec<(&'static str, usize, usize)>) {
    let at = |x: usize, y: usize| d[y * W + x];
    let (mut lines, mut broken) = (0, Vec::new());
    let mut check = |side: &'static str, inner: bool, outer: bool, gap: Vec<(usize, usize)>| {
        if inner && outer {
            lines += 1;
            if let Some(&(x, y)) = gap.iter().find(|&&(x, y)| !at(x, y)) {
                broken.push((side, x, y));
            }
        }
    };
    let [x0, y0, x1, y1] = rect;
    for y in y0..y1 {
        check(
            "left",
            (x0..x0 + RUN).all(|x| at(x, y)),
            (x0 - SEAM - RUN..x0 - SEAM).all(|x| at(x, y)),
            (x0 - SEAM..x0).map(|x| (x, y)).collect(),
        );
        check(
            "right",
            (x1 - RUN..x1).all(|x| at(x, y)),
            (x1 + SEAM..x1 + SEAM + RUN).all(|x| at(x, y)),
            (x1..x1 + SEAM).map(|x| (x, y)).collect(),
        );
    }
    for x in x0..x1 {
        check(
            "top",
            (y0..y0 + RUN).all(|y| at(x, y)),
            (y0 - SEAM - RUN..y0 - SEAM).all(|y| at(x, y)),
            (y0 - SEAM..y0).map(|y| (x, y)).collect(),
        );
        check(
            "bottom",
            (y1 - RUN..y1).all(|y| at(x, y)),
            (y1 + SEAM..y1 + SEAM + RUN).all(|y| at(x, y)),
            (y1..y1 + SEAM).map(|y| (x, y)).collect(),
        );
    }
    (lines, broken)
}

/// With F3 off the figure fills the window, the shown rect no longer an edge of it (R-406: "the rest of the viewport just
/// becomes the entire window"; the overlay "covers the extra quads too"): a tile bound running across where the shown
/// rect's edge was, inside it and past it, is drawn unbroken across that edge, so the hidden layout's rect leaves no
/// seam in the bounds. Checked at the grid-aligned and panned views, where such lines cross each side. Controls: a
/// mask with one pixel cleared beside the edge on a crossing line fails; the same mask with each crossing line filled
/// across the edge passes, so the check can pass.
#[test]
fn qa_f3_off_bounds_unbroken_across_the_shown_rect_edge() {
    let canvas = canvas();
    let f = shown_px();
    let mut seen = 0;
    let mut failures = Vec::new();
    for view in VIEWS {
        let bounds = diff(
            &shot_at(&canvas, view, true, true).rgba,
            &shot_at(&canvas, view, false, true).rgba,
        );
        let (lines, broken) = seams(&bounds, f);
        seen += lines;
        if !broken.is_empty() {
            failures.push(format!(
                "{}: {} of {lines} lines broken, e.g. {:?}",
                view.0,
                broken.len(),
                &broken[..broken.len().min(4)]
            ));
        }
        if lines > 0 {
            // Controls on this view's mask: every crossing line filled across the edge passes; one pixel cleared fails.
            let mut filled = bounds.clone();
            let mut fill = |xs: std::ops::Range<usize>, ys: std::ops::Range<usize>| {
                for y in ys {
                    for x in xs.clone() {
                        filled[y * W + x] = true;
                    }
                }
            };
            for (side, x, y) in &broken {
                match *side {
                    "left" => fill(f[0] - SEAM..f[0], *y..*y + 1),
                    "right" => fill(f[2]..f[2] + SEAM, *y..*y + 1),
                    "top" => fill(*x..*x + 1, f[1] - SEAM..f[1]),
                    _ => fill(*x..*x + 1, f[3]..f[3] + SEAM),
                }
            }
            let (n, still) = seams(&filled, f);
            assert!(
                n == lines && still.is_empty(),
                "{}: the filled control",
                view.0
            );
            let y =
                (f[1]..f[3]).find(|&y| (f[0] - SEAM - RUN..f[0] + RUN).all(|x| filled[y * W + x]));
            if let Some(y) = y {
                filled[y * W + f[0] - 1] = false;
                rejects(
                    &format!("{}, a pixel cleared beside the edge", view.0),
                    || assert!(seams(&filled, f).1.is_empty()),
                );
            }
        }
    }
    assert!(
        seen > 0,
        "no bound line crosses the shown rect's edge at any view"
    );
    assert!(
        failures.is_empty(),
        "with F3 off the tile bounds break where the shown rect's edge was:\n{}",
        failures.join("\n")
    );
}
