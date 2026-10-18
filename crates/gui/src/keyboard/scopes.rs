//! The tree of keyboard scopes (render_gui_spec §G3; GUI_DESIGN_NOTES § "07 Keyboard"; R-390): the big scopes in Tab
//! order, each with the sub-scopes Enter reaches. Each screen joins the tree through [`ScopeTree::register`], in the
//! order it draws its scopes; the focus is a path through it, held in `ViewUI` (gui_state_contract §2), with the
//! keyboard's mode, navigation or interaction (R-409; [`ScopeTree::navigate`]).
//!
//! The base steps (REQ-GUI-158) are calibrations (R-71), proposed by TASK-M6-25 and confirmed by R-404, so no longer
//! provisional (R-182): [`StepKind::base`]; R-409 retired R-404's `Orbit` step, the compass having no orbit. The
//! compass's slice-mode step (REQ-GUI-184) is proposed by TASK-M6-31 and used provisionally until the M8 gate.

use engine::contract::view_ui::{Focus, KeyMode};

use crate::keyboard::keymap::{Command, Direction, ScrollKey};

/// A scope's id: unique in its tree, and what `ViewUI`'s focus path holds.
pub type ScopeId = &'static str;

/// What the arrows do on a scope in interaction mode; in navigation mode they always move between siblings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrows {
    /// Nothing: the scope has no interaction that takes them.
    Siblings,
    /// Adjust the focused value by steps of this kind; Enter on it starts interaction mode.
    Adjust(StepKind),
    /// Scroll the focused list by entries (↑ ↓), pages (Page Up, Page Down) and to its ends (Home, End); Enter on it
    /// starts interaction mode (the console's log, R-450 A4).
    Scroll,
}

/// Which arrows move between a scope's children in navigation mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Every arrow: ↑ and ← to the previous, ↓ and → to the next.
    Both,
    /// ← and → only: a row (the console's header bar, R-450 A1).
    Row,
    /// ↑ and ↓ only: a column (the console's two scopes, R-450 A1).
    Column,
}

impl Axis {
    /// Whether the arrow `direction` moves along it.
    pub fn moves(self, direction: Direction) -> bool {
        match self {
            Axis::Both => true,
            Axis::Row => matches!(direction, Direction::Left | Direction::Right),
            Axis::Column => matches!(direction, Direction::Up | Direction::Down),
        }
    }
}

/// Where the focus ring is drawn on a scope's rect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ring {
    /// Inside the rect.
    Inside,
    /// Outside it: the figure's, so the ring covers none of the figure (render_gui_spec §G1).
    Outside,
}

/// One scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope {
    /// Its id.
    pub id: ScopeId,
    /// Its name in the breadcrumb.
    pub label: &'static str,
    /// What the arrows do on it.
    pub arrows: Arrows,
    /// Whether Enter on it, with no sub-scope to go into, activates it as a click would.
    pub activates: bool,
    /// Where its ring is drawn.
    pub ring: Ring,
    /// Whether it is a menu: its sub-scopes are the entries of a popup drawn from its widget, open while the focus is
    /// in it.
    pub menu: bool,
    /// Whether it is a text field: Enter on it starts typing, interaction mode, in which every key but Esc, Enter and
    /// Tab goes to the field (R-409 G4, G5).
    pub text: bool,
    /// Whether it is a window (render_gui_spec §G3, §G5; R-405): registered at the top level but outside the big
    /// scopes' Tab order, reached only by opening it. With the focus inside it, Tab and Shift+Tab move between its
    /// sections and stay in it, and Esc in navigation mode on a section leaves it ([`Outcome::Leave`]).
    pub window: bool,
    /// For a window, whether its sections are scopes the focus lands on themselves, opening and on Tab, Enter going
    /// into one (the console's header bar and log, R-450); otherwise a section's landing is its first element.
    pub scopes: bool,
    /// Which arrows move between its children in navigation mode.
    pub axis: Axis,
}

impl Scope {
    /// A scope the arrows move away from, which Enter goes into.
    pub const fn group(id: ScopeId, label: &'static str) -> Self {
        Self {
            id,
            label,
            arrows: Arrows::Siblings,
            activates: false,
            ring: Ring::Inside,
            menu: false,
            text: false,
            window: false,
            scopes: false,
            axis: Axis::Both,
        }
    }

    /// A menu: Enter goes into its entries, registered under it, and opens it; Esc closes it, back to it.
    pub const fn menu(id: ScopeId, label: &'static str) -> Self {
        Self {
            menu: true,
            ..Self::group(id, label)
        }
    }

    /// A control drawn disabled: Enter does nothing on it.
    pub const fn disabled(id: ScopeId, label: &'static str) -> Self {
        Self::group(id, label)
    }

    /// A control that Enter activates.
    pub const fn control(id: ScopeId, label: &'static str) -> Self {
        Self {
            activates: true,
            ..Self::group(id, label)
        }
    }

    /// A window: its sections are registered under it, and its elements under them (render_gui_spec §G3, §G5).
    pub const fn window(id: ScopeId, label: &'static str) -> Self {
        Self {
            window: true,
            ..Self::group(id, label)
        }
    }

    /// A window whose sections are scopes, landed on themselves, the arrows moving between them on ↑ ↓ (R-450).
    pub const fn scoped_window(id: ScopeId, label: &'static str) -> Self {
        Self {
            scopes: true,
            axis: Axis::Column,
            ..Self::window(id, label)
        }
    }

    /// A list the keys scroll in interaction mode, after Enter on it (R-450 A4).
    pub const fn scroll(id: ScopeId, label: &'static str) -> Self {
        Self {
            arrows: Arrows::Scroll,
            ..Self::group(id, label)
        }
    }

    /// The scope with its children in a row: ← → move between them, ↑ ↓ do nothing (R-450 A1).
    pub const fn row(self) -> Self {
        Self {
            axis: Axis::Row,
            ..self
        }
    }

    /// A text field: Enter on it starts typing (R-409 G4).
    pub const fn text(id: ScopeId, label: &'static str) -> Self {
        Self {
            text: true,
            ..Self::group(id, label)
        }
    }

    /// A value the arrows adjust by steps of `kind`, in interaction mode.
    pub const fn value(id: ScopeId, label: &'static str, kind: StepKind) -> Self {
        Self {
            arrows: Arrows::Adjust(kind),
            ..Self::group(id, label)
        }
    }

    /// The scope with its ring drawn outside its rect.
    pub const fn ring_outside(self) -> Self {
        Self {
            ring: Ring::Outside,
            ..self
        }
    }
}

/// The kinds of adjustable field and in-scope arrow action, each with its base step (REQ-GUI-158).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepKind {
    /// A bounded field on a slider: a `z₀` value, the slice step, the playback speed.
    Bounded,
    /// An angle field: the tilt angles `τ₁`, `τ₂` and the rotation `γ`; and the Trajectory panel's sphere's orbit
    /// (R-453 D6).
    Angle,
    /// The zoom, in log₂: Navigate's, and the Trajectory panel's views' + and − (R-453 D6).
    ZoomLog2,
    /// Figure's arrows: pan the view; and the Trajectory panel's real-space and unwrapped views' (R-453 D6).
    Pan,
    /// Compass's arrows in tilt mode: tilt the slice plane.
    Tilt,
    /// Compass's arrows in slice mode: move the slice plane along its normal (R-409 F3; REQ-GUI-184).
    SliceNormal,
    /// Time's ← →: step the playhead.
    TimeStep,
}

/// A base step: absolute in the field's own unit, or relative to the field's range or to the view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Base {
    /// Degrees.
    Degrees(f64),
    /// Octaves of zoom (log₂ units).
    Log2(f64),
    /// A fraction of the field's range.
    OfRange(f64),
    /// A fraction of the view's span along the arrow's axis.
    OfView(f64),
}

impl Base {
    /// The step `times` over.
    pub fn times(self, times: f64) -> Self {
        match self {
            Base::Degrees(v) => Base::Degrees(v * times),
            Base::Log2(v) => Base::Log2(v * times),
            Base::OfRange(v) => Base::OfRange(v * times),
            Base::OfView(v) => Base::OfView(v * times),
        }
    }
}

impl StepKind {
    /// The base step, before Shift's ×10 or Alt's ×0.1: confirmed by R-404 (REQ-GUI-158; R-71), the reasoning in the
    /// PR of TASK-M6-25; R-409 retired its `Orbit` step. `SliceNormal`'s is REQ-GUI-184's, proposed by TASK-M6-31 and
    /// used provisionally (R-182) until the human confirms it at the M8 gate.
    pub const fn base(self) -> Base {
        match self {
            // A hundredth of the range: Shift crosses a tenth, Alt a thousandth.
            StepKind::Bounded | StepKind::TimeStep => Base::OfRange(0.01),
            // A hundredth of the slice step's range, `[−1, 1]`: untilted, one press moves the slice as far as one
            // press on the slice step's slider does.
            StepKind::SliceNormal => Base::OfRange(0.01),
            // A degree: Shift ten, Alt a tenth.
            StepKind::Angle | StepKind::Tilt => Base::Degrees(1.0),
            // A quarter octave: four presses double the zoom, Shift is 2.5 octaves.
            StepKind::ZoomLog2 => Base::Log2(0.25),
            // A twentieth of the view: Shift moves half a view, keeping half of it in sight.
            StepKind::Pan => Base::OfView(0.05),
        }
    }
}

/// An adjustment the arrows asked of a focused value; the screen that owns the value applies it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Adjust {
    /// The value's scope.
    pub scope: ScopeId,
    /// Its kind.
    pub kind: StepKind,
    /// The signed multiple of the base step: ±1, ±10 with Shift, ±0.1 with Alt.
    pub times: f64,
    /// The arrow pressed: a value with two axes (the figure's pan, the compass's tilt) reads its axis from it.
    pub direction: Direction,
}

impl Adjust {
    /// The step to apply.
    pub fn delta(&self) -> Base {
        self.kind.base().times(self.times)
    }
}

/// How far a scroll moves a list (R-450 A4): positive towards its last entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollBy {
    /// This many entries.
    Entries(i64),
    /// This many pages, a page the entries the list's rect shows.
    Pages(i64),
    /// To the first entry.
    First,
    /// To the last entry.
    Last,
}

/// A scroll the keys asked of a focused list; the screen that owns the list applies it, clamped to its ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scroll {
    /// The list's scope.
    pub scope: ScopeId,
    /// How far.
    pub by: ScrollBy,
}

/// The count a step of `sign` moves with `multiplier`: Shift's ten; one with no modifier, and with Alt, an entry or
/// a page being the smallest step (R-450 A4).
pub fn scroll_count(sign: f64, multiplier: f64) -> i64 {
    let n = if multiplier > 1.0 { 10 } else { 1 };
    if sign < 0.0 {
        -n
    } else {
        n
    }
}

/// What one command did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// Nothing.
    None,
    /// The focus moved, or its mode changed.
    Moved,
    /// Enter activated this control.
    Activate(ScopeId),
    /// The arrows adjusted a value.
    Adjust(Adjust),
    /// The keys scrolled a list.
    Scroll(Scroll),
    /// Esc in navigation mode at this window's top level, on one of its sections: the window is left, closed, and the
    /// focus goes back to the scope that opened it (render_gui_spec §G3, §G5; R-405). The focus is left as it was for
    /// the layer to move.
    Leave(ScopeId),
}

struct Node {
    scope: Scope,
    parent: Option<usize>,
}

/// A row of the `?` overlay: a key and its action.
pub type Shortcut = (&'static str, &'static str);

/// A tree of scopes, its children in registration order, and the rows its screens add to the `?` overlay.
#[derive(Default)]
pub struct ScopeTree {
    nodes: Vec<Node>,
    shortcuts: Vec<Shortcut>,
}

impl ScopeTree {
    /// An empty tree.
    pub fn new() -> Self {
        Self::default()
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.scope.id == id)
    }

    /// Adds `scope` as the last child of `parent`, or as the last big scope for `None`.
    ///
    /// # Panics
    /// When the id is already in the tree, or `parent` is not: a registration error, caught by the screen's tests.
    pub fn register(&mut self, parent: Option<ScopeId>, scope: Scope) {
        assert!(
            self.index(scope.id).is_none(),
            "the scope `{}` is registered twice",
            scope.id
        );
        let parent = parent.map(|p| {
            self.index(p)
                .unwrap_or_else(|| panic!("no scope `{p}` to register `{}` under", scope.id))
        });
        self.nodes.push(Node { scope, parent });
    }

    /// Removes every scope below `parent`, keeping `parent`: for a screen whose elements come and go (the kept
    /// orbits), which registers them again. A focus path into a removed scope is cut back at the next command.
    pub fn remove_children(&mut self, parent: &str) {
        let Some(root) = self.index(parent) else {
            return;
        };
        let below = |mut i: usize, nodes: &[Node]| loop {
            match nodes[i].parent {
                Some(p) if p == root => return true,
                Some(p) => i = p,
                None => return false,
            }
        };
        let keep: Vec<bool> = (0..self.nodes.len())
            .map(|i| !below(i, &self.nodes))
            .collect();
        let mut new_index = vec![None; self.nodes.len()];
        let mut next = 0;
        for (i, k) in keep.iter().enumerate() {
            if *k {
                new_index[i] = Some(next);
                next += 1;
            }
        }
        let nodes = std::mem::take(&mut self.nodes);
        self.nodes = nodes
            .into_iter()
            .zip(keep)
            .filter_map(|(node, k)| {
                k.then(|| Node {
                    scope: node.scope,
                    parent: node.parent.and_then(|p| new_index[p]),
                })
            })
            .collect();
    }

    /// Adds `scope` as the sibling right after scope `after`, under the same parent: for an element that comes and goes
    /// in the middle of its scope's order (the Trajectory panel's unpin, R-453). Nothing when `after` is not in the
    /// tree.
    ///
    /// # Panics
    /// When the id is already in the tree: a registration error, caught by the screen's tests.
    pub fn insert_after(&mut self, after: &str, scope: Scope) {
        assert!(
            self.index(scope.id).is_none(),
            "the scope `{}` is registered twice",
            scope.id
        );
        let Some(i) = self.index(after) else {
            return;
        };
        let at = i + 1;
        for node in &mut self.nodes {
            if let Some(p) = node.parent.as_mut() {
                if *p >= at {
                    *p += 1;
                }
            }
        }
        let parent = self.nodes[i].parent;
        self.nodes.insert(at, Node { scope, parent });
    }

    /// Removes scope `id` and every scope below it; the rest keep their places and parents. Nothing for an id the tree
    /// does not hold. A focus path into a removed scope is cut back at the next command.
    pub fn remove(&mut self, id: &str) {
        if self.index(id).is_none() {
            return;
        }
        self.remove_children(id);
        let i = self.index(id).expect("kept by remove_children");
        self.nodes.remove(i);
        for node in &mut self.nodes {
            if let Some(p) = node.parent.as_mut() {
                if *p > i {
                    *p -= 1;
                }
            }
        }
    }

    /// The scope `id`, to change its label or its arrows as the screen's state changes (the Trajectory panel's first
    /// tab, "pinned" or "under cursor", and its sphere's place, the sphere or the unwrapped view, R-453).
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Scope> {
        self.index(id).map(|i| &mut self.nodes[i].scope)
    }

    /// Adds a row to the `?` overlay, after the global table's: a screen's own keys, registered with its scopes.
    pub fn register_shortcut(&mut self, key: &'static str, action: &'static str) {
        self.shortcuts.push((key, action));
    }

    /// The rows the screens added to the `?` overlay, in registration order.
    pub fn shortcuts(&self) -> &[Shortcut] {
        &self.shortcuts
    }

    /// The parent of scope `id`: `None` for a big scope or an id the tree does not hold.
    pub fn parent(&self, id: &str) -> Option<ScopeId> {
        let i = self.index(id)?;
        self.nodes[i].parent.map(|p| self.nodes[p].scope.id)
    }

    /// The path from a big scope down to scope `id`; empty for an id the tree does not hold.
    pub fn path_to(&self, id: &str) -> Vec<String> {
        let mut path = Vec::new();
        let mut at = self.index(id);
        while let Some(i) = at {
            path.push(self.nodes[i].scope.id.to_owned());
            at = self.nodes[i].parent;
        }
        path.reverse();
        path
    }

    /// The menus, in registration order.
    pub fn menus(&self) -> Vec<ScopeId> {
        self.nodes
            .iter()
            .filter(|n| n.scope.menu)
            .map(|n| n.scope.id)
            .collect()
    }

    /// The scope `id`.
    pub fn get(&self, id: &str) -> Option<&Scope> {
        self.index(id).map(|i| &self.nodes[i].scope)
    }

    /// The big scopes, in Tab order: the top level, its windows left out (§G3).
    pub fn bigs(&self) -> Vec<ScopeId> {
        self.nodes
            .iter()
            .filter(|n| n.parent.is_none() && !n.scope.window)
            .map(|n| n.scope.id)
            .collect()
    }

    /// The window `path` is in: its first scope, when that is a window.
    pub fn window_of(&self, path: &[String]) -> Option<ScopeId> {
        let first = self.get(path.first()?)?;
        first.window.then_some(first.id)
    }

    /// Where the focus lands on section `section` of a window: on the section's first element, or on the section
    /// itself when it is one element (applied per R-369: a Tab lands on a section's first element, and the arrows
    /// move among its siblings). In a window whose sections are scopes, on the section itself (R-450).
    pub fn section_landing(&self, window: ScopeId, section: ScopeId) -> Vec<String> {
        let mut path = vec![window.to_owned(), section.to_owned()];
        if self.get(window).is_some_and(|w| w.scopes) {
            return path;
        }
        if let Some(first) = self.children(Some(section)).first() {
            path.push((*first).to_owned());
        }
        path
    }

    /// Where the focus lands when window `id` opens: its first section's landing; the window itself when it has no
    /// section.
    pub fn window_landing(&self, id: ScopeId) -> Vec<String> {
        match self.children(Some(id)).first() {
            Some(section) => self.section_landing(id, section),
            None => vec![id.to_owned()],
        }
    }

    /// The children of `parent`, or every top-level scope for `None` (the big scopes and the windows), in order.
    pub fn children(&self, parent: Option<&str>) -> Vec<ScopeId> {
        let parent = match parent {
            Some(p) => match self.index(p) {
                Some(i) => Some(i),
                None => return Vec::new(),
            },
            None => None,
        };
        self.nodes
            .iter()
            .filter(|n| n.parent == parent)
            .map(|n| n.scope.id)
            .collect()
    }

    /// The length of `path`'s longest prefix that is a path through the tree from a big scope down.
    pub fn valid_prefix(&self, path: &[String]) -> usize {
        let mut parent = None;
        for (depth, id) in path.iter().enumerate() {
            match self.index(id) {
                Some(i) if self.nodes[i].parent == parent => parent = Some(i),
                _ => return depth,
            }
        }
        path.len()
    }

    /// The breadcrumb labels of `path`.
    pub fn labels(&self, path: &[String]) -> Vec<&'static str> {
        path.iter()
            .filter_map(|id| self.get(id))
            .map(|s| s.label)
            .collect()
    }

    /// Runs `command` on `focus`, in its mode (render_gui_spec §G3, "Two modes"; R-409), the steps `multiplier` times
    /// the base. A path the tree no longer holds (the mode switched) is cut back to its valid part first, in
    /// navigation mode.
    ///
    /// - Tab and Shift+Tab, from either mode, land on the next or previous big scope itself, in navigation mode; inside a
    ///   window, on its next or previous section's landing, staying in the window (R-409 G6).
    /// - Enter in navigation mode: on a scope, into it, on its first element; on a control, its click; on a value (the
    ///   Figure's pan, the compass, a slider), a list or a text field, interaction mode; on anything else, the Legend
    ///   among them, nothing. In interaction mode: back to navigation mode on the same element.
    /// - Esc in interaction mode: back to navigation mode on the same element; in navigation mode, up one scope, or,
    ///   on a window's section or the window itself, [`Outcome::Leave`].
    /// - The arrows in navigation mode move between siblings, stopping at either end (on a big scope, between the big
    ///   scopes in Tab order), only along the parent's [`Axis`]; in interaction mode they adjust the focused value, or,
    ///   on a list, ↑ ↓ scroll it an entry, Shift ten (R-450 A4).
    /// - Page Up, Page Down, Home and End, in interaction mode on a list, scroll it a page, Shift ten, or to its first
    ///   or last entry; otherwise nothing.
    pub fn navigate(&self, focus: &mut Focus, command: Command, multiplier: f64) -> Outcome {
        let valid = self.valid_prefix(&focus.path);
        if valid < focus.path.len() || valid == 0 {
            focus.path.truncate(valid);
            focus.mode = KeyMode::Navigation;
        }
        let interacting = focus.mode == KeyMode::Interaction;
        match command {
            Command::Next | Command::Previous => {
                if let Some(window) = self.window_of(&focus.path) {
                    let sections = self.children(Some(window));
                    let n = sections.len();
                    if n == 0 {
                        return Outcome::None;
                    }
                    let at = focus
                        .path
                        .get(1)
                        .and_then(|id| sections.iter().position(|s| s == id));
                    let to = match (at, command) {
                        (None, Command::Next) => 0,
                        (None, _) => n - 1,
                        (Some(i), Command::Next) => (i + 1) % n,
                        (Some(i), _) => (i + n - 1) % n,
                    };
                    focus.path = self.section_landing(window, sections[to]);
                    focus.mode = KeyMode::Navigation;
                    return Outcome::Moved;
                }
                let bigs = self.bigs();
                let n = bigs.len();
                if n == 0 {
                    return Outcome::None;
                }
                let at = focus
                    .path
                    .first()
                    .and_then(|id| bigs.iter().position(|b| b == id));
                let to = match (at, command) {
                    (None, Command::Next) => 0,
                    (None, _) => n - 1,
                    (Some(i), Command::Next) => (i + 1) % n,
                    (Some(i), _) => (i + n - 1) % n,
                };
                focus.path = vec![bigs[to].to_owned()];
                focus.mode = KeyMode::Navigation;
                Outcome::Moved
            }
            Command::Enter => {
                let Some(id) = focus.path.last() else {
                    return Outcome::None;
                };
                if interacting {
                    focus.mode = KeyMode::Navigation;
                    return Outcome::Moved;
                }
                let scope = *self.get(id).expect("a valid path");
                if let Some(first) = self.children(Some(id)).first() {
                    focus.path.push((*first).to_owned());
                    Outcome::Moved
                } else if scope.activates {
                    Outcome::Activate(scope.id)
                } else if scope.text || scope.arrows != Arrows::Siblings {
                    focus.mode = KeyMode::Interaction;
                    Outcome::Moved
                } else {
                    Outcome::None
                }
            }
            Command::Back if interacting => {
                focus.mode = KeyMode::Navigation;
                Outcome::Moved
            }
            Command::Back => match self.window_of(&focus.path) {
                Some(window) if focus.path.len() <= 2 => Outcome::Leave(window),
                _ => match focus.path.pop() {
                    Some(_) => Outcome::Moved,
                    None => Outcome::None,
                },
            },
            Command::Arrow(direction) => {
                let Some(id) = focus.path.last().cloned() else {
                    return Outcome::None;
                };
                let scope = *self.get(&id).expect("a valid path");
                if interacting {
                    return match scope.arrows {
                        Arrows::Adjust(kind) => Outcome::Adjust(Adjust {
                            scope: scope.id,
                            kind,
                            times: direction.sign() * multiplier,
                            direction,
                        }),
                        // ↓ towards the last entry: the opposite of a value's sign.
                        Arrows::Scroll if Axis::Column.moves(direction) => {
                            Outcome::Scroll(Scroll {
                                scope: scope.id,
                                by: ScrollBy::Entries(scroll_count(-direction.sign(), multiplier)),
                            })
                        }
                        Arrows::Scroll | Arrows::Siblings => Outcome::None,
                    };
                }
                let path = &mut focus.path;
                let parent = path.len().checked_sub(2).map(|i| path[i].as_str());
                if let Some(p) = parent.and_then(|p| self.get(p)) {
                    if !p.axis.moves(direction) {
                        return Outcome::None;
                    }
                }
                let siblings = match parent {
                    Some(p) => self.children(Some(p)),
                    None => self.bigs(),
                };
                // A window itself has no siblings: the big scopes are not its.
                let Some(i) = siblings.iter().position(|s| *s == id) else {
                    return Outcome::None;
                };
                let to = match direction {
                    Direction::Up | Direction::Left => i.checked_sub(1),
                    Direction::Down | Direction::Right => {
                        Some(i + 1).filter(|j| *j < siblings.len())
                    }
                };
                match to {
                    Some(j) => {
                        *path.last_mut().expect("non-empty") = siblings[j].to_owned();
                        Outcome::Moved
                    }
                    None => Outcome::None,
                }
            }
            Command::Scroll(key) => {
                let Some(scope) = focus.path.last().and_then(|id| self.get(id)) else {
                    return Outcome::None;
                };
                if !interacting || scope.arrows != Arrows::Scroll {
                    return Outcome::None;
                }
                let by = match key {
                    ScrollKey::PageUp => ScrollBy::Pages(scroll_count(-1.0, multiplier)),
                    ScrollKey::PageDown => ScrollBy::Pages(scroll_count(1.0, multiplier)),
                    ScrollKey::Home => ScrollBy::First,
                    ScrollKey::End => ScrollBy::Last,
                };
                Outcome::Scroll(Scroll {
                    scope: scope.id,
                    by,
                })
            }
            Command::Shortcuts => Outcome::None,
        }
    }
}
