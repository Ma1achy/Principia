//! The global key table (render_gui_spec §G3): which key, with which modifiers, is which command; the steps Shift and
//! Alt give; which commands repeat while held; and the rows the `?` overlay shows.

use eframe::egui::{Key, Modifiers};

/// An arrow's direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// ↑.
    Up,
    /// ↓.
    Down,
    /// ←.
    Left,
    /// →.
    Right,
}

impl Direction {
    /// The sign it adjusts a value by: ↑ and → raise it, ↓ and ← lower it.
    pub fn sign(self) -> f64 {
        match self {
            Direction::Up | Direction::Right => 1.0,
            Direction::Down | Direction::Left => -1.0,
        }
    }
}

/// A global command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Tab: the next big scope.
    Next,
    /// Shift+Tab: the previous big scope.
    Previous,
    /// Enter: into the focused scope.
    Enter,
    /// Esc: back out one level.
    Back,
    /// An arrow: between siblings, or adjust a focused value.
    Arrow(Direction),
    /// `?`: the shortcuts, over everything.
    Shortcuts,
}

/// Whether `m` holds Ctrl or Cmd, which the table leaves to the system and to Ctrl+Z.
fn command_held(m: Modifiers) -> bool {
    m.ctrl || m.command || m.mac_cmd
}

/// The command `key` with `modifiers` is, if any.
pub fn command(key: Key, modifiers: Modifiers) -> Option<Command> {
    if command_held(modifiers) {
        return None;
    }
    let plain = !modifiers.shift && !modifiers.alt;
    match key {
        Key::Tab if plain => Some(Command::Next),
        Key::Tab if modifiers.shift && !modifiers.alt => Some(Command::Previous),
        Key::Enter if plain => Some(Command::Enter),
        Key::Escape if plain => Some(Command::Back),
        Key::ArrowUp => Some(Command::Arrow(Direction::Up)),
        Key::ArrowDown => Some(Command::Arrow(Direction::Down)),
        Key::ArrowLeft => Some(Command::Arrow(Direction::Left)),
        Key::ArrowRight => Some(Command::Arrow(Direction::Right)),
        // `?` is Shift+/ on most layouts, so Shift is allowed.
        Key::Questionmark if !modifiers.alt => Some(Command::Shortcuts),
        _ => None,
    }
}

/// Shift's ×10 and Alt's ×0.1; both together, ×1.
pub fn multiplier(modifiers: Modifiers) -> f64 {
    let shift = if modifiers.shift { 10.0 } else { 1.0 };
    let alt = if modifiers.alt { 0.1 } else { 1.0 };
    shift * alt
}

/// Whether a held key repeats its command: Tab, Shift+Tab and the arrows do; Enter, Esc and `?` act once a press.
pub fn repeats(command: Command) -> bool {
    matches!(
        command,
        Command::Next | Command::Previous | Command::Arrow(_)
    )
}

/// The `?` overlay's global rows: render_gui_spec §G3's table, key and action, then F3, which hides the layer
/// (§G1, §G2's "F3 hide"). A screen adds its own rows through its tree's registration
/// ([`ScopeTree::register_shortcut`](crate::keyboard::scopes::ScopeTree::register_shortcut)).
pub const SHORTCUTS: [(&str, &str); 9] = [
    (
        "Tab / Shift+Tab",
        "next / previous big scope, in the numbered order",
    ),
    ("Enter", "into the focused scope"),
    ("Esc", "back out one level"),
    ("arrows", "between siblings; adjust a focused value"),
    ("Shift · Alt", "×10 · ×0.1 steps"),
    ("held keys", "delay, then repeat (the DAS / ARR model)"),
    ("Ctrl+Z", "undo, from the contract's history"),
    ("?", "shortcuts, over everything"),
    ("F3", "hide / show the egui layer"),
];
