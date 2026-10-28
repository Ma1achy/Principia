//! The Explore page's regions (render_gui_spec §G2; 01_main.png): the top bar, the left Manifold view, the figure,
//! the right Trajectory panel, the bottom row (compass, Time, Legend) and the footer. Each edge is rounded to a whole
//! physical pixel. The top bar and the footer keep 01_main.png's proportions of the window; every panel boundary is a
//! splitter ([`Splitter`], R-462 J1), at 01_main.png's proportions by default ([`Splits`]), a drag holding each panel
//! and the figure's canvas to a minimum size (J2), the canvas taking the space the panels leave. With the egui layer hidden (F3)
//! the figure's rect is the whole window. The field keeps the place the default layout gives it, whatever the
//! splitters and the layer (R-406; R-462 J4): the canvas is given that placement, [`Placement`], so it draws each point
//! of the field where the default layout puts it, more of the field around it where the canvas is larger, less where
//! it is smaller, never at another scale. The figure's axis labels take a strip along the bottom of the figure's box
//! and one down its left side, so they sit beside the figure and never over it (render_gui_spec §G1, §G2).

use eframe::egui::{self, pos2, vec2, CursorIcon, Id, Rect, Sense, Ui};

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

/// The Manifold view's least width, in points: its widest row, Centre z₀'s slider and number box, whole (R-462 J2;
/// measured at the capture's size: the number box passes the panel's edge below 340).
pub const MIN_MANIFOLD_W: f32 = 340.0;
/// The Trajectory panel's least width, in points: its views' buttons and checkboxes whole (J2; measured: "follow
/// cursor" passes the edge below 246).
pub const MIN_TRAJECTORY_W: f32 = 250.0;
/// The top row's least height, in points: the Manifold view's 28 rows each at least egui's interact height, 18 points,
/// below its title and margins, `28 × 18 + 40 + 3 × 6 + 2 × 12 = 586` (J2).
pub const MIN_TOP_H: f32 = 590.0;
/// The figure's canvas's least width, in points, its axis strip apart: so no splitter can close it (J2; a look choice,
/// R-390). Its height is held by [`MIN_TOP_H`].
pub const MIN_CANVAS_W: f32 = 240.0;
/// The bottom row's least height, in points: the compass's cube, buttons and readout, and Time's transport, whole (J2;
/// measured: the compass's readout passes the footer below 154.7).
pub const MIN_BOTTOM_H: f32 = 160.0;
/// The compass's least width, in points: its cube and its buttons beside it, and its readout (J2; measured: the readout
/// passes the edge below 224, a tilted readout a few characters longer).
pub const MIN_COMPASS_W: f32 = 240.0;
/// Time's least width, in points: its transport row, the speed slider and its value, whole (J2; measured: the speed's
/// value passes the edge below 336).
pub const MIN_TIME_W: f32 = 340.0;
/// The Legend's least width, in points: a key's swatch, label and share on one row (J2; a look choice, R-390: its rows
/// elide rather than overflow).
pub const MIN_LEGEND_W: f32 = 240.0;

/// The width of a splitter's handle, centred on its boundary, in points (a look choice, R-390).
pub const HANDLE: f32 = 6.0;

/// A panel boundary the pointer drags (R-462 J1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Splitter {
    /// Between the Manifold view and the figure.
    ManifoldFigure,
    /// Between the figure and the Trajectory panel.
    FigureTrajectory,
    /// Between the top row (the Manifold view, the figure, the Trajectory panel) and the bottom row.
    TopBottom,
    /// Between the compass and Time.
    CompassTime,
    /// Between Time and the Legend.
    TimeLegend,
}

impl Splitter {
    /// Every splitter.
    pub const ALL: [Splitter; 5] = [
        Splitter::ManifoldFigure,
        Splitter::FigureTrajectory,
        Splitter::TopBottom,
        Splitter::CompassTime,
        Splitter::TimeLegend,
    ];

    /// Its name in the GUI's settings.
    pub fn name(self) -> &'static str {
        match self {
            Splitter::ManifoldFigure => "manifold_figure",
            Splitter::FigureTrajectory => "figure_trajectory",
            Splitter::TopBottom => "top_bottom",
            Splitter::CompassTime => "compass_time",
            Splitter::TimeLegend => "time_legend",
        }
    }

    /// Whether it is dragged across, a vertical boundary; else down, the top row's bottom.
    pub fn across(self) -> bool {
        self != Splitter::TopBottom
    }

    fn index(self) -> usize {
        self as usize
    }

    /// Its default: 01_main.png's edge, as a fraction of the artboard's width, or height.
    fn default_fraction(self) -> f32 {
        match self {
            Splitter::ManifoldFigure => LEFT_RIGHT / ARTBOARD[0],
            Splitter::FigureTrajectory => RIGHT_LEFT / ARTBOARD[0],
            Splitter::TopBottom => BOTTOM_TOP / ARTBOARD[1],
            Splitter::CompassTime => COMPASS_RIGHT / ARTBOARD[0],
            Splitter::TimeLegend => LEGEND_LEFT / ARTBOARD[0],
        }
    }

    /// The id of its handle's widget.
    pub fn id(self) -> Id {
        Id::new(("splitter", self.name()))
    }

    /// The splitter whose handle has the id `id`, if any.
    pub fn of_id(id: Id) -> Option<Splitter> {
        Splitter::ALL.into_iter().find(|s| s.id() == id)
    }
}

/// Where each splitter is, as a fraction of the window's width, or its height for [`Splitter::TopBottom`]: so a
/// layout keeps its proportions as the window is resized. The GUI's own state, kept across sessions in its settings
/// (R-462 J5; `crate::settings`); never the contract's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Splits([f32; 5]);

impl Default for Splits {
    /// 01_main.png's layout.
    fn default() -> Self {
        Self(Splitter::ALL.map(Splitter::default_fraction))
    }
}

impl Splits {
    /// Where `splitter` is.
    pub fn get(&self, splitter: Splitter) -> f32 {
        self.0[splitter.index()]
    }

    /// Puts `splitter` at `fraction`.
    pub fn set(&mut self, splitter: Splitter, fraction: f32) {
        self.0[splitter.index()] = fraction;
    }

    /// Returns `splitter`, and only it, to its default (R-462 J3).
    pub fn reset(&mut self, splitter: Splitter) {
        self.set(splitter, splitter.default_fraction());
    }
}

/// The edges a layout's rects are built from, in points, each on a whole physical pixel.
#[derive(Clone, Copy, Debug)]
struct Edges {
    top: f32,
    footer: f32,
    left: f32,
    right: f32,
    bottom: f32,
    compass: f32,
    legend: f32,
}

/// `v` held to `[lo, hi]`, `hi` winning where they cross.
fn between(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi)
}

impl Edges {
    fn of(screen: Rect, pixels_per_point: f32, splits: &Splits) -> Self {
        let at = |fraction: f32, min: f32, size: f32| {
            min + (fraction * size * pixels_per_point).round() / pixels_per_point
        };
        let x = |s: Splitter| at(splits.get(s), screen.min.x, screen.width());
        let y = |f: f32| at(f, screen.min.y, screen.height());
        Self {
            top: y(TOP_BAR_BOTTOM / ARTBOARD[1]),
            footer: y(FOOTER_TOP / ARTBOARD[1]),
            left: x(Splitter::ManifoldFigure),
            right: x(Splitter::FigureTrajectory),
            bottom: y(splits.get(Splitter::TopBottom)),
            compass: x(Splitter::CompassTime),
            legend: x(Splitter::TimeLegend),
        }
    }
}

/// The regions of a window, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    /// The top bar.
    pub top_bar: Rect,
    /// The left panel, "Manifold view".
    pub manifold_view: Rect,
    /// The figure's rect: its canvas, where it is drawn with the layer shown and its gestures land.
    pub figure: Rect,
    /// Where the field's `[0, 1]²` is: the default layout's figure rect (R-406; R-462 J4). The figure's points, its
    /// gestures' and its marks', are read in it, so no splitter moves or scales the field.
    pub field: Rect,
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
    /// The whole window.
    pub window: Rect,
}

impl Layout {
    /// The regions of a window `screen` points in size at `pixels_per_point`, in the default layout: 01_main.png's.
    pub fn new(screen: Rect, pixels_per_point: f32) -> Self {
        Self::with_splits(screen, pixels_per_point, &Splits::default())
    }

    /// The regions of a window `screen` points in size at `pixels_per_point`, its splitters at `splits`: each panel
    /// keeps its share of the window as the window is resized, as 01_main.png's proportions always have; the minimums
    /// hold the splitters' drags ([`Layout::range`]).
    pub fn with_splits(screen: Rect, pixels_per_point: f32, splits: &Splits) -> Self {
        let e = Edges::of(screen, pixels_per_point, splits);
        let d = Edges::of(screen, pixels_per_point, &Splits::default());
        let snap = |v: f32| (v * pixels_per_point).round() / pixels_per_point;
        let (x0, x1) = (screen.min.x, screen.max.x);
        let (y0, y1) = (screen.min.y, screen.max.y);
        let rect = |a: f32, b: f32, c: f32, d: f32| Rect::from_min_max(pos2(a, b), pos2(c, d));
        let (axis_w, axis_h) = (snap(AXIS_Y_W), snap(AXIS_X_H));
        let (inner_left, inner_bottom) = (e.left + axis_w, e.bottom - axis_h);
        Self {
            top_bar: rect(x0, y0, x1, e.top),
            manifold_view: rect(x0, e.top, e.left, e.bottom),
            figure: rect(inner_left, e.top, e.right, inner_bottom),
            field: rect(d.left + axis_w, d.top, d.right, d.bottom - axis_h),
            axis_x: rect(e.left, inner_bottom, e.right, e.bottom),
            axis_y: rect(e.left, e.top, inner_left, inner_bottom),
            trajectory: rect(e.right, e.top, x1, e.bottom),
            compass: rect(x0, e.bottom, e.compass, e.footer),
            time: rect(e.compass, e.bottom, e.legend, e.footer),
            legend: rect(e.legend, e.bottom, x1, e.footer),
            footer: rect(x0, e.footer, x1, y1),
            bottom_row: rect(x0, e.bottom, x1, e.footer),
            page: rect(x0, e.top, x1, e.footer),
            window: screen,
        }
    }

    /// The figure's rect as it stands: [`Layout::figure`] with the egui layer shown, the whole window with it hidden
    /// (F3) or in present mode, which hides the chrome as F3 does (R-406).
    pub fn figure_rect(&self, shown: bool) -> Rect {
        if shown {
            self.figure
        } else {
            self.window
        }
    }

    /// Where the field is in the window: [`Layout::field`]'s placement in [`Layout::window`], whatever the splitters
    /// (R-406; R-462 J4).
    pub fn placement(&self) -> Placement {
        Placement::of(self.field, self.window)
    }

    /// The span of the field the figure shows, `[s₀, t₀, s₁, t₁]`, `t` Y-up: the figure's rect as it stands
    /// ([`Layout::figure_rect`]) read in [`Layout::field`], `[0, 0, 1, 1]` exactly in the default layout shown.
    pub fn span(&self, shown: bool) -> [f64; 4] {
        span_of(self.figure_rect(shown), self.field)
    }

    /// Where `splitter` is, in points: its boundary's `x`, or `y` for the top row's bottom.
    pub fn edge(&self, splitter: Splitter) -> f32 {
        match splitter {
            Splitter::ManifoldFigure => self.manifold_view.max.x,
            Splitter::FigureTrajectory => self.trajectory.min.x,
            Splitter::TopBottom => self.bottom_row.min.y,
            Splitter::CompassTime => self.compass.max.x,
            Splitter::TimeLegend => self.legend.min.x,
        }
    }

    /// How far `splitter` can be dragged, in points, with the other splitters where they are: as far as keeps the
    /// panels on each side of it at their minimums, the canvas among them (R-462 J2).
    pub fn range(&self, splitter: Splitter) -> (f32, f32) {
        let w = self.window;
        let figure_w = MIN_CANVAS_W + self.axis_y.width();
        match splitter {
            Splitter::ManifoldFigure => (
                w.min.x + MIN_MANIFOLD_W,
                self.edge(Splitter::FigureTrajectory) - figure_w,
            ),
            Splitter::FigureTrajectory => (
                self.edge(Splitter::ManifoldFigure) + figure_w,
                w.max.x - MIN_TRAJECTORY_W,
            ),
            Splitter::TopBottom => (
                self.top_bar.max.y + MIN_TOP_H,
                self.footer.min.y - MIN_BOTTOM_H,
            ),
            Splitter::CompassTime => (
                w.min.x + MIN_COMPASS_W,
                self.edge(Splitter::TimeLegend) - MIN_TIME_W,
            ),
            Splitter::TimeLegend => (
                self.edge(Splitter::CompassTime) + MIN_TIME_W,
                w.max.x - MIN_LEGEND_W,
            ),
        }
    }

    /// `splitter` dragged from `start`, its edge when the drag began, by `delta` points along it: the fraction of the
    /// window it then stands at, the edge held to [`Layout::range`].
    pub fn dragged(&self, splitter: Splitter, start: f32, delta: f32) -> f32 {
        let (lo, hi) = self.range(splitter);
        let edge = between(start + delta, lo, hi);
        let (min, size) = if splitter.across() {
            (self.window.min.x, self.window.width())
        } else {
            (self.window.min.y, self.window.height())
        };
        (edge - min) / size
    }

    /// `splitter`'s handle: a strip [`HANDLE`] wide centred on its boundary, along the boundary's length.
    pub fn handle(&self, splitter: Splitter) -> Rect {
        let half = HANDLE / 2.0;
        let edge = self.edge(splitter);
        let (top, bottom) = (self.top_bar.max.y, self.bottom_row.min.y);
        let footer = self.footer.min.y;
        match splitter {
            Splitter::ManifoldFigure | Splitter::FigureTrajectory => {
                Rect::from_min_max(pos2(edge - half, top), pos2(edge + half, bottom - half))
            }
            Splitter::TopBottom => Rect::from_min_max(
                pos2(self.window.min.x, edge - half),
                pos2(self.window.max.x, edge + half),
            ),
            Splitter::CompassTime | Splitter::TimeLegend => {
                Rect::from_min_max(pos2(edge - half, bottom + half), pos2(edge + half, footer))
            }
        }
    }

    /// The splitter whose handle holds `point`, if any.
    pub fn splitter_at(&self, point: egui::Pos2) -> Option<Splitter> {
        Splitter::ALL
            .into_iter()
            .find(|s| self.handle(*s).contains(point))
    }
}

/// The span of `rect` in `field`'s `[0, 1]²`, `[s₀, t₀, s₁, t₁]`, `t` Y-up, in `f64`: `[0, 0, 1, 1]` exactly for the
/// field itself.
pub fn span_of(rect: Rect, field: Rect) -> [f64; 4] {
    let s = |x: f32| f64::from(x - field.min.x) / f64::from(field.width());
    let t = |y: f32| f64::from(field.max.y - y) / f64::from(field.height());
    [s(rect.min.x), t(rect.max.y), s(rect.max.x), t(rect.min.y)]
}

/// Lays the splitters' handles on `layout`, after the panels, so they take the pointer at the boundaries: each shows a
/// resize cursor while hovered or dragged (R-462 J1). Returns the splitters double-clicked this frame, each to be
/// reset (J3). The drag itself is read as the frame begins ([`held`]), so the layout is drawn where the drag puts it.
pub fn splitters(ui: &mut Ui, layout: &Layout) -> Vec<Splitter> {
    let mut reset = Vec::new();
    for splitter in Splitter::ALL {
        // Not focusable: a splitter is a pointer control, outside the keyboard's tree and AccessKit's (J6).
        let response = ui.interact(
            layout.handle(splitter),
            splitter.id(),
            Sense::CLICK | Sense::DRAG,
        );
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(if splitter.across() {
                CursorIcon::ResizeHorizontal
            } else {
                CursorIcon::ResizeVertical
            });
        }
        if response.double_clicked() {
            reset.push(splitter);
        }
    }
    reset
}

/// The splitter a drag holds this frame, if any, and how far the pointer has moved since it was pressed, along the
/// splitter. egui decides the drag as the frame begins, so it is read before anything is drawn.
pub fn held(ctx: &egui::Context) -> Option<(Splitter, f32)> {
    let splitter = Splitter::of_id(ctx.dragged_id()?)?;
    let delta = ctx
        .input(|i| i.pointer.total_drag_delta())
        .unwrap_or(vec2(0.0, 0.0));
    Some((splitter, if splitter.across() { delta.x } else { delta.y }))
}

/// Where the field is drawn in a canvas's viewport (R-406): the shown figure's rect, `[0,1]²` of the field, as its
/// centre and half-size in the viewport's normalized device coordinates, Y-up. A field point `(x, y)`, each in
/// `[−1, 1]` over the shown rect (`x = 2s − 1`, `y = 2t − 1`), is at `centre + (x, y) · half` in the viewport; past
/// the shown rect the field continues at the same scale. [`Placement::IDENTITY`] is the field over the whole viewport,
/// as the Chart builder's quick render and the Stain editor's preview draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// The shown rect's centre, in the viewport's normalized device coordinates, Y-up.
    pub centre: [f32; 2],
    /// The shown rect's half-size, in the same coordinates: 1 where it spans the viewport.
    pub half: [f32; 2],
}

impl Placement {
    /// The field over the whole viewport.
    pub const IDENTITY: Placement = Placement {
        centre: [0.0, 0.0],
        half: [1.0, 1.0],
    };

    /// The placement of `shown` in a viewport over `drawn`, both in points.
    pub fn of(shown: Rect, drawn: Rect) -> Self {
        let ndc = |v: f32, min: f32, size: f32| (v - min) / size * 2.0 - 1.0;
        Self {
            centre: [
                ndc(shown.center().x, drawn.min.x, drawn.width()),
                -ndc(shown.center().y, drawn.min.y, drawn.height()),
            ],
            half: [
                shown.width() / drawn.width(),
                shown.height() / drawn.height(),
            ],
        }
    }

    /// The viewport's span of the field, `[s₀, t₀, s₁, t₁]`, `t` Y-up: `[0, 0, 1, 1]` for the identity, wider for a
    /// viewport larger than the shown rect.
    pub fn span(&self) -> [f64; 4] {
        let edge = |ndc: f64, centre: f32, half: f32| {
            ((ndc - f64::from(centre)) / f64::from(half) + 1.0) / 2.0
        };
        [
            edge(-1.0, self.centre[0], self.half[0]),
            edge(-1.0, self.centre[1], self.half[1]),
            edge(1.0, self.centre[0], self.half[0]),
            edge(1.0, self.centre[1], self.half[1]),
        ]
    }
}
