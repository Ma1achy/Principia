//! The Explore page's regions (render_gui_spec §G2; 01_main.png): the top bar, the left Manifold view, the figure,
//! the right Trajectory panel, the bottom row (compass, Time, Legend) and the footer, in 01_main.png's proportions of
//! the window. Each edge is rounded to a whole physical pixel, so the figure's rect is the same pixel rect whether the
//! egui layer is shown or hidden (F3; RQ-248). The figure's axis labels take a strip along the bottom of 01_main.png's
//! figure box and one down its left side, so they sit beside the figure and never over it (render_gui_spec §G1, §G2).

use eframe::egui::{pos2, Rect};

/// The artboard 01_main.png's size, in pixels.
pub const ARTBOARD: [f32; 2] = [2160.0, 1350.0];

/// 01_main.png's edges, in its pixels: the top bar's bottom, the left panel's right edge, the right panel's left edge,
/// the bottom row's top, the footer's top, and the bottom row's two inner edges (compass | Time, Time | Legend).
const TOP_BAR_BOTTOM: f32 = 43.0;
const LEFT_RIGHT: f32 = 540.0;
const RIGHT_LEFT: f32 = 1596.0;
const BOTTOM_TOP: f32 = 1020.0;
const FOOTER_TOP: f32 = 1315.0;
const COMPASS_RIGHT: f32 = 531.0;
const LEGEND_LEFT: f32 = 1463.0;

/// The height of the horizontal axis labels' strip under the figure, in points (a look choice, R-390).
pub const AXIS_X_H: f32 = 16.0;
/// The width of the vertical axis labels' strip left of the figure, in points (a look choice, R-390).
pub const AXIS_Y_W: f32 = 16.0;

/// The regions of a window, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    /// The top bar.
    pub top_bar: Rect,
    /// The left panel, "Manifold view".
    pub manifold_view: Rect,
    /// The figure's rect.
    pub figure: Rect,
    /// The horizontal axis labels' strip, under the figure and the vertical strip.
    pub axis_x: Rect,
    /// The vertical axis labels' strip, left of the figure.
    pub axis_y: Rect,
    /// The right panel, "Trajectory".
    pub trajectory: Rect,
    /// The bottom row's compass.
    pub compass: Rect,
    /// The bottom row's Time.
    pub time: Rect,
    /// The bottom row's Legend.
    pub legend: Rect,
    /// The footer.
    pub footer: Rect,
    /// The whole bottom row, where the console opens, above the footer (12_console.png).
    pub bottom_row: Rect,
    /// Everything between the top bar and the footer: the Stain page's frame.
    pub page: Rect,
}

impl Layout {
    /// The regions of a window `screen` points in size at `pixels_per_point`.
    pub fn new(screen: Rect, pixels_per_point: f32) -> Self {
        let px = |artboard_edge: f32, artboard_size: f32, min: f32, size: f32| {
            let edge = (artboard_edge / artboard_size * size * pixels_per_point).round();
            min + edge / pixels_per_point
        };
        let x = |edge| px(edge, ARTBOARD[0], screen.min.x, screen.width());
        let y = |edge| px(edge, ARTBOARD[1], screen.min.y, screen.height());
        let (x0, x1) = (screen.min.x, screen.max.x);
        let (y0, y1) = (screen.min.y, screen.max.y);
        let (top, bottom, footer) = (y(TOP_BAR_BOTTOM), y(BOTTOM_TOP), y(FOOTER_TOP));
        let (left, right) = (x(LEFT_RIGHT), x(RIGHT_LEFT));
        let (compass, legend) = (x(COMPASS_RIGHT), x(LEGEND_LEFT));
        let rect = |a: f32, b: f32, c: f32, d: f32| Rect::from_min_max(pos2(a, b), pos2(c, d));
        let snap = |v: f32| (v * pixels_per_point).round() / pixels_per_point;
        let (inner_left, inner_bottom) = (left + snap(AXIS_Y_W), bottom - snap(AXIS_X_H));
        Self {
            top_bar: rect(x0, y0, x1, top),
            manifold_view: rect(x0, top, left, bottom),
            figure: rect(inner_left, top, right, inner_bottom),
            axis_x: rect(left, inner_bottom, right, bottom),
            axis_y: rect(left, top, inner_left, inner_bottom),
            trajectory: rect(right, top, x1, bottom),
            compass: rect(x0, bottom, compass, footer),
            time: rect(compass, bottom, legend, footer),
            legend: rect(legend, bottom, x1, footer),
            footer: rect(x0, footer, x1, y1),
            bottom_row: rect(x0, bottom, x1, footer),
            page: rect(x0, top, x1, footer),
        }
    }
}
