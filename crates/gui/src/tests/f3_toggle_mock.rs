//! `f3_toggle_mock` (REQ-GUI-168; RQ-248): F3 hides and shows the egui layer over the stand-in figure, which stays
//! identical underneath: the captures with F3 on and off are pixel-identical over the figure's rect, and with F3 off
//! the rest of the window is the clear colour. A raised warning and error change the footer's counts and nothing over
//! the figure; a footer click opens the console window frame. These run on the GPU, on `PRIN_GPU_BACKEND`'s backend
//! (R-206), which the mock's canvas logs.

use std::sync::Arc;

use eframe::egui;

use super::support::{rejects, texts};
use crate::app::clear_colour;
use crate::capture::{shoot, Shot, Step, PIXELS_PER_POINT, SIZE};
use crate::explore::footer::MOCK_TAG;
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::canvas::MockCanvas;
use crate::mock::{MOCK_ERROR, MOCK_WARNING};

fn canvas() -> Arc<MockCanvas> {
    Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"))
}

/// The figure's rect in the capture, in whole pixels: `[x0, y0, x1, y1)`.
fn figure_px() -> [usize; 4] {
    let screen = Headless::new(SIZE, PIXELS_PER_POINT).screen();
    let f = Layout::new(screen, PIXELS_PER_POINT).figure;
    [f.min.x, f.min.y, f.max.x, f.max.y].map(|v| (v * PIXELS_PER_POINT).round() as usize)
}

fn pixel(shot: &Shot, x: usize, y: usize) -> [u8; 4] {
    let i = (y * shot.size[0] as usize + x) * 4;
    shot.rgba[i..i + 4].try_into().unwrap()
}

fn inside(rect: [usize; 4], x: usize, y: usize) -> bool {
    (rect[0]..rect[2]).contains(&x) && (rect[1]..rect[3]).contains(&y)
}

/// The two captures are identical over `rect`.
fn check_same_figure(a: &Shot, b: &Shot, rect: [usize; 4]) {
    for y in rect[1]..rect[3] {
        for x in rect[0]..rect[2] {
            assert_eq!(
                pixel(a, x, y),
                pixel(b, x, y),
                "the figure differs at ({x}, {y})"
            );
        }
    }
}

/// Every pixel of `shot` outside `rect` is `clear`.
fn check_clear_outside(shot: &Shot, rect: [usize; 4], clear: [u8; 4]) {
    let [w, h] = shot.size.map(|v| v as usize);
    for y in 0..h {
        for x in 0..w {
            if !inside(rect, x, y) {
                assert_eq!(
                    pixel(shot, x, y),
                    clear,
                    "drawn outside the figure at ({x}, {y}) with F3 off"
                );
            }
        }
    }
}

#[test]
fn f3_toggle_mock_figure_identical_underneath() {
    let canvas = canvas();
    let on = shoot(canvas.clone(), &[]);
    let off = shoot(canvas.clone(), &[Step::F3]);
    let back = shoot(canvas, &[Step::F3, Step::F3]);
    let rect = figure_px();
    assert_eq!(
        rect,
        [540, 43, 1596, 1020],
        "the figure's rect is 01_main.png's"
    );
    check_same_figure(&on, &off, rect);
    let clear = clear_colour(&egui::Visuals::dark()).to_array();
    check_clear_outside(&off, rect, clear);
    assert_eq!(on.rgba, back.rgba, "F3 twice is not the shell again");
    // The figure is drawn: it is not the clear colour, and not uniform.
    let centre = pixel(&on, (rect[0] + rect[2]) / 2, (rect[1] + rect[3]) / 2);
    let corner = pixel(&on, rect[0] + 10, rect[1] + 10);
    assert!(centre != clear || corner != clear, "no figure drawn");
    assert!(
        (rect[0]..rect[2]).any(|x| pixel(&on, x, 500) != centre),
        "a uniform figure"
    );
    // With F3 off the layer's names are gone; with it on the shell's are there.
    assert!(
        texts(&off.names).is_empty(),
        "names with F3 off: {:?}",
        texts(&off.names)
    );
    assert!(texts(&on.names).iter().any(|t| t == MOCK_TAG));
    rejects(
        "the shell with F3 on, which draws outside the figure",
        || check_clear_outside(&on, rect, clear),
    );
    rejects("a figure rect shifted by a pixel", || {
        check_same_figure(&on, &off, [rect[0] - 1, rect[1], rect[2], rect[3]]);
        check_clear_outside(&off, [rect[0] + 1, rect[1], rect[2], rect[3]], clear);
    });
}

#[test]
fn f3_toggle_mock_warning_and_error_reach_the_footer_only() {
    let canvas = canvas();
    let shell = shoot(canvas.clone(), &[]);
    let raised = shoot(canvas.clone(), &[Step::RaiseWarning, Step::RaiseError]);
    let rect = figure_px();
    check_same_figure(&shell, &raised, rect);
    let raised_texts = texts(&raised.names);
    assert!(
        raised_texts.iter().any(|t| t == "⚠ 1 · × 1 · console ▴"),
        "{raised_texts:?}"
    );
    assert!(
        raised_texts.iter().any(|t| t == MOCK_ERROR),
        "the latest message"
    );
    assert!(texts(&shell.names)
        .iter()
        .any(|t| t == "⚠ 0 · × 0 · console ▴"));
    // A footer click opens the console window frame, its rows the entries; it covers no part of the figure.
    let console = shoot(
        canvas,
        &[Step::RaiseWarning, Step::RaiseError, Step::ClickFooter],
    );
    let console_texts = texts(&console.names);
    for name in [
        "Console",
        "warn",
        "error",
        "stain",
        "integrator",
        MOCK_WARNING,
        "12:03:59.410",
    ] {
        assert!(
            console_texts.iter().any(|t| t == name),
            "no `{name}` in {console_texts:?}"
        );
    }
    assert!(console_texts.iter().any(|t| t == "⚠ 1 · × 1 · console ▾"));
    check_same_figure(&shell, &console, rect);
    rejects("the shell's names, with no console", || {
        assert!(
            texts(&shell.names).iter().any(|t| t == "Console"),
            "no console"
        );
    });
}

/// The canvas draws on targets of either format, its pipeline built for each.
#[test]
fn f3_toggle_mock_canvas_draws_on_each_format() {
    use engine::contract::canvas::Canvas;
    let canvas = canvas();
    let device = &canvas.context().device;
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8Unorm,
    ] {
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
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
            canvas.draw_figure(&mut pass, format);
        }
        canvas.context().queue.submit([encoder.finish()]);
        let error = pollster::block_on(scope.pop());
        assert!(error.is_none(), "drawing on {format:?}: {error:?}");
    }
}

/// The regions tile the window at any size and scale, each edge on a whole pixel, so the figure keeps one pixel rect
/// whether the layer is shown or not.
#[test]
fn f3_toggle_mock_layout_tiles_the_window() {
    for (size, ppp) in [
        ([2160.0, 1350.0], 1.5),
        ([1440.0, 900.0], 2.0),
        ([1000.0, 700.0], 1.0),
    ] {
        let screen = egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(size[0] / ppp, size[1] / ppp),
        );
        let l = Layout::new(screen, ppp);
        let area = |r: egui::Rect| r.area();
        let parts = [
            l.top_bar,
            l.manifold_view,
            l.figure,
            l.trajectory,
            l.compass,
            l.time,
            l.legend,
            l.footer,
        ];
        let total: f32 = parts.iter().map(|r| area(*r)).sum();
        assert!(
            (total - area(screen)).abs() < 1.0,
            "the regions do not tile {size:?}"
        );
        assert_eq!(l.bottom_row.min, l.compass.min);
        assert_eq!(l.bottom_row.max, l.legend.max);
        assert_eq!(
            (l.page.min.y, l.page.max.y),
            (l.top_bar.max.y, l.footer.min.y)
        );
        for r in parts {
            for v in [r.min.x, r.min.y, r.max.x, r.max.y] {
                let px = v * ppp;
                assert!(
                    (px - px.round()).abs() < 1e-3,
                    "an edge off the pixel grid: {px}"
                );
            }
        }
        // 01_main.png's proportions: the figure starts a quarter of the way across.
        assert!((l.figure.min.x / screen.width() - 0.25).abs() < 0.01);
    }
}
