//! Where each window is (R-461; render_gui_spec §G5): every window but the console is dragged by its title bar and
//! resized from its edges and corners, down to its minimum size, and kept at least partly on screen; its place is the
//! GUI's own state, never the contract's (gui_state_contract §2), remembered across sessions in the GUI's own settings
//! (`crate::settings`).
//!
//! - **The title bar** is the band [`TITLE_BAR_H`] tall along a window's top: its title and its × (`super::frame`).
//!   A drag that starts on it, off the ×, moves the window by the drag.
//! - **The edges and corners** are bands [`EDGE_GRAB`] either side of each edge, and squares [`CORNER_GRAB`] either
//!   side of each corner; a drag on one moves that edge, or the corner's two, by the drag, never below the window's
//!   minimum size ([`min_size`], I3).
//! - **On screen** (I2): a window keeps at least its title bar's height, and [`KEEP_W`] points of its title bar's
//!   width, inside the GUI, after a drag, after the GUI's window is resized, and when a remembered place is opened on a
//!   smaller screen ([`on_screen`]).
//! - **With nothing remembered** a window opens where it opened before R-461: the artboards' places (`super::rect`).
//! - Moving and resizing are pointer actions only; the keyboard's model of a window is unchanged (I5).

use eframe::egui::{pos2, vec2, CursorIcon, Rect, Vec2};
use engine::contract::view_ui::Window;

/// The title bar's height, in points: the band along a window's top that a drag moves it by, holding its title row
/// (the window frame's margin, the heading's line and the separator under it, about 34 points with the theme's fonts;
/// a look choice, R-390).
pub const TITLE_BAR_H: f32 = 36.0;

/// How much of a window's title bar's width stays inside the GUI, in points (R-461 I2).
pub const KEEP_W: f32 = 64.0;

/// How far either side of an edge a drag grabs it, in points (a look choice, R-390).
pub const EDGE_GRAB: f32 = 4.0;

/// How far either side of a corner a drag grabs it, in points: corners take priority over the edges they join (a
/// look choice, R-390).
pub const CORNER_GRAB: f32 = 10.0;

/// The windows that move and resize: every window but the console, which is docked (R-461 I1; R-463).
pub fn movable(window: Window) -> bool {
    window != Window::Console
}

/// A window's minimum size, in points: its title bar and its controls usable (R-461 I3; the values are TASK-M6-42's,
/// applied per R-369). Each is the width of its widest row of controls, rounded up (the Inspector's, its three panes'
/// row, is its artboard width), and enough height for its title bar and a section's first rows, the rest scrolling.
pub fn min_size(window: Window) -> Vec2 {
    match window {
        Window::Run => vec2(300.0, 200.0),
        Window::ChartBuilder => vec2(240.0, 240.0),
        Window::Export => vec2(280.0, 200.0),
        Window::Display => vec2(260.0, 200.0),
        Window::Profiler => vec2(280.0, 240.0),
        Window::Inspector => vec2(376.0, 240.0),
        Window::Console => Vec2::ZERO,
    }
}

/// Which edges a drag moves: one for an edge, two for a corner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges {
    /// The left edge.
    pub left: bool,
    /// The right edge.
    pub right: bool,
    /// The top edge.
    pub top: bool,
    /// The bottom edge.
    pub bottom: bool,
}

impl Edges {
    /// The four edges and the four corners, edges first: the order their handles are laid, the last on top, so a
    /// corner takes priority over the two edges it joins.
    pub const ALL: [Edges; 8] = [
        Edges::of(true, false, false, false),
        Edges::of(false, true, false, false),
        Edges::of(false, false, true, false),
        Edges::of(false, false, false, true),
        Edges::of(true, false, true, false),
        Edges::of(false, true, true, false),
        Edges::of(true, false, false, true),
        Edges::of(false, true, false, true),
    ];

    /// The edges `left`, `right`, `top` and `bottom`.
    pub const fn of(left: bool, right: bool, top: bool, bottom: bool) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Whether this is a corner: one vertical edge and one horizontal.
    pub fn is_corner(self) -> bool {
        (self.left || self.right) && (self.top || self.bottom)
    }

    /// The handle's rect on `rect`'s edges: a band [`EDGE_GRAB`] either side of an edge, short of the corners, or a
    /// square [`CORNER_GRAB`] either side of a corner.
    pub fn handle(self, rect: Rect) -> Rect {
        let x = if self.left { rect.min.x } else { rect.max.x };
        let y = if self.top { rect.min.y } else { rect.max.y };
        if self.is_corner() {
            return Rect::from_center_size(pos2(x, y), Vec2::splat(2.0 * CORNER_GRAB));
        }
        if self.left || self.right {
            Rect::from_min_max(
                pos2(x - EDGE_GRAB, rect.min.y + CORNER_GRAB),
                pos2(x + EDGE_GRAB, rect.max.y - CORNER_GRAB),
            )
        } else {
            Rect::from_min_max(
                pos2(rect.min.x + CORNER_GRAB, y - EDGE_GRAB),
                pos2(rect.max.x - CORNER_GRAB, y + EDGE_GRAB),
            )
        }
    }

    /// The pointer's cursor over the handle: a two-way arrow along the drag it makes.
    pub fn cursor(self) -> CursorIcon {
        match (self.left || self.right, self.top || self.bottom) {
            (true, true) if self.left == self.top => CursorIcon::ResizeNwSe,
            (true, true) => CursorIcon::ResizeNeSw,
            (true, false) => CursorIcon::ResizeHorizontal,
            _ => CursorIcon::ResizeVertical,
        }
    }
}

/// What a drag holds: a window's title bar, or edges of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grip {
    /// The title bar: the drag moves the window.
    Title,
    /// Edges, or a corner: the drag resizes the window.
    Edges(Edges),
}

/// The title bar of a window at `rect`: the band [`TITLE_BAR_H`] tall along its top.
pub fn title_bar(rect: Rect) -> Rect {
    Rect::from_min_max(
        rect.min,
        pos2(rect.max.x, (rect.min.y + TITLE_BAR_H).min(rect.max.y)),
    )
}

/// `rect` moved by the least that keeps it on `screen` (R-461 I2): its title bar's height and [`KEEP_W`] points of its
/// width inside. Its size is kept. On a screen less than a title bar tall, its top is kept on screen.
pub fn on_screen(rect: Rect, screen: Rect) -> Rect {
    let left = rect
        .min
        .x
        .max(screen.min.x + KEEP_W - rect.width())
        .min(screen.max.x - KEEP_W);
    let top = rect.min.y.min(screen.max.y - TITLE_BAR_H).max(screen.min.y);
    Rect::from_min_size(pos2(left, top), rect.size())
}

/// The window at `start` with `edges` moved by `delta`, each moved edge stopped where the window would be smaller than
/// `min`, or where its title bar would leave `screen` (R-461 I2, I3): the top edge stays on screen and a title bar's
/// height above its bottom, the left edge [`KEEP_W`] left of its right, the right edge [`KEEP_W`] right of its left.
pub fn resized(start: Rect, edges: Edges, delta: Vec2, min: Vec2, screen: Rect) -> Rect {
    let mut r = start;
    if edges.left {
        r.min.x = (start.min.x + delta.x)
            .min(start.max.x - min.x)
            .min(screen.max.x - KEEP_W);
    }
    if edges.right {
        r.max.x = (start.max.x + delta.x)
            .max(start.min.x + min.x)
            .max(screen.min.x + KEEP_W);
    }
    if edges.top {
        r.min.y = (start.min.y + delta.y)
            .min(start.max.y - min.y)
            .min(screen.max.y - TITLE_BAR_H)
            .max(screen.min.y);
    }
    if edges.bottom {
        r.max.y = (start.max.y + delta.y).max(start.min.y + min.y);
    }
    r
}

/// The drag in progress: the window, what it holds, and the window's rect when it began.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Drag {
    window: Window,
    grip: Grip,
    start: Rect,
}

/// The windows' places: each window moved or resized, or remembered, by its rect; the others where the artboards put
/// them. The GUI's own state, never the contract's (R-461 I4).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Places {
    rects: Vec<(Window, Rect)>,
    drag: Option<Drag>,
}

impl Places {
    /// The places `rects` remembered, each window's last.
    pub fn remembered(rects: impl IntoIterator<Item = (Window, Rect)>) -> Self {
        let mut places = Self::default();
        for (window, rect) in rects {
            places.set(window, rect);
        }
        places
    }

    /// The windows with a place of their own, in the order first placed.
    pub fn rects(&self) -> &[(Window, Rect)] {
        &self.rects
    }

    /// `window`'s own place, if it has one.
    pub fn get(&self, window: Window) -> Option<Rect> {
        self.rects
            .iter()
            .find(|(w, _)| *w == window)
            .map(|(_, r)| *r)
    }

    fn set(&mut self, window: Window, rect: Rect) {
        match self.rects.iter_mut().find(|(w, _)| *w == window) {
            Some(slot) => slot.1 = rect,
            None => self.rects.push((window, rect)),
        }
    }

    /// Where `window` is drawn on `screen`: its own place kept on screen, or `default`, the artboards' place.
    pub fn shown(&self, window: Window, default: Rect, screen: Rect) -> Rect {
        match self.get(window) {
            Some(rect) => on_screen(rect, screen),
            None => default,
        }
    }

    /// A frame of the drag on `window`'s `grip`, the pointer `total` from where it was pressed, the window `shown`
    /// where it was drawn: the window moved, or resized, from where it was when the drag began, and kept on `screen`.
    pub fn drag(&mut self, window: Window, grip: Grip, shown: Rect, total: Vec2, screen: Rect) {
        let start = match self.drag {
            Some(d) if d.window == window && d.grip == grip => d.start,
            _ => {
                self.drag = Some(Drag {
                    window,
                    grip,
                    start: shown,
                });
                shown
            }
        };
        let rect = match grip {
            Grip::Title => start.translate(total),
            Grip::Edges(edges) => resized(start, edges, total, min_size(window), screen),
        };
        self.set(window, on_screen(rect, screen));
    }

    /// Ends the drag in progress, if any; returns whether one ended, so the places are saved.
    pub fn release(&mut self) -> bool {
        self.drag.take().is_some()
    }
}
