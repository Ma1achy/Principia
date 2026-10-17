//! The dev GUI's windows (render_gui_spec §G5, §G7, §G8, §G12; GUI_DESIGN_NOTES § "03 Chart builder", § "04 Windows",
//! § "05 Inspector", § "12 Console"; R-390, ORDER item 5): Run, Chart builder, Export & share, Display, Profiler,
//! Console and the Inspector's frame, one registry of seven that the Windows menu lists and every opener reads.
//!
//! **The window rule** (render_gui_spec §G3; R-405, R-409): an open window is a scope outside the big scopes' Tab
//! order that takes the focus when it opens, on its first section's first element, in navigation mode; Tab and
//! Shift+Tab move between its sections and stay in it; the arrows move within a section in navigation mode; Esc backs
//! out one level, and from a section closes the window, the focus going back to the scope that opened it
//! (`crate::keyboard::Keyboard::open_window`). The console's sections are its two scopes, the header bar and the log,
//! landed on themselves (R-450); it opening by itself on an error takes no focus. Every window's × is a control in its
//! scope tree, Enter on it closing the window as Esc does: the console's the last of its header bar (R-450), every
//! other window's its last section, drawn at its title row's right (the R-450 audit's finding, applied per R-369).
//!
//! **Where they open** (applied per R-369): each at a fixed rect over the side panels or the bottom row, never over the
//! figure (§G1, "The figure is never covered"; R-390's review list): Profiler and Chart builder over the Manifold view,
//! Run, Export & share, Display and the Inspector over the Trajectory panel, the console over the bottom row (§G12).
//! The artboards' windows are layouts of their content, kept inside those rects, each scrolling where it is longer.
//!
//! Each window's state is the GUI's own (`ViewUI`'s open list, and its drafts), never the engine's; what reaches the
//! engine goes through `SetField` (gui_state_contract §1).

pub mod chart_builder;
pub mod console;
pub mod display;
pub mod export;
pub mod inspector;
pub mod profiler;
pub mod run;

use eframe::egui::{self, Id, Rect, Response, RichText, ScrollArea, Ui};
use engine::contract::view_ui::Window;

use crate::keyboard::scopes::{Scope, ScopeId, ScopeTree};
use crate::keyboard::Keyboard;
use crate::layout::Layout;

/// One window of the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The window, by its contract name.
    pub window: Window,
    /// Its scope in the keyboard's tree.
    pub scope: ScopeId,
    /// Its name in the Windows menu (§G5's names).
    pub name: &'static str,
    /// Its title bar, as its artboard's.
    pub title: &'static str,
    /// Its entry in the Windows menu, a scope under it.
    pub menu_entry: ScopeId,
    /// Its close button ×, a control in its scope tree (R-450; the audit's finding, applied per R-369): the last of
    /// the console's header bar, and every other window's last section, drawn at its title row's right.
    pub close: ScopeId,
}

/// The seven windows, in the Windows menu's order (applied per R-369: the first five as TASK-M6-24 listed them, then
/// the Chart builder and the Inspector).
pub const REGISTRY: [Entry; 7] = [
    Entry {
        window: Window::Run,
        scope: run::WINDOW,
        name: "Run",
        title: run::TITLE,
        menu_entry: "windows_run",
        close: "run_close",
    },
    Entry {
        window: Window::Profiler,
        scope: profiler::WINDOW,
        name: "Profiler",
        title: profiler::TITLE,
        menu_entry: "windows_profiler",
        close: "profiler_close",
    },
    Entry {
        window: Window::Export,
        scope: export::WINDOW,
        name: "Export & share",
        title: export::TITLE,
        menu_entry: "windows_export",
        close: "export_close",
    },
    Entry {
        window: Window::Display,
        scope: display::WINDOW,
        name: "Display",
        title: display::TITLE,
        menu_entry: "windows_display",
        close: "display_close",
    },
    Entry {
        window: Window::Console,
        scope: console::WINDOW,
        name: "Console",
        title: console::TITLE,
        menu_entry: "windows_console",
        close: console::CLOSE,
    },
    Entry {
        window: Window::ChartBuilder,
        scope: chart_builder::WINDOW,
        name: "Chart builder",
        title: chart_builder::TITLE,
        menu_entry: "windows_chart_builder",
        close: "chart_builder_close",
    },
    Entry {
        window: Window::Inspector,
        scope: inspector::WINDOW,
        name: "Inspector",
        title: inspector::TITLE,
        menu_entry: "windows_inspector",
        close: "inspector_close",
    },
];

/// The registry's entry for `window`.
pub fn entry(window: Window) -> &'static Entry {
    REGISTRY
        .iter()
        .find(|e| e.window == window)
        .expect("every window is in the registry")
}

/// The window control `id` opens, as its click or Enter: a Windows menu entry, the top bar's Run…, Profiler… and
/// Export…, and the footer (the console, §G12). The Manifold view's Chart builder… and open in Inspector, and the
/// Trajectory panel's IC Inspector… and Open full viewer…, ask through their own panels instead, the one path for their
/// click and their Enter, so the lock badge's guard holds (open in Inspector acts only while locked, §G4).
pub fn opened_by(id: &str) -> Option<Window> {
    if let Some(e) = REGISTRY.iter().find(|e| e.menu_entry == id) {
        return Some(e.window);
    }
    match id {
        "run" => Some(Window::Run),
        "profiler" => Some(Window::Profiler),
        "export" => Some(Window::Export),
        crate::explore::footer::FOOTER => Some(Window::Console),
        _ => None,
    }
}

/// Joins each window to `tree`, outside the big scopes' Tab order, its sections and elements under it: the console's
/// two scopes, its × the last of its header bar (R-450); every other window's × its last section, so Shift+Tab from
/// its landing reaches it (the audit's finding, applied per R-369).
pub fn register(tree: &mut ScopeTree) {
    for e in REGISTRY {
        let scope = match e.window {
            Window::Console => Scope::scoped_window(e.scope, e.title),
            _ => Scope::window(e.scope, e.title),
        };
        tree.register(None, scope);
    }
    run::register(tree);
    profiler::register(tree);
    export::register(tree);
    display::register(tree);
    console::register(tree);
    chart_builder::register(tree);
    inspector::register(tree);
    for e in REGISTRY.iter().filter(|e| e.window != Window::Console) {
        tree.register(Some(e.scope), Scope::control(e.close, CLOSE_LABEL));
    }
}

/// The close button's name in the breadcrumb (R-450 A6).
pub const CLOSE_LABEL: &str = "Close";
/// The close button's text: §G12's ×.
pub const CLOSE_TEXT: &str = "×";

/// The window whose close button is `id`, if any.
pub fn closed_by(id: &str) -> Option<Window> {
    REGISTRY.iter().find(|e| e.close == id).map(|e| e.window)
}

/// Where `window` opens in `layout` (applied per R-369, as the module says): never over the figure.
pub fn rect(window: Window, layout: &Layout) -> Rect {
    match window {
        Window::Profiler | Window::ChartBuilder => layout.manifold_view,
        Window::Run | Window::Export | Window::Display | Window::Inspector => layout.trajectory,
        Window::Console => layout.bottom_row,
    }
}

/// The windows' own state: each window's draft and view, the GUI's, never the engine's.
#[derive(Debug, Default)]
pub struct State {
    /// The Run window's draft.
    pub run: run::Draft,
    /// The Chart builder's draft.
    pub chart: chart_builder::Draft,
    /// Export & share's choices.
    pub export: export::State,
    /// Display's choices.
    pub display: display::State,
    /// The Profiler's tab and transport.
    pub profiler: profiler::State,
    /// The console's filters and scroll.
    pub console: console::State,
    /// The Inspector's IC and timeline.
    pub inspector: inspector::State,
}

/// The frames a window keeps scrolling to its focused element after the focus moves there: egui lays a new window
/// out in an invisible sizing pass first, so the first frame's scroll can be lost (a look choice, R-390).
pub const SCROLL_FRAMES: u8 = 3;

/// What a frame's drawing of the windows needs from the keyboard: where to place each scope, and which is focused.
pub struct Keys<'a> {
    /// The keyboard layer, which records where each scope is drawn.
    pub keyboard: &'a mut Keyboard,
    /// The focused scope, the last of the focus path, while the window is to scroll to it: for
    /// [`SCROLL_FRAMES`] frames after the focus moved.
    pub focused: Option<String>,
}

impl Keys<'_> {
    /// Scope `id` is the widget of `response`; focused, the window scrolls to it.
    pub fn place(&mut self, id: ScopeId, response: &Response) {
        self.keyboard.place_widget(id, response);
        if self.focused.as_deref() == Some(id) {
            response.scroll_to_me(None);
        }
    }

    /// Scope `id` was drawn in `rect`, in `ui`: placed on the part of it `ui` shows; focused, the window scrolls to it,
    /// so the ring is drawn on the section, never past the window's edge.
    pub fn place_rect(&mut self, ui: &Ui, id: ScopeId, rect: Rect) {
        if self.focused.as_deref() == Some(id) {
            ui.scroll_to_rect(rect, None);
        }
        self.keyboard.place(id, rect.intersect(ui.clip_rect()));
    }
}

/// Draws `entry`'s window's frame, fixed at `rect`, its content scrolling when `scroll`, the window's scope placed on
/// its rect. Every window but the console starts with a title row: its title, and its × at the right, the scope
/// `entry.close` (the audit's finding: egui's own title-bar × is outside the scope tree, which no key reaches); the
/// console's header bar holds its title and its × (R-450). Returns whether it is still open: false once its title
/// row's × is clicked.
pub fn frame(
    ctx: &egui::Context,
    entry: &Entry,
    rect: Rect,
    keys: &mut Keys<'_>,
    scroll: bool,
    add: impl FnOnce(&mut Ui, &mut Keys<'_>),
) -> bool {
    let (scope, title) = (entry.scope, entry.title);
    let mut open = true;
    let shown = egui::Window::new(title)
        .id(Id::new(("window", scope)))
        // The size first, then the rect: egui lays a new window out in an invisible sizing pass, at its default size
        // pushed back on screen; at the rect's own size, that pass already sits on the rect, never over the figure.
        .default_size(rect.size())
        .fixed_rect(rect)
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            if entry.window != Window::Console {
                open = !title_row(ui, keys, title, entry.close);
            }
            if scroll {
                ScrollArea::vertical()
                    .id_salt(scope)
                    .auto_shrink(false)
                    .show(ui, |ui| add(ui, keys));
            } else {
                add(ui, keys);
            }
        });
    if let Some(shown) = shown {
        keys.keyboard.place(scope, shown.response.rect);
    }
    open
}

/// Draws a window's title row: `title`, centred, and its × at the right, placed as scope `close`; then a separator.
/// Returns whether the × was clicked.
fn title_row(ui: &mut Ui, keys: &mut Keys<'_>, title: &str, close: ScopeId) -> bool {
    let mut clicked = false;
    let height =
        ui.text_style_height(&egui::TextStyle::Heading) + ui.spacing().button_padding.y * 2.0;
    let size = egui::vec2(ui.available_width(), height);
    let right = egui::Layout::right_to_left(egui::Align::Center);
    ui.allocate_ui_with_layout(size, right, |ui| {
        let button = ui.add(egui::Button::new(RichText::new(CLOSE_TEXT).heading()).frame(false));
        clicked = button.clicked();
        keys.place(close, &button);
        let rest = egui::vec2(ui.available_width(), height);
        let centred = egui::Layout::centered_and_justified(egui::Direction::LeftToRight);
        ui.allocate_ui_with_layout(rest, centred, |ui| ui.label(RichText::new(title).heading()));
    });
    ui.separator();
    clicked
}

/// Draws one section: its heading, as the artboards' "▾ Title", and its content, the section's scope placed on the
/// rect they take.
pub fn section(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    id: ScopeId,
    heading: &str,
    add: impl FnOnce(&mut Ui, &mut Keys<'_>),
) {
    let response = ui
        .scope(|ui| {
            ui.label(RichText::new(format!("▾ {heading}")).strong());
            add(ui, keys);
        })
        .response;
    keys.place_rect(ui, id, response.rect);
    ui.add_space(8.0);
}

/// A row of one label and its value, as the windows' forms draw them, the label in a fixed column.
pub fn form_label(ui: &mut Ui, label: &str) {
    ui.add_sized(
        [FORM_LABEL_W, ui.spacing().interact_size.y],
        egui::Label::new(label),
    );
}

/// The form labels' column width, in points (a look choice, R-390).
pub const FORM_LABEL_W: f32 = 96.0;

/// A placeholder line, in egui's weak text colour.
pub fn note(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).weak());
}
