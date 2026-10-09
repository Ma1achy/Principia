//! Keyboard navigation (render_gui_spec §G3; GUI_DESIGN_NOTES § "07 Keyboard"; R-390, ORDER item 2): the tree of
//! scopes each screen joins, the global key table, held keys' delay and repeat, and the `?` shortcuts overlay. The
//! focus is `ViewUI`'s (gui_state_contract §2); what the user sees of it is the focus ring and the top bar's
//! breadcrumb (`explore::breadcrumb`), nothing else.
//!
//! The layer takes its keys from the raw input before egui sees them, so egui's own Tab and arrow focus never moves;
//! it stands aside while an egui widget holds the keyboard (a text field) or a menu is open, which then has the keys.
//! A key's release goes to whoever saw its press.

pub mod keymap;
pub mod overlay;
pub mod repeat;
pub mod scopes;

use eframe::egui::{self, Event, Id, Key, Modifiers, PointerButton, RawInput, Rect, Response};
use engine::contract::view_ui::{Focus, Mode};

use keymap::Command;
use repeat::Repeat;
use scopes::{Adjust, Outcome, ScopeId, ScopeTree};

/// Where a scope was drawn this frame: its rect, and the egui widget it is, for a control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// Its rect, in points.
    pub rect: Rect,
    /// The widget's id, for a control drawn as one.
    pub widget: Option<Id>,
}

/// A key event the layer took.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Press {
    key: Key,
    modifiers: Modifiers,
    pressed: bool,
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
    adjusted: Vec<Adjust>,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Keyboard {
    /// The layer, each mode's screens registered: Explore's seven big scopes in §G3's Tab order; on the Stain page,
    /// the top bar, until TASK-M6-29 adds Stain's scopes.
    pub fn new() -> Self {
        let mut explore = ScopeTree::new();
        crate::explore::top_bar::register(&mut explore);
        crate::explore::register(&mut explore);
        let mut stain = ScopeTree::new();
        crate::explore::top_bar::register(&mut stain);
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
            adjusted: Vec::new(),
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

    /// Takes the table's keys out of `raw`, before egui's pass sees them, and the release of each key whose press it
    /// took; the rest, and every key while an egui widget holds the keyboard or a menu is open (and the overlay is
    /// closed), are egui's.
    /// The system's own repeats go with their press: the layer drops its own, as it repeats them itself. A lost
    /// window focus forgets the held keys. While the overlay is open it takes every click: a press outside its frame
    /// closes it.
    pub fn take_keys(&mut self, ctx: &egui::Context, raw: &mut RawInput) {
        let aside = !self.shortcuts_open
            && (ctx.memory(|m| m.focused().is_some()) || egui::Popup::is_any_open(ctx));
        let overlay = self.shortcuts_open.then_some(self.overlay_rect);
        let mut close_overlay = false;
        let Self {
            pending,
            down,
            swallowed,
            ..
        } = self;
        raw.events.retain(|event| match *event {
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
                if aside || keymap::command(key, modifiers).is_none() {
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
        });
        if close_overlay {
            self.shortcuts_open = false;
        }
    }

    /// The layer is hidden (F3): it forgets the keys it took and holds, so none repeats once it is shown again.
    pub fn stand_down(&mut self) {
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
    /// tree. Returns the controls Enter activated; the adjustments are kept for [`Self::adjusted`].
    pub fn run(&mut self, mode: Mode, focus: &mut Focus, now_s: f64) -> Vec<ScopeId> {
        let now = repeat::millis(now_s);
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
            match tree.navigate(&mut focus.path, command, keymap::multiplier(modifiers)) {
                Outcome::Activate(id) => activated.push(id),
                Outcome::Adjust(adjust) => self.adjusted.push(adjust),
                Outcome::None | Outcome::Moved => {}
            }
        }
        activated
    }

    /// The adjustments the arrows asked for this frame, in order, for the screens that own the values.
    pub fn adjusted(&self) -> &[Adjust] {
        &self.adjusted
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
        self.places.push((id, Place { rect, widget: None }));
    }

    /// Scope `id` is the widget of `response`.
    pub fn place_widget(&mut self, id: ScopeId, response: &Response) {
        self.places.push((
            id,
            Place {
                rect: response.rect,
                widget: Some(response.id),
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
