//! Keyboard navigation (render_gui_spec §G3; GUI_DESIGN_NOTES § "07 Keyboard"; R-390, ORDER item 2): the tree of
//! scopes each screen joins, the global key table, held keys' delay and repeat, and the `?` shortcuts overlay. The
//! focus is `ViewUI`'s (gui_state_contract §2); what the user sees of it is the focus ring and the top bar's
//! breadcrumb (`explore::breadcrumb`), nothing else.
//!
//! The layer takes its keys from the raw input before egui sees them, so egui's own Tab and arrow focus never moves;
//! it stands aside while an egui widget holds the keyboard (a text field) or a menu is open, which then has the keys.

pub mod keymap;
pub mod overlay;
pub mod repeat;
pub mod scopes;

use eframe::egui::{self, Event, Id, Key, Modifiers, RawInput, Rect, Response};
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

/// The keyboard layer.
pub struct Keyboard {
    explore: ScopeTree,
    stain: ScopeTree,
    repeat: Repeat,
    pending: Vec<Press>,
    places: Vec<(ScopeId, Place)>,
    /// Whether the `?` shortcuts overlay is open.
    pub shortcuts_open: bool,
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
            places: Vec::new(),
            shortcuts_open: false,
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

    /// Takes the table's keys out of `raw`, before egui's pass sees them. While an egui widget holds the keyboard or
    /// a menu is open (and the overlay is closed), it leaves them to egui, noting only the held key's release. The
    /// system's own repeats of the table's keys are dropped: the layer repeats them itself.
    pub fn take_keys(&mut self, ctx: &egui::Context, raw: &mut RawInput) {
        let aside = !self.shortcuts_open
            && (ctx.memory(|m| m.focused().is_some()) || egui::Popup::is_any_open(ctx));
        let pending = &mut self.pending;
        let repeat = &self.repeat;
        raw.events.retain(|event| {
            let Event::Key {
                key,
                pressed,
                repeat: system_repeat,
                modifiers,
                ..
            } = *event
            else {
                return true;
            };
            let ours = keymap::command(key, modifiers).is_some() || repeat.holds(key);
            if !ours {
                return true;
            }
            if aside {
                if !pressed {
                    pending.push(Press {
                        key,
                        modifiers,
                        pressed,
                    });
                }
                return true;
            }
            if !system_repeat {
                pending.push(Press {
                    key,
                    modifiers,
                    pressed,
                });
            }
            false
        });
    }

    /// Runs the keys taken since the last frame, and the held key's repeats due by `now_s`, on `focus` in `mode`'s
    /// tree. Returns the controls Enter activated; the adjustments are kept for [`Self::adjusted`].
    pub fn run(&mut self, mode: Mode, focus: &mut Focus, now_s: f64) -> Vec<ScopeId> {
        let now = repeat::millis(now_s);
        let mut fired = Vec::new();
        for press in std::mem::take(&mut self.pending) {
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
