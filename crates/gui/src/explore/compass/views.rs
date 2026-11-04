//! The compass's views (render_gui_spec § "The compass: look and views (R-451, R-452)", §G3; R-452): four buttons on
//! the compass, Cube (the default), Plücker, Jacobi and Shape, which set what it draws, independent of its Tilt and
//! Slice modes, which set what a drag on the Cube does. The view is the compass's own GUI state, not undoable and not
//! in the contract (C1); in a grid view a drag does nothing.
//!
//! The Plücker view draws the slice plane's heading in the latent chart: the Plücker coordinates
//! `p_ij = q₁ᵢ q₂ⱼ − q₁ⱼ q₂ᵢ` (i < j) of the `q₁`, `q₂` the decoder uses, orthonormalised by Gram–Schmidt, never of a
//! reconstructed tilt ([`plucker`]), as the upper triangle of an 8 × 8 grid labelled with the chart's coordinate names
//! on both edges, the diagonal empty; each cell coloured by its sign in one of two dark-mode hues, its intensity `|p|`,
//! zero the cell background, with no number in a cell and its value on hover, as egui's tooltip beside the compass
//! (C2); the dominant cell outlined when its `|p|` exceeds 0.02, and the readout naming it and `Σ p²` (C3). Under it
//! the position readout draws `z₀` as one signed bar per latent coordinate, the slice axis highlighted: a readout, taking
//! no input. A click on cell (i, j) steers the slice to the coordinate plane `z_i × z_j`, the tilts and `γ` reset to 0,
//! the zoom and `z₀` kept: one `SetField` on the basis, turned about the pin while locked, as every basis edit is (C5).
//!
//! From the keyboard, in a grid view's interaction mode the arrows move a cell cursor over the upper triangle,
//! starting on the dominant cell, Space steering to it as a click does (C6). The Jacobi and Shape views draw the same
//! grid over physical coordinates ([`super::physical`]; R-550).

use eframe::egui::{self, pos2, vec2, Color32, Painter, Pos2, Rect, Stroke};
use engine::contract::set_field::SetField;
use engine::contract::sim_config::{Latent, Plane, SimConfig, LATENT_DIM};

use super::physical::Set;
use crate::explore::manifold_view::{
    axis, basis_edit, dot, norm, plane_edit, spans_no_plane, View, AXES, Z0_HALF_RANGE,
};
use crate::explore::{COMPASS_CUBE, COMPASS_JACOBI, COMPASS_PLUCKER, COMPASS_SHAPE};
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::ScopeId;

/// The compass's view (R-452 C1): what it draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompassView {
    /// The cube (R-451), the default.
    #[default]
    Cube,
    /// The slice plane's Plücker coordinates in the latent chart.
    Plucker,
    /// The plane pushed into Jacobi coordinates (R-550).
    Jacobi,
    /// The plane pushed into shape-sphere coordinates (R-550).
    Shape,
}

/// The four views in their buttons' order, each with its button's scope and label (R-452 C1; a look choice, R-390).
pub const VIEWS: [(CompassView, ScopeId, &str); 4] = [
    (CompassView::Cube, COMPASS_CUBE, "Cube"),
    (CompassView::Plucker, COMPASS_PLUCKER, "Plücker"),
    (CompassView::Jacobi, COMPASS_JACOBI, "Jacobi"),
    (CompassView::Shape, COMPASS_SHAPE, "Shape"),
];

/// The Plücker view's help text, under the compass's buttons (a look choice, R-390): it shows heading, not position, and
/// does not change as the slice moves.
pub const PLUCKER_HELP: &str =
    "the plane's heading in the latent chart; unchanged as the slice moves. Click a cell to steer";

impl CompassView {
    /// The view whose button is scope `id`.
    pub fn of_button(id: &str) -> Option<Self> {
        VIEWS.iter().find(|(_, s, _)| *s == id).map(|(v, _, _)| *v)
    }

    /// Its button's label.
    pub fn label(self) -> &'static str {
        VIEWS
            .iter()
            .find(|(v, _, _)| *v == self)
            .map(|(_, _, l)| *l)
            .expect("every view has a button")
    }

    /// The physical coordinate set this view draws, if it is a physical view.
    pub fn physical(self) -> Option<Set> {
        match self {
            CompassView::Jacobi => Some(Set::Jacobi),
            CompassView::Shape => Some(Set::Shape),
            CompassView::Cube | CompassView::Plucker => None,
        }
    }

    /// The compass's name in the breadcrumb in this view: "compass" in the Cube view, the view's name in a grid view
    /// ("Compass › Plücker", R-452 C6).
    pub fn breadcrumb(self) -> &'static str {
        match self {
            CompassView::Cube => "compass",
            view => view.label(),
        }
    }
}

// --- the Plücker coordinates -----------------------------------------------------------------------------------------

/// A grid of Plücker coordinates: `p[i][j]` for `i < j`, and 0 on and below the diagonal.
pub type Grid = [[f64; LATENT_DIM]; LATENT_DIM];

/// The upper-triangle cells `(i, j)`, `i < j`, row by row.
pub fn upper() -> impl Iterator<Item = (usize, usize)> {
    (0..LATENT_DIM).flat_map(|i| ((i + 1)..LATENT_DIM).map(move |j| (i, j)))
}

/// `plane`'s basis orthonormalised by Gram–Schmidt: `ê₁ = q₁/|q₁|`, `ê₂` the unit part of `q₂` across `ê₁`. `None` where
/// the basis spans no plane by the guards' test (`s < 10⁻⁶`, REQ-GUI-203).
pub fn orthonormal(plane: &Plane) -> Option<(Latent, Latent)> {
    if spans_no_plane(plane) {
        return None;
    }
    let n1 = norm(&plane.q1);
    let e1 = plane.q1.map(|x| x / n1);
    let along = dot(&plane.q2, &e1);
    let w: Latent = std::array::from_fn(|k| plane.q2[k] - along * e1[k]);
    let n2 = norm(&w);
    Some((e1, w.map(|x| x / n2)))
}

/// The Plücker coordinates of `plane`'s orientation, `p_ij = ê₁ᵢ ê₂ⱼ − ê₁ⱼ ê₂ᵢ` for `i < j`, from its basis
/// orthonormalised ([`orthonormal`]): `Σ p² = 1`, unchanged by a rotation within the plane. All zero where the basis
/// spans no plane.
pub fn plucker(plane: &Plane) -> Grid {
    let mut grid = [[0.0; LATENT_DIM]; LATENT_DIM];
    if let Some((e1, e2)) = orthonormal(plane) {
        for (i, j) in upper() {
            grid[i][j] = e1[i] * e2[j] - e1[j] * e2[i];
        }
    }
    grid
}

/// `Σ p²` over the upper triangle.
pub fn sum_sq(grid: &Grid) -> f64 {
    upper().map(|(i, j)| grid[i][j] * grid[i][j]).sum()
}

/// The dominant cell's least `|p|`, below which no cell is outlined and the readout shows "—" (R-452 C3).
pub const OUTLINE_MIN: f64 = 0.02;

/// The upper cell of the largest `|p|`, the first of equals row by row, when that `|p|` exceeds [`OUTLINE_MIN`].
pub fn dominant(grid: &Grid) -> Option<(usize, usize)> {
    let best = upper().fold((0, 1), |best, (i, j)| {
        if grid[i][j].abs() > grid[best.0][best.1].abs() {
            (i, j)
        } else {
            best
        }
    });
    (grid[best.0][best.1].abs() > OUTLINE_MIN).then_some(best)
}

/// A signed value with three decimals, "+" before a value ≥ 0 (R-452 C3).
pub fn value(p: f64) -> String {
    super::signed(p, 3)
}

/// The readout: "strongest: z_a × z_b +0.993 · Σ p² 1.000", or "strongest: — · Σ p² …" with no dominant cell (R-452 C3).
pub fn readout(grid: &Grid) -> String {
    let strongest = match dominant(grid) {
        Some((i, j)) => format!("{} × {} {}", AXES[i], AXES[j], value(grid[i][j])),
        None => "—".to_owned(),
    };
    format!("strongest: {strongest} · Σ p² {:.3}", sum_sq(grid))
}

/// The hover text of cell `(i, j)`: "z_i × z_j  +0.123" (R-452 C2).
pub fn hover_text(grid: &Grid, i: usize, j: usize) -> String {
    format!("{} × {}  {}", AXES[i], AXES[j], value(grid[i][j]))
}

// --- the steer --------------------------------------------------------------------------------------------------------

/// The steer to cell `(i, j)` of the Plücker grid from a chart at scale `scale` (R-452 C5): the basis `q₁ = σ e_i`,
/// `q₂ = σ e_j`, the plane `z_i × z_j` with `τ₁ = τ₂ = γ = 0`, the scale `σ` (the zoom) kept and `z₀` unchanged, one
/// `SetField` on the basis. `None` for a cell on or below the diagonal, which is empty.
pub fn steer(scale: f64, i: usize, j: usize) -> Option<SetField> {
    if i >= j || j >= LATENT_DIM {
        return None;
    }
    Some(basis_edit((axis(i, scale), axis(j, scale))))
}

/// [`steer`] on `sim`'s chart: a plane that takes the slice normal or a target moves it by the slice-axis rule in the same
/// edit ([`plane_edit`]; R-459 G4).
pub fn steer_on(sim: &SimConfig, i: usize, j: usize) -> Option<SetField> {
    if i >= j || j >= LATENT_DIM {
        return None;
    }
    let scale = View::read(sim).scale;
    Some(plane_edit(sim, (axis(i, scale), axis(j, scale))))
}

// --- the keyboard's cell cursor (R-452 C6) -----------------------------------------------------------------------------

/// Where the cell cursor starts on `grid`: the dominant cell, or the first upper cell when none is.
pub fn cursor_start(grid: &Grid) -> (usize, usize) {
    dominant(grid).unwrap_or((0, 1))
}

/// The cursor at `(i, j)` moved by an arrow `direction` with the step multiple `times` (±1, Shift's ±10, Alt's ±0.1):
/// ← → along its row, ↑ ↓ along its column, over the upper triangle only, stopping at its edges; Shift moves ten
/// cells, so to the row's or column's end, and Alt one, a cell being the smallest step (applied per R-369).
pub fn cursor_moved((i, j): (usize, usize), direction: Direction, times: f64) -> (usize, usize) {
    let n = if times.abs() > 1.0 { 10 } else { 1 };
    match direction {
        Direction::Left => (i, j.saturating_sub(n).max(i + 1)),
        Direction::Right => (i, (j + n).min(LATENT_DIM - 1)),
        Direction::Up => (i.saturating_sub(n), j),
        Direction::Down => ((i + n).min(j - 1), j),
    }
}

// --- the position readout ---------------------------------------------------------------------------------------------

/// The position readout's bars: each `z₀` component over the slider's half-range, clamped to ±1.
pub fn gauge(z0: &Latent) -> [f64; LATENT_DIM] {
    z0.map(|x| (x / Z0_HALF_RANGE).clamp(-1.0, 1.0))
}

// --- the drawing ------------------------------------------------------------------------------------------------------

/// The room the row names take left of the grid, in points (a look choice, R-390).
pub const ROW_NAMES_W: f32 = 26.0;
/// The room the column names take above the grid, and the gauge's names below it, in points (a look choice, R-390).
pub const COLUMN_NAMES_H: f32 = 22.0;
/// The gap between the grid and the position readout, in points.
pub const GAUGE_GAP: f32 = 6.0;
/// The position readout's height, both signs, in points (a look choice, R-390).
pub const GAUGE_H: f32 = 30.0;
/// A bar's width as a share of a cell's.
pub const BAR_SHARE: f32 = 0.6;
/// The names' size, in points.
pub const NAME_SIZE: f32 = 8.0;
/// The positive hue, a green (a look choice, R-390; R-452 C7).
pub const POSITIVE: Color32 = Color32::from_rgb(110, 200, 120);
/// The negative hue, a magenta (a look choice, R-390; R-452 C7).
pub const NEGATIVE: Color32 = Color32::from_rgb(215, 95, 185);
/// The dominant cell's outline width, in points (a look choice, R-390; R-452 C7).
pub const OUTLINE_W: f32 = 1.5;
/// The cell cursor's ring width, in points, in interaction mode's blue (R-452 C6).
pub const CURSOR_W: f32 = 2.0;

/// Where the grid and its readout sit in the cube's `square`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridPlace {
    /// The grid's top left, in points.
    pub origin: Pos2,
    /// A cell's side, in points.
    pub cell: f32,
}

impl GridPlace {
    /// The grid in `square`: the row names left of it, the column names above, and the position readout and its names
    /// under it, the cells square and as large as the square leaves.
    pub fn fit(square: Rect) -> Self {
        let across = (square.width() - ROW_NAMES_W) / LATENT_DIM as f32;
        let down =
            (square.height() - 2.0 * COLUMN_NAMES_H - GAUGE_GAP - GAUGE_H) / LATENT_DIM as f32;
        Self {
            origin: square.min + vec2(ROW_NAMES_W, COLUMN_NAMES_H),
            cell: across.min(down).max(0.0).floor(),
        }
    }

    /// Cell `(i, j)`'s rect: row `i`, column `j`.
    pub fn cell_rect(&self, i: usize, j: usize) -> Rect {
        Rect::from_min_size(
            self.origin + vec2(j as f32 * self.cell, i as f32 * self.cell),
            vec2(self.cell, self.cell),
        )
    }

    /// The grid's rect.
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.origin, vec2(1.0, 1.0) * self.cell * LATENT_DIM as f32)
    }

    /// The position readout's rect, under the grid, as wide.
    pub fn gauge_rect(&self) -> Rect {
        let grid = self.rect();
        Rect::from_min_size(
            pos2(grid.min.x, grid.max.y + GAUGE_GAP),
            vec2(grid.width(), GAUGE_H),
        )
    }

    /// Bar `k`'s rect for the value `v` in `[−1, 1]`: centred under column `k`, from the readout's zero line, up for a
    /// positive value.
    pub fn bar_rect(&self, k: usize, v: f64) -> Rect {
        let gauge = self.gauge_rect();
        let mid = gauge.center().y;
        let x = self.origin.x + (k as f32 + 0.5) * self.cell;
        let half_w = self.cell * BAR_SHARE / 2.0;
        let tip = mid - v as f32 * GAUGE_H / 2.0;
        Rect::from_min_max(
            pos2(x - half_w, tip.min(mid)),
            pos2(x + half_w, tip.max(mid)),
        )
    }
}

/// A colour between `a` and `b`, `t` of the way, in `[0, 1]`.
pub fn mix(a: Color32, b: Color32, t: f64) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// The cell background, which a zero coordinate draws: the theme's darkest background.
pub fn cell_background(visuals: &egui::Visuals) -> Color32 {
    visuals.extreme_bg_color
}

/// Cell `p`'s colour: its sign's hue at intensity `|p|`, from the cell background (R-452).
pub fn cell_colour(visuals: &egui::Visuals, p: f64) -> Color32 {
    // A zero, of either sign, draws the background whichever hue it takes.
    let hue = if p.is_sign_negative() {
        NEGATIVE
    } else {
        POSITIVE
    };
    mix(cell_background(visuals), hue, p.abs())
}

/// What the Plücker view draws, as data: the paint follows it, and the tests read it.
#[derive(Clone, Debug, PartialEq)]
pub struct PluckerDrawing {
    /// Where the grid sits.
    pub place: GridPlace,
    /// The grid.
    pub grid: Grid,
    /// The outlined cell.
    pub outline: Option<(usize, usize)>,
    /// The keyboard's cell cursor, in interaction mode.
    pub cursor: Option<(usize, usize)>,
    /// The position readout's bars.
    pub bars: [f64; LATENT_DIM],
    /// The highlighted bar: the slice axis.
    pub slice_axis: usize,
    /// The readout.
    pub readout: String,
}

/// The Plücker view of `plane` in the cube's `square`, with the cell cursor `cursor`, its position readout highlighting
/// the slice axis of the slice frame's defaults ([`View::of`]).
pub fn plucker_drawing(
    square: Rect,
    plane: &Plane,
    cursor: Option<(usize, usize)>,
) -> PluckerDrawing {
    plucker_drawing_with(square, plane, View::of(plane).d, cursor)
}

/// The Plücker view of `plane` in the cube's `square`, with the cell cursor `cursor`, its position readout highlighting
/// `slice_axis`, the slice frame's normal.
pub fn plucker_drawing_with(
    square: Rect,
    plane: &Plane,
    slice_axis: usize,
    cursor: Option<(usize, usize)>,
) -> PluckerDrawing {
    let grid = plucker(plane);
    PluckerDrawing {
        place: GridPlace::fit(square),
        outline: dominant(&grid),
        cursor,
        bars: gauge(&plane.z0),
        slice_axis,
        readout: readout(&grid),
        grid,
    }
}

/// Paints the upper cells of `grid` at `place`, with the `outline` and the `cursor`.
pub fn paint_cells(
    painter: &Painter,
    visuals: &egui::Visuals,
    place: GridPlace,
    grid: &Grid,
    outline: Option<(usize, usize)>,
    cursor: Option<(usize, usize)>,
) {
    for (i, j) in upper() {
        painter.rect_filled(
            place.cell_rect(i, j).shrink(0.5),
            0.0,
            cell_colour(visuals, grid[i][j]),
        );
    }
    if let Some((i, j)) = outline {
        painter.rect_stroke(
            place.cell_rect(i, j),
            0.0,
            Stroke::new(OUTLINE_W, visuals.strong_text_color()),
            egui::StrokeKind::Inside,
        );
    }
    paint_cursor(painter, visuals, place, cursor);
}

/// Paints the upper cells at `place` outlined and empty, with the `cursor`: a grid with no values to draw, whose cells a
/// click or the keyboard's cursor still steers to (R-452 C5, C6).
pub fn paint_empty_cells(
    painter: &Painter,
    visuals: &egui::Visuals,
    place: GridPlace,
    cursor: Option<(usize, usize)>,
) {
    for (i, j) in upper() {
        painter.rect_stroke(
            place.cell_rect(i, j),
            0.0,
            visuals.widgets.noninteractive.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    paint_cursor(painter, visuals, place, cursor);
}

/// Paints the keyboard's cell cursor's ring, in interaction mode's blue (R-452 C6).
fn paint_cursor(
    painter: &Painter,
    visuals: &egui::Visuals,
    place: GridPlace,
    cursor: Option<(usize, usize)>,
) {
    if let Some((i, j)) = cursor {
        painter.rect_stroke(
            place.cell_rect(i, j),
            0.0,
            Stroke::new(CURSOR_W, visuals.hyperlink_color),
            egui::StrokeKind::Outside,
        );
    }
}

/// Paints coordinate `k`'s `name` on the grid's left and top edges.
pub fn paint_edge_names(
    painter: &Painter,
    visuals: &egui::Visuals,
    place: GridPlace,
    k: usize,
    name: &str,
) {
    let font = egui::FontId::proportional(NAME_SIZE);
    let grid = place.rect();
    painter.text(
        pos2(grid.min.x - 3.0, place.cell_rect(k, 0).center().y),
        egui::Align2::RIGHT_CENTER,
        name,
        font.clone(),
        visuals.weak_text_color(),
    );
    upright(
        painter,
        pos2(place.cell_rect(0, k).center().x, grid.min.y - 3.0),
        name,
        &font,
        visuals.weak_text_color(),
        true,
    );
}

/// Paints `drawing` with `painter`: the upper cells, the outline and the cursor, the names on both edges, and the
/// position readout with its zero line and names, the slice axis's bar and name in the accent colour.
pub fn paint_plucker(painter: &Painter, visuals: &egui::Visuals, drawing: &PluckerDrawing) {
    let place = drawing.place;
    paint_cells(
        painter,
        visuals,
        place,
        &drawing.grid,
        drawing.outline,
        drawing.cursor,
    );
    let font = egui::FontId::proportional(NAME_SIZE);
    let gauge = place.gauge_rect();
    for (k, name) in AXES.iter().enumerate() {
        let colour = if k == drawing.slice_axis {
            visuals.hyperlink_color
        } else {
            visuals.weak_text_color()
        };
        paint_edge_names(painter, visuals, place, k, name);
        let x = place.cell_rect(0, k).center().x;
        painter.rect_filled(place.bar_rect(k, drawing.bars[k]), 0.0, colour);
        upright(
            painter,
            pos2(x, gauge.max.y + 3.0),
            name,
            &font,
            colour,
            false,
        );
    }
    painter.line_segment(
        [
            pos2(gauge.min.x, gauge.center().y),
            pos2(gauge.max.x, gauge.center().y),
        ],
        Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color),
    );
}

/// Paints `text` turned to read upwards, ending at `at` when `above`, starting at it otherwise, centred on its x.
fn upright(
    painter: &Painter,
    at: Pos2,
    text: &str,
    font: &egui::FontId,
    colour: Color32,
    above: bool,
) {
    let galley = painter.layout_no_wrap(text.to_owned(), font.clone(), colour);
    let (w, h) = (galley.size().x, galley.size().y);
    // Turned a quarter anticlockwise about its top left, the text runs up from there, its height to the right.
    let bottom = if above { at.y } else { at.y + w };
    let origin = pos2(at.x - h / 2.0, bottom);
    painter.add(
        egui::epaint::TextShape::new(origin, galley, colour)
            .with_angle(-std::f32::consts::FRAC_PI_2),
    );
}
