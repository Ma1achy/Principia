//! The hover label (render_gui_spec §G1, §G2; GUI_DESIGN_NOTES § "01 Explore"; 01_main.png): the figure's own mark,
//! not a notification, drawn by the GUI beside the cursor from the snapshot's sample under it. It names a fate only
//! once the fate is decided; before that it reads "undecided · still interacting at t … of …", the playhead's `t` of
//! the horizon `T`. It is the only mark the GUI draws over the figure for the hover: the hover path is the engine's,
//! drawn into the figure (the mock's stand-in here; the compositor's trace layer on the real engine, TASK-M8-10). The
//! GUI writes the cursor's figure point to `RenderState`'s hover through a no-history `SetField` as the pointer moves,
//! and none when it leaves the figure, which clears the label and the path.

use eframe::egui::{self, pos2, vec2, Color32, Id, LayerId, Order, Pos2, Rect};
use engine::contract::render_state::Hover;
use engine::contract::sample::Sample;
use engine::contract::set_field::{Edit, RenderField, SetField};

use crate::explore::time::number;

/// The label's offset from the cursor, in points (a look choice, R-390).
pub const OFFSET: egui::Vec2 = vec2(14.0, -10.0);
/// The widest the label grows before it wraps, in points (a look choice, R-390).
pub const MAX_WIDTH: f32 = 260.0;
/// The label's padding, in points (a look choice, R-390).
pub const PADDING: f32 = 4.0;
/// The label's backing, so it reads over any stand-in colour: near-black at most of full opacity (a look choice,
/// R-390; 01_main.png's label sits on a dark backing).
pub const BACKING: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 200);
/// The key hint after the fate, as 01_main.png's.
pub const LISTEN_HINT: &str = "L listen";

/// The label of `sample` with the playhead at `t` and the horizon `t_max`: undecided, "undecided · still interacting at
/// t 12.4 of 50"; decided, its fate, the one conversion function's name of its `state` and `detail` (R-449;
/// `engine::contract::display::name`), and `t_end`, "escape body 1 · t_end 31.2" (applied per R-369: the decided
/// wording, a look silence under R-390). Each ends with the listen hint.
pub fn text(sample: &Sample, t: f64, t_max: f64) -> String {
    let fate = match (&sample.fate, sample.t_end) {
        (Some(fate), Some(t_end)) => format!("{fate} · t_end {t_end:.1}"),
        (Some(fate), None) => fate.clone(),
        (None, _) => format!(
            "undecided · still interacting at t {t:.1} of {}",
            number(t_max)
        ),
    };
    format!("{fate} · {LISTEN_HINT}")
}

/// The hover edit for the cursor at `at`, or none: always marked "no history" (R-101).
pub fn hover_edit(at: Option<[f64; 2]>) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::Hover(Hover { at })),
        no_history: true,
    }
}

/// The figure point `(s, t)` of the pointer at `pointer`, if it is over `figure` and no layer above the figure's takes
/// it: `over_figure` says whether the layer under the pointer is the figure's.
pub fn figure_point(figure: Rect, pointer: Option<Pos2>, over_figure: bool) -> Option<[f64; 2]> {
    let pos = pointer.filter(|p| over_figure && figure.contains(*p))?;
    let (s, t) = crate::explore::manifold_view::navigate::figure_coords(figure, pos);
    Some([s, t])
}

/// Where the label of `size` goes for the cursor at `cursor`: [`OFFSET`] from it, kept inside `figure`.
pub fn placed(figure: Rect, cursor: Pos2, size: egui::Vec2) -> Rect {
    let min = cursor + OFFSET - vec2(0.0, size.y);
    let max_x = (figure.max.x - size.x).max(figure.min.x);
    let max_y = (figure.max.y - size.y).max(figure.min.y);
    Rect::from_min_size(
        pos2(
            min.x.clamp(figure.min.x, max_x),
            min.y.clamp(figure.min.y, max_y),
        ),
        size,
    )
}

/// Draws the label `text` beside `cursor`, inside `figure`, above the figure's layer.
pub fn show(ctx: &egui::Context, figure: Rect, cursor: Pos2, text: String) {
    let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("hover label")));
    let font = egui::FontId::monospace(12.0);
    let galley = painter.layout(text, font, Color32::WHITE, MAX_WIDTH);
    let rect = placed(
        figure,
        cursor,
        galley.size() + vec2(2.0 * PADDING, 2.0 * PADDING),
    );
    painter.rect_filled(rect, 2.0, BACKING);
    painter.galley(rect.min + vec2(PADDING, PADDING), galley, Color32::WHITE);
}
