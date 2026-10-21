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
    /// Tab: the next big scope, in navigation mode.
    Next,
    /// Shift+Tab: the previous big scope, in navigation mode.
    Previous,
    /// Enter: in navigation mode, into a scope, a control's click or interaction with an element; in interaction
    /// mode, back to navigation mode.
    Enter,
    /// Esc: in interaction mode, back to navigation mode; in navigation mode, up one scope.
    Back,
    /// An arrow: in navigation mode between siblings; in interaction mode, adjust the focused element.
    Arrow(Direction),
    /// `?`: the shortcuts, over everything.
    Shortcuts,
    /// Page Up, Page Down, Home and End: in interaction mode, scroll a list the keyboard scrolls (the console's log,
    /// R-450); elsewhere nothing.
    Scroll(ScrollKey),
}

/// A key that scrolls a list by a page or to one of its ends (R-450, A4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollKey {
    /// Page Up: a page towards the first entry.
    PageUp,
    /// Page Down: a page towards the last entry.
    PageDown,
    /// Home: to the first entry.
    Home,
    /// End: to the last entry.
    End,
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
        Key::PageUp => Some(Command::Scroll(ScrollKey::PageUp)),
        Key::PageDown => Some(Command::Scroll(ScrollKey::PageDown)),
        Key::Home => Some(Command::Scroll(ScrollKey::Home)),
        Key::End => Some(Command::Scroll(ScrollKey::End)),
        // `?` is Shift+/ on most layouts, so Shift is allowed.
        Key::Questionmark if !modifiers.alt => Some(Command::Shortcuts),
        _ => None,
    }
}

/// Whether `key` with `modifiers` is Ctrl+Z (Cmd+Z on macOS): undo from the contract's history, in either mode (§G3;
/// R-409 G5). It is no command of the layer's: the app reads it from egui's input, so a text field being typed in
/// keeps it for its own edit. Shift or Alt with it is not undo.
pub fn undoes(key: Key, modifiers: Modifiers) -> bool {
    key == Key::Z && command_held(modifiers) && !modifiers.shift && !modifiers.alt
}

/// Shift's ×10 and Alt's ×0.1; both together, ×1.
pub fn multiplier(modifiers: Modifiers) -> f64 {
    let shift = if modifiers.shift { 10.0 } else { 1.0 };
    let alt = if modifiers.alt { 0.1 } else { 1.0 };
    shift * alt
}

/// Whether `command` is one the layer keeps while a text field is typed in: Esc and Enter end the typing, back to
/// navigation mode, and Tab and Shift+Tab move on (R-409 G4); every other key goes to the field.
pub fn ends_typing(command: Command) -> bool {
    matches!(
        command,
        Command::Back | Command::Enter | Command::Next | Command::Previous
    )
}

/// Whether a held key repeats its command: Tab, Shift+Tab, the arrows, Page Up and Page Down do; Enter, Esc, `?`,
/// Home and End act once a press.
pub fn repeats(command: Command) -> bool {
    matches!(
        command,
        Command::Next
            | Command::Previous
            | Command::Arrow(_)
            | Command::Scroll(ScrollKey::PageUp | ScrollKey::PageDown)
    )
}

/// The `?` overlay's global rows: render_gui_spec §G3's table, key and action, then F3, which hides the layer
/// (§G1, §G2's "F3 hide"). A screen adds its own rows through its tree's registration
/// ([`ScopeTree::register_shortcut`](crate::keyboard::scopes::ScopeTree::register_shortcut)).
pub const SHORTCUTS: [(&str, &str); 10] = [
    (
        "Tab / Shift+Tab",
        "next / previous big scope, in the numbered order, in navigation mode",
    ),
    (
        "Enter",
        "into a scope; interact with an element (blue); a button's click; again, back to navigating (orange)",
    ),
    ("Esc", "interacting: back to navigating; navigating: up one scope"),
    (
        "arrows",
        "navigating: between siblings; interacting: adjust the element (they need Enter)",
    ),
    ("Shift · Alt", "×10 · ×0.1 steps, the compass included"),
    (
        "letters, Space",
        "a scope's shortcuts, in either mode, whenever the focus is in it",
    ),
    ("held keys", "delay, then repeat (the DAS / ARR model)"),
    ("Ctrl+Z", "undo, from the contract's history, in either mode"),
    ("?", "shortcuts, over everything, in either mode"),
    ("F3", "hide / show the egui layer"),
];
