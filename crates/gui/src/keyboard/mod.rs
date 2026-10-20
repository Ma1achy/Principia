//! Keyboard navigation (render_gui_spec §G3; GUI_DESIGN_NOTES § "07 Keyboard"; R-390, ORDER item 2): the tree of
//! scopes each screen joins, the global key table, held keys' delay and repeat, and the `?` shortcuts overlay. The
//! focus is `ViewUI`'s (gui_state_contract §2), with the keyboard's mode, navigation or interaction (R-409); what the
//! user sees of it is the focus ring, orange in navigation mode and blue in interaction mode, and the top bar's
//! breadcrumb (`explore::breadcrumb`), nothing else.
//!
//! The layer takes its keys from the raw input before egui sees them, so egui's own Tab and arrow focus never moves;
//! it stands aside only while a text field holds the keyboard: one being typed in (interaction mode on a text scope,
//! after Enter or a click on it, R-456) takes every key but Esc, Enter and Tab (R-409 G4, G5), and any other text
//! entry the mouse focused (a value's typed entry, egui's) takes every key. A key's release goes to whoever saw its
//! press. The menus are scopes too: the focus inside a menu opens it, and a menu the mouse opens takes the focus. In
//! either mode the letter and Space shortcuts are egui's to read, Ctrl+Z is never the layer's, and `?` opens the
//! shortcuts (R-409 F1, G1).
//!
//! Keyboard navigation is on or off, apart from the focus (render_gui_spec §G3; R-460, R-464): the layer's own flag,
//! never `ViewUI`'s. Any key press turns it on, and any pointer press, a click's or a drag's, turns it off; a scroll
//! or a hover leaves it. The focus moves as ever, whoever moved it, so a window the mouse opened has the focus on its
//! landing with keyboard navigation off. Enter in navigation mode, while it is off, only turns it on, without acting.

pub mod keymap;
pub mod overlay;
pub mod repeat;
pub mod scopes;

use eframe::egui::{
    self, Event, Id, Key, LayerId, Modifiers, PointerButton, Popup, RawInput, Rect, Response,
};
use engine::contract::view_ui::{Focus, KeyMode, Mode};

use keymap::Command;
use repeat::Repeat;
use scopes::{Adjust, Outcome, ScopeId, ScopeTree, Scroll};

/// Where a scope was drawn this frame: its rect, and the egui widget it is, for a control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// Its rect, in points.
    pub rect: Rect,
    /// The widget's id, for a control drawn as one.
    pub widget: Option<Id>,
    /// Where a press reaches the widget: its rect clipped as egui clips it (a scroll area's, a window's), for a
    /// control drawn as one; otherwise its rect.
    pub hit: Rect,
    /// The layer the widget is drawn on (its window's, or the page's), for a control drawn as one.
    pub layer: Option<LayerId>,
}

/// A key event the layer took.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Press {
    key: Key,
    modifiers: Modifiers,
    pressed: bool,
    /// Pressed while keyboard navigation was off: an Enter in navigation mode then only turns it on (R-460 H1, R-464
    /// M2).
    reveal: bool,
}

/// What the layer took from the raw input, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Taken {
    /// One of its keys.
    Key(Press),
    /// The window lost the focus: no release comes for a key held then.
    FocusLost,
}

/// The keyboard layer.
pub struct Keyboard {
    explore: ScopeTree,
    stain: ScopeTree,
    repeat: Repeat,
    pending: Vec<Taken>,
    /// The keys whose press the layer took: their releases are its own; every other release is egui's.
    down: Vec<Key>,
    /// The pointer buttons whose press the overlay swallowed: their releases are swallowed too.
    swallowed: Vec<PointerButton>,
    places: Vec<(ScopeId, Place)>,
    /// Whether the `?` shortcuts overlay is open.
    pub shortcuts_open: bool,
    overlay_rect: Option<Rect>,
    open_menu: Option<ScopeId>,
    adjusted: Vec<Adjust>,
    scrolled: Vec<Scroll>,
    /// Whether a text scope is being typed in from the keyboard: interaction mode on it after the last run.
    typing: Option<ScopeId>,
    /// Each open window that took the focus, with the path of the scope that opened it: where the focus goes back to
    /// when it closes (render_gui_spec §G3; R-405). GUI-local, never `ViewUI`'s (applied per R-369).
    openers: Vec<(ScopeId, Vec<String>)>,
    /// The windows Esc left in the last run, for the app to close.
    left: Vec<ScopeId>,
    /// Whether keyboard navigation is on: the ring and the breadcrumb's focus path drawn (R-460 H4, R-464).
    navigating: bool,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Keyboard {
    /// The layer, each mode's screens registered: Explore's eight big scopes in §G3's Tab order, the footer last
    /// (R-405); on the Stain page, the top bar, the library drawer, the graph canvas, the preview and node inspector,
    /// the code and Problems pane, and the footer last (`crate::stain`). The windows join both trees, outside the Tab
    /// order (§G5).
    pub fn new() -> Self {
        let mut explore = ScopeTree::new();
        crate::explore::top_bar::register(&mut explore);
        crate::explore::register(&mut explore);
        crate::explore::footer::register(&mut explore);
        crate::windows::register(&mut explore);
        let mut stain = ScopeTree::new();
        crate::explore::top_bar::register(&mut stain);
        crate::stain::register(&mut stain);
        crate::explore::footer::register(&mut stain);
        crate::windows::register(&mut stain);
        Self {
            explore,
            stain,
            repeat: Repeat::default(),
            pending: Vec::new(),
            down: Vec::new(),
            swallowed: Vec::new(),
            places: Vec::new(),
            shortcuts_open: false,
            overlay_rect: None,
            open_menu: None,
            adjusted: Vec::new(),
            scrolled: Vec::new(),
            typing: None,
            openers: Vec::new(),
            left: Vec::new(),
            navigating: false,
        }
    }

    /// The tree of `mode`.
    pub fn tree(&self, mode: Mode) -> &ScopeTree {
        match mode {
            Mode::Explore => &self.explore,
            Mode::Stain => &self.stain,
        }
    }

    /// The tree of `mode`, for a screen to join.
    pub fn tree_mut(&mut self, mode: Mode) -> &mut ScopeTree {
        match mode {
            Mode::Explore => &mut self.explore,
            Mode::Stain => &mut self.stain,
        }
    }

    /// The menu the layer holds open, if any.
    pub fn open_menu(&self) -> Option<ScopeId> {
        self.open_menu
    }

    /// The text scope being typed in from the keyboard, if any (R-409 G4).
    pub fn typing(&self) -> Option<ScopeId> {
        self.typing
    }

    /// Whether keyboard navigation is on: the ring and the breadcrumb's focus path are drawn only while it is
    /// (render_gui_spec §G3; R-460 H4, R-464).
    pub fn navigating(&self) -> bool {
        self.navigating
    }

    /// Follows keyboard navigation through `raw`'s events while the layer takes no keys (the egui layer hidden, or
    /// present mode): a key press turns it on, a pointer press off (R-460, R-464).
    pub fn follow_input(&mut self, raw: &RawInput) {
        for event in &raw.events {
            self.navigating = navigation_after(self.navigating, event);
        }
    }

    /// Takes the table's keys out of `raw`, before egui's pass sees them, and the release of each key whose press it
    /// took; the rest are egui's, and, while the overlay is closed, so is every key while a text entry the mouse
    /// focused that is no text scope holds the keyboard, and every key but Esc, Enter, Tab and Shift+Tab while a text
    /// scope is typed in, after Enter or a click on it (R-409 G4, G5; R-456). The system's own repeats go with their
    /// press: the layer drops its own, as it repeats them itself. A lost window focus forgets the held keys. While the
    /// overlay is open it takes every click: a press outside its frame closes it.
    pub fn take_keys(&mut self, ctx: &egui::Context, raw: &mut RawInput) {
        let typing = !self.shortcuts_open && self.typing.is_some();
        let aside = !self.shortcuts_open && !typing && ctx.text_edit_focused();
        if !aside && !typing {
            // A widget egui focused would draw itself active: the focus is the layer's alone.
            if let Some(id) = ctx.memory(|m| m.focused()) {
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
        }
        let overlay = self.shortcuts_open.then_some(self.overlay_rect);
        let mut close_overlay = false;
        let Self {
            pending,
            down,
            swallowed,
            navigating,
            ..
        } = self;
        raw.events.retain(|event| {
            // Keyboard navigation as it was before this event, which then turns it on or off (R-460, R-464).
            let was = *navigating;
            *navigating = navigation_after(was, event);
            match *event {
                Event::WindowFocused(false) => {
                    down.clear();
                    pending.push(Taken::FocusLost);
                    true
                }
                Event::Key {
                    key,
                    pressed,
                    repeat: system_repeat,
                    modifiers,
                    ..
                } => {
                    let layers = down.contains(&key);
                    let press = Taken::Key(Press {
                        key,
                        modifiers,
                        pressed,
                        reveal: !was,
                    });
                    if !pressed {
                        if layers {
                            down.retain(|k| *k != key);
                            pending.push(press);
                        }
                        return !layers;
                    }
                    if system_repeat {
                        return !layers;
                    }
                    let Some(command) = keymap::command(key, modifiers) else {
                        return true;
                    };
                    if aside || (typing && !keymap::ends_typing(command)) {
                        return true;
                    }
                    if !layers {
                        down.push(key);
                    }
                    pending.push(press);
                    false
                }
                Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    ..
                } => {
                    if pressed {
                        let Some(frame) = overlay else {
                            return true;
                        };
                        close_overlay |= !frame.is_some_and(|r| r.contains(pos));
                        swallowed.push(button);
                        false
                    } else if let Some(i) = swallowed.iter().position(|b| *b == button) {
                        swallowed.remove(i);
                        false
                    } else {
                        true
                    }
                }
                _ => true,
            }
        });
        if close_overlay {
            self.shortcuts_open = false;
        }
    }

    /// The layer is hidden (F3): it forgets the keys it took and holds, so none repeats once it is shown again, and
    /// a frame it does not run asks for no adjustment.
    pub fn stand_down(&mut self) {
        self.adjusted.clear();
        self.scrolled.clear();
        self.typing = None;
        self.pending.clear();
        self.down.clear();
        self.swallowed.clear();
        self.repeat = Repeat::default();
    }

    /// The overlay was drawn in `rect` this frame.
    pub fn overlay_drawn(&mut self, rect: Rect) {
        self.overlay_rect = Some(rect);
    }

    /// Runs the keys taken since the last frame, and the held key's repeats due by `now_s`, on `focus` in `mode`'s
    /// tree; follows a menu egui opened or closed since (the mouse's), and opens or closes the menus as the focus
    /// leaves them. Returns the controls Enter activated; the adjustments are kept for [`Self::adjusted`].
    pub fn run(
        &mut self,
        ctx: &egui::Context,
        mode: Mode,
        focus: &mut Focus,
        now_s: f64,
    ) -> Vec<ScopeId> {
        let now = repeat::millis(now_s);
        self.follow_menus(ctx, mode, focus);
        let mut fired = Vec::new();
        for taken in std::mem::take(&mut self.pending) {
            let press = match taken {
                Taken::FocusLost => {
                    self.repeat = Repeat::default();
                    continue;
                }
                Taken::Key(press) => press,
            };
            if !press.pressed {
                self.repeat.release(press.key);
                continue;
            }
            if let Some(command) = keymap::command(press.key, press.modifiers) {
                // Enter in navigation mode with keyboard navigation off only turns it on and shows the ring where
                // the focus is, so nothing unseen is pressed (R-460 H1, R-464 M2).
                if press.reveal && command == Command::Enter && focus.mode == KeyMode::Navigation {
                    continue;
                }
                fired.push((command, press.modifiers));
                if keymap::repeats(command) {
                    self.repeat.press(press.key, press.modifiers, now);
                }
            }
        }
        if let Some((key, modifiers, n)) = self.repeat.due(now) {
            if let Some(command) = keymap::command(key, modifiers) {
                fired.extend((0..n).map(|_| (command, modifiers)));
            }
        }
        self.adjusted.clear();
        self.scrolled.clear();
        self.left.clear();
        let mut activated = Vec::new();
        for (command, modifiers) in fired {
            if self.shortcuts_open {
                // The overlay is over everything: it takes every key, and Esc or `?` closes it.
                if matches!(command, Command::Back | Command::Shortcuts) {
                    self.shortcuts_open = false;
                }
                continue;
            }
            if command == Command::Shortcuts {
                self.shortcuts_open = true;
                continue;
            }
            let tree = match mode {
                Mode::Explore => &self.explore,
                Mode::Stain => &self.stain,
            };
            let at_menu = self.open_menu.filter(|m| ends_at(&focus.path, m));
            if command == Command::Back && at_menu.is_some() {
                // Esc on an open menu's own control (one the mouse opened with nothing in it) closes it.
                self.open_menu = None;
                continue;
            }
            let multiplier = keymap::multiplier(modifiers);
            match tree.navigate(focus, command, multiplier) {
                Outcome::Activate(id) => {
                    activated.push(id);
                    // An entry acts and its menu closes, the focus back on the menu, as a click on it does.
                    if self.open_menu.is_some() && tree.parent(id) == self.open_menu {
                        focus.path.pop();
                        self.open_menu = None;
                    }
                }
                Outcome::Adjust(adjust) => self.adjusted.push(adjust),
                Outcome::Scroll(scroll) => self.scrolled.push(scroll),
                Outcome::Leave(window) => {
                    if !self.left.contains(&window) {
                        self.left.push(window);
                    }
                }
                Outcome::None | Outcome::Moved => {}
            }
            self.open_menu =
                menu_holding(tree, &focus.path).or(at_menu.filter(|m| ends_at(&focus.path, m)));
        }
        self.show_menus(ctx, mode);
        self.follow_typing(ctx, mode, focus);
        activated
    }

    /// Starts typing in a text scope Enter put in interaction mode, giving its widget egui's focus; ends it when the
    /// focus leaves interaction mode or the scope, taking egui's focus back from the widget, so its next key is the
    /// layer's.
    fn follow_typing(&mut self, ctx: &egui::Context, mode: Mode, focus: &Focus) {
        let tree = self.tree(mode);
        let typing = focus
            .path
            .last()
            .and_then(|id| tree.get(id))
            .filter(|s| s.text && focus.mode == KeyMode::Interaction)
            .map(|s| s.id);
        if typing != self.typing {
            let widget =
                |id: Option<ScopeId>| id.and_then(|id| self.place_of(id)).and_then(|p| p.widget);
            if let Some(ended) = widget(self.typing) {
                ctx.memory_mut(|m| m.surrender_focus(ended));
            }
            if let Some(started) = widget(typing) {
                ctx.memory_mut(|m| m.request_focus(started));
            }
        }
        self.typing = typing;
    }

    /// The popup of menu `id`, drawn from its widget last frame.
    fn popup_of(&self, id: &str) -> Option<Id> {
        self.place_of(id)?.widget.map(|w| w.with("popup"))
    }

    /// A menu egui opened since the last frame (a click on it) takes the focus, on its first entry; one egui closed
    /// (a click on an entry or outside it) gives the focus back to the menu's control.
    fn follow_menus(&mut self, ctx: &egui::Context, mode: Mode, focus: &mut Focus) {
        let tree = self.tree(mode);
        let open = tree
            .menus()
            .into_iter()
            .find(|m| self.popup_of(m).is_some_and(|p| Popup::is_id_open(ctx, p)));
        if open == self.open_menu {
            return;
        }
        match open {
            Some(m) => {
                focus.path = tree.path_to(m);
                focus.mode = KeyMode::Navigation;
                if let Some(first) = tree.children(Some(m)).first() {
                    focus.path.push((*first).to_owned());
                }
            }
            // While a menu is open the focus is in it: run left it there.
            None => {
                if let Some(m) = self.open_menu {
                    focus.path = tree.path_to(m);
                    focus.mode = KeyMode::Navigation;
                }
            }
        }
        self.open_menu = open;
    }

    /// Opens the menu the layer holds open, and closes every other.
    fn show_menus(&self, ctx: &egui::Context, mode: Mode) {
        for m in self.tree(mode).menus() {
            let Some(popup) = self.popup_of(m) else {
                continue;
            };
            let want = self.open_menu == Some(m);
            if want != Popup::is_id_open(ctx, popup) {
                if want {
                    Popup::open_id(ctx, popup);
                } else {
                    Popup::close_id(ctx, popup);
                }
            }
        }
    }

    /// The windows Esc left in the last run, for the app to close ([`Self::close_window`] moves the focus).
    pub fn left(&self) -> &[ScopeId] {
        &self.left
    }

    /// Window `window` opened and takes the focus, in navigation mode, on its landing (its first section's first
    /// element, §G3), `opener` recorded as the path the focus goes back to when it closes. A menu the focus leaves
    /// closes now, so egui's popup never pulls the focus back into it.
    pub fn open_window(
        &mut self,
        ctx: &egui::Context,
        mode: Mode,
        focus: &mut Focus,
        window: ScopeId,
        opener: Vec<String>,
    ) {
        self.openers.retain(|(w, _)| *w != window);
        self.openers.push((window, opener));
        focus.path = self.tree(mode).window_landing(window);
        focus.mode = KeyMode::Navigation;
        self.open_menu = None;
        self.show_menus(ctx, mode);
    }

    /// Window `window` closed: with the focus in it, the focus goes back to the scope that opened it, in navigation
    /// mode, a menu on that path opened again (Windows › Console); otherwise the focus stays where it is.
    pub fn close_window(
        &mut self,
        ctx: &egui::Context,
        mode: Mode,
        focus: &mut Focus,
        window: ScopeId,
    ) {
        let opener = self
            .openers
            .iter()
            .position(|(w, _)| *w == window)
            .map(|i| self.openers.remove(i).1);
        if self.tree(mode).window_of(&focus.path) != Some(window) {
            return;
        }
        focus.path = opener.unwrap_or_default();
        focus.mode = KeyMode::Navigation;
        let tree = self.tree(mode);
        self.open_menu = menu_holding(tree, &focus.path);
        self.show_menus(ctx, mode);
    }

    /// The adjustments the arrows asked for this frame, in order, for the screens that own the values.
    pub fn adjusted(&self) -> &[Adjust] {
        &self.adjusted
    }

    /// The scrolls the keys asked for this frame, in order, for the screens that own the lists (R-450 A4).
    pub fn scrolled(&self) -> &[Scroll] {
        &self.scrolled
    }

    /// A click on control `id` of window `window` moves the focus onto it, so the mouse and the keyboard never
    /// disagree about where the focus is (R-450 A3). A click on a text field starts typing in it: interaction mode,
    /// the ring blue, egui's focus left on the field, every key the field's but Esc, Enter and Tab (R-456; §G3, R-409
    /// G4). A click on any other control lands in navigation mode, the ring orange: typing ends and egui's focus is
    /// released, so the layer takes the keys again (R-456). With the focus outside the window until then (the window
    /// opened by itself, or the focus moved off it), where it was is recorded as the path it goes back to when the
    /// window closes (A2).
    pub fn focus_by_click(
        &mut self,
        ctx: &egui::Context,
        mode: Mode,
        focus: &mut Focus,
        window: ScopeId,
        id: ScopeId,
    ) {
        self.click_onto(ctx, mode, focus, Some(window), id);
    }

    /// Follows the mouse once the frame is drawn (R-456): a pointer press anywhere but on a text field (where egui hits
    /// it: in its clipped rect, its layer on top) lands in navigation mode, the focus where it was (B6; a console
    /// control's click then moves it, R-450 A3), ending any typing or interaction; a text field the mouse gave egui's
    /// focus to (a click on it) is typed in, as [`Self::focus_by_click`] has it; one being typed in that lost egui's
    /// focus otherwise stops typing, navigation mode on the field. egui focuses a text field the layer did not on a
    /// click only: the layer takes Tab and the arrows before egui sees them.
    pub fn follow_text_clicks(&mut self, ctx: &egui::Context, mode: Mode, focus: &mut Focus) {
        let pressed = ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.press_origin()));
        if let Some(at) = pressed.flatten() {
            // Hit as egui hits: inside the field's clipped rect, and on its layer, the one on top at the press.
            let top = ctx.layer_id_at(at);
            let tree = self.tree(mode);
            let on_text = self.places.iter().any(|(id, p)| {
                p.layer.is_some_and(|l| top == Some(l))
                    && p.hit.contains(at)
                    && tree.get(id).is_some_and(|s| s.text)
            });
            if !on_text {
                focus.mode = KeyMode::Navigation;
            }
        }
        let held = ctx.memory(|m| m.focused());
        let tree = self.tree(mode);
        let clicked = self
            .places
            .iter()
            .rev()
            .find(|(id, p)| {
                p.widget.is_some() && p.widget == held && tree.get(id).is_some_and(|s| s.text)
            })
            .map(|(id, _)| *id);
        if let Some(id) = clicked.filter(|id| self.typing != Some(id)) {
            let window = tree.window_of(&tree.path_to(id));
            self.click_onto(ctx, mode, focus, window, id);
            return;
        }
        let lost = self
            .typing
            .and_then(|t| self.place_of(t))
            .and_then(|p| p.widget)
            .is_some_and(|w| held != Some(w));
        if lost {
            self.typing = None;
            focus.mode = KeyMode::Navigation;
        }
    }

    /// Moves the focus onto control `id`, clicked, of window `window`, if it is in one (see [`Self::focus_by_click`]).
    fn click_onto(
        &mut self,
        ctx: &egui::Context,
        mode: Mode,
        focus: &mut Focus,
        window: Option<ScopeId>,
        id: ScopeId,
    ) {
        let tree = self.tree(mode);
        let entered = window.filter(|w| tree.window_of(&focus.path) != Some(*w));
        let (path, text) = (tree.path_to(id), tree.get(id).is_some_and(|s| s.text));
        if let Some(window) = entered {
            self.openers.retain(|(w, _)| *w != window);
            self.openers.push((window, focus.path.clone()));
        }
        focus.path = path;
        let widget = self.place_of(id).and_then(|p| p.widget);
        match widget.filter(|_| text) {
            Some(field) => {
                focus.mode = KeyMode::Interaction;
                self.typing = Some(id);
                ctx.memory_mut(|m| m.request_focus(field));
            }
            None => {
                focus.mode = KeyMode::Navigation;
                self.typing = None;
                if let Some(held) = ctx.memory(|m| m.focused()) {
                    ctx.memory_mut(|m| m.surrender_focus(held));
                }
            }
        }
        self.open_menu = None;
        self.show_menus(ctx, mode);
    }

    /// The time the held key's next repeat falls due, in seconds of input time, while one is held.
    pub fn next_repeat_s(&self) -> Option<f64> {
        self.repeat.next_due_ms().map(|ms| ms as f64 / 1000.0)
    }

    /// Forgets last frame's places, before the frame draws its scopes.
    pub fn clear_places(&mut self) {
        self.places.clear();
    }

    /// Scope `id` was drawn in `rect`.
    pub fn place(&mut self, id: ScopeId, rect: Rect) {
        self.places.push((
            id,
            Place {
                rect,
                widget: None,
                hit: rect,
                layer: None,
            },
        ));
    }

    /// Scope `id` is the widget of `response`.
    pub fn place_widget(&mut self, id: ScopeId, response: &Response) {
        self.places.push((
            id,
            Place {
                rect: response.rect,
                widget: Some(response.id),
                hit: response.interact_rect,
                layer: Some(response.layer_id),
            },
        ));
    }

    /// Where scope `id` was last drawn.
    pub fn place_of(&self, id: &str) -> Option<Place> {
        self.places
            .iter()
            .rev()
            .find(|(i, _)| *i == id)
            .map(|(_, p)| *p)
    }
}

/// Keyboard navigation after `event`, from `navigating` before it: any key press turns it on, a modifier pressed alone
/// being no event of egui's (Cmd+C, X and V come as their own events, a character with no key of egui's as its text);
/// any pointer press, the start of a click or a drag, turns it off; everything else, a scroll and a hover among them,
/// leaves it (R-460 H1, R-464 M2, M3).
fn navigation_after(navigating: bool, event: &Event) -> bool {
    match event {
        Event::Key { pressed: true, .. }
        | Event::Text(_)
        | Event::Copy
        | Event::Cut
        | Event::Paste(_) => true,
        Event::PointerButton { pressed: true, .. } => false,
        _ => navigating,
    }
}

/// Whether `path` ends at scope `id`.
fn ends_at(path: &[String], id: &str) -> bool {
    path.last().is_some_and(|l| l == id)
}

/// The menu the focus is in: the innermost menu on `path` with one of its entries below it.
fn menu_holding(tree: &ScopeTree, path: &[String]) -> Option<ScopeId> {
    let above = path.len().saturating_sub(1);
    path[..above]
        .iter()
        .rev()
        .find_map(|id| tree.get(id).filter(|s| s.menu).map(|s| s.id))
}
