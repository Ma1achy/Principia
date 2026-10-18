//! The reachability check (R-450 A7; REQ-GUI-190), beside `support`: a scope or a window walked with §G3's keys alone,
//! from where the focus lands on it, and every control drawn in it reached. Reusable: the console's tests and the
//! audit of every other window and big scope run it.
//!
//! - [`walk`] runs §G3's keys (↑ ↓ ← →, Tab, Shift+Tab, Enter, Esc, Page Up, Page Down, Home, End) on the tree from a
//!   focus, every key from every focus reached, breadth first, and returns the scopes reached in the order first
//!   reached: the order the keys give, which must not depend on state (A5). A key that leaves the walked scope (Esc
//!   out of a window, Tab out of a big scope) is not followed.
//! - [`drawn_controls`] reads the controls a frame draws from its AccessKit tree, by role: the buttons, fields, sliders,
//!   checkboxes and the like, never a label.
//! - [`unreached`] is the controls no reached scope covers: a control is reached when its centre is inside the rect a
//!   reached scope was placed on this frame, the scope drawn as that widget (a menu, an edit… button with values
//!   under it) or with nothing under it (an element, or a section of one element). A section of several elements
//!   covers none of them: each must be reached itself.

use std::collections::VecDeque;

use eframe::egui::accesskit::Role;
use eframe::egui::{Key, Modifiers, Rect};
use engine::contract::view_ui::{Focus, KeyMode};

use super::support::Node;
use crate::keyboard::keymap::{self, Command};
use crate::keyboard::scopes::{Outcome, ScopeTree};
use crate::keyboard::Place;

/// §G3's keys, each with no modifier: the walk's alphabet.
pub const KEYS: [Key; 12] = [
    Key::ArrowUp,
    Key::ArrowDown,
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::Tab,
    Key::Tab,
    Key::Enter,
    Key::Escape,
    Key::PageUp,
    Key::PageDown,
    Key::Home,
    Key::End,
];

/// The commands [`KEYS`] give, the second Tab with Shift.
fn commands() -> Vec<Command> {
    KEYS.iter()
        .enumerate()
        .map(|(i, key)| {
            let modifiers = if i == 5 {
                Modifiers::SHIFT
            } else {
                Modifiers::NONE
            };
            keymap::command(*key, modifiers).expect("a key of the table")
        })
        .collect()
}

/// The scopes the keys reach from `start`, staying under `within` (the first scope of every path kept), in the order
/// first reached, breadth first.
pub fn walk(tree: &ScopeTree, start: &Focus, within: &str) -> Vec<String> {
    let mut seen: Vec<(Vec<String>, KeyMode)> = vec![(start.path.clone(), start.mode)];
    let mut reached: Vec<String> = start.path.last().cloned().into_iter().collect();
    let mut queue = VecDeque::from([start.clone()]);
    let commands = commands();
    while let Some(focus) = queue.pop_front() {
        for command in &commands {
            let mut next = focus.clone();
            if let Outcome::Leave(_) = tree.navigate(&mut next, *command, 1.0) {
                continue;
            }
            if next.path.first().map(String::as_str) != Some(within) {
                continue;
            }
            let state = (next.path.clone(), next.mode);
            if seen.contains(&state) {
                continue;
            }
            seen.push(state);
            if let Some(last) = next.path.last() {
                if !reached.contains(last) {
                    reached.push(last.clone());
                }
            }
            queue.push_back(next);
        }
    }
    reached
}

/// Whether `role` is a control's: something the pointer acts on.
pub fn is_control(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::DefaultButton
            | Role::CheckBox
            | Role::RadioButton
            | Role::Switch
            | Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::SpinButton
            | Role::Slider
            | Role::ComboBox
            | Role::EditableComboBox
            | Role::Link
            | Role::MenuItem
            | Role::MenuItemCheckBox
            | Role::MenuItemRadio
            | Role::Tab
    )
}

/// The controls among `nodes` whose centre is inside `rect`: those drawn there.
pub fn drawn_controls(nodes: &[Node], rect: Rect) -> Vec<Node> {
    nodes
        .iter()
        .filter(|n| is_control(n.role) && rect.contains(n.rect.center()))
        .cloned()
        .collect()
}

/// The controls of `after` not in `before`, by role, rect and text: what a window opening drew.
pub fn new_controls(before: &[Node], after: &[Node]) -> Vec<Node> {
    let same = |a: &Node, b: &Node| a.role == b.role && a.rect == b.rect && a.text == b.text;
    after
        .iter()
        .filter(|n| is_control(n.role) && !before.iter().any(|b| same(b, n)))
        .cloned()
        .collect()
}

/// The controls among `controls` that no scope of `reached` covers, its place this frame's `place`: a scope drawn as a
/// widget, or one with nothing under it in `tree`.
pub fn unreached(
    tree: &ScopeTree,
    reached: &[String],
    place: impl Fn(&str) -> Option<Place>,
    controls: &[Node],
) -> Vec<Node> {
    let rects: Vec<Rect> = reached
        .iter()
        .filter_map(|id| place(id).map(|p| (id, p)))
        .filter(|(id, p)| p.widget.is_some() || tree.children(Some(id)).is_empty())
        .map(|(_, p)| p.rect)
        .collect();
    controls
        .iter()
        .filter(|c| {
            !rects
                .iter()
                .any(|r| r.expand(0.5).contains(c.rect.center()))
        })
        .cloned()
        .collect()
}

/// Asserts that every one of `controls` is covered by a scope of `reached`: names each one that is not.
pub fn check_reached(
    what: &str,
    tree: &ScopeTree,
    reached: &[String],
    place: impl Fn(&str) -> Option<Place>,
    controls: &[Node],
) {
    let missed = unreached(tree, reached, place, controls);
    assert!(
        missed.is_empty(),
        "{what}: drawn controls no key reaches: {:?}",
        missed
            .iter()
            .map(|n| (n.role, n.text.as_str(), n.rect))
            .collect::<Vec<_>>()
    );
}

/// Asserts that two walks reached the same scopes in the same order: reachability that does not depend on state.
pub fn check_same_order(what: &str, a: &[String], b: &[String]) {
    assert_eq!(a, b, "{what}: the order the keys give depends on state");
}
