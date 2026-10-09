//! The tree of keyboard scopes (render_gui_spec §G3; GUI_DESIGN_NOTES § "07 Keyboard"; R-390): the big scopes in Tab
//! order, each with the sub-scopes Enter reaches. Each screen joins the tree through [`ScopeTree::register`], in the
//! order it draws its scopes; the focus is a path through it, held in `ViewUI` (gui_state_contract §2).
//!
//! The base steps (REQ-GUI-158) are calibrations proposed here (R-71), used provisionally until the human confirms
//! them at the M8 gate (R-182): [`StepKind::base`].

use crate::keyboard::keymap::{Command, Direction};

/// A scope's id: unique in its tree, and what `ViewUI`'s focus path holds.
pub type ScopeId = &'static str;

/// What the arrows do while a scope has focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrows {
    /// Move between the scope and its siblings, stopping at either end.
    Siblings,
    /// Adjust the focused value by steps of this kind.
    Adjust(StepKind),
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
        }
    }

    /// A control that Enter activates.
    pub const fn control(id: ScopeId, label: &'static str) -> Self {
        Self {
            activates: true,
            ..Self::group(id, label)
        }
    }

    /// A value the arrows adjust by steps of `kind`.
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
    /// An angle field: the tilt angles `τ₁`, `τ₂` and the rotation `γ`.
    Angle,
    /// The zoom, in log₂.
    ZoomLog2,
    /// Figure's arrows: pan the view.
    Pan,
    /// Compass's arrows: tilt the slice plane.
    Tilt,
    /// Compass's Shift+arrows: orbit the cube.
    Orbit,
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
    /// The base step, before Shift's ×10 or Alt's ×0.1: proposed, R-71 (REQ-GUI-158), the reasoning in the PR of
    /// TASK-M6-25 and the human's confirmation at the M8 gate to come.
    pub const fn base(self) -> Base {
        match self {
            // A hundredth of the range: Shift crosses a tenth, Alt a thousandth.
            StepKind::Bounded | StepKind::TimeStep => Base::OfRange(0.01),
            // A degree: Shift ten, Alt a tenth.
            StepKind::Angle | StepKind::Tilt => Base::Degrees(1.0),
            // A quarter octave: four presses double the zoom, Shift is 2.5 octaves.
            StepKind::ZoomLog2 => Base::Log2(0.25),
            // A twentieth of the view: Shift moves half a view, keeping half of it in sight.
            StepKind::Pan => Base::OfView(0.05),
            // Five degrees: a turn of the cube in 72 presses, 7.2 with Shift.
            StepKind::Orbit => Base::Degrees(5.0),
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
}

impl Adjust {
    /// The step to apply.
    pub fn delta(&self) -> Base {
        self.kind.base().times(self.times)
    }
}

/// What one command did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// Nothing.
    None,
    /// The focus moved.
    Moved,
    /// Enter activated this control.
    Activate(ScopeId),
    /// The arrows adjusted a value.
    Adjust(Adjust),
}

struct Node {
    scope: Scope,
    parent: Option<usize>,
}

/// A tree of scopes, its children in registration order.
#[derive(Default)]
pub struct ScopeTree {
    nodes: Vec<Node>,
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

    /// The scope `id`.
    pub fn get(&self, id: &str) -> Option<&Scope> {
        self.index(id).map(|i| &self.nodes[i].scope)
    }

    /// The children of `parent`, or the big scopes for `None`, in order.
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

    /// Runs `command` on the focus `path`, the steps `multiplier` times the base. A path the tree no longer holds
    /// (the mode switched) is cut back to its valid part first.
    pub fn navigate(&self, path: &mut Vec<String>, command: Command, multiplier: f64) -> Outcome {
        path.truncate(self.valid_prefix(path));
        match command {
            Command::Next | Command::Previous => {
                let bigs = self.children(None);
                let n = bigs.len();
                if n == 0 {
                    return Outcome::None;
                }
                let at = path
                    .first()
                    .and_then(|id| bigs.iter().position(|b| b == id));
                let to = match (at, command) {
                    (None, Command::Next) => 0,
                    (None, _) => n - 1,
                    (Some(i), Command::Next) => (i + 1) % n,
                    (Some(i), _) => (i + n - 1) % n,
                };
                *path = vec![bigs[to].to_owned()];
                Outcome::Moved
            }
            Command::Enter => {
                let Some(id) = path.last() else {
                    return Outcome::None;
                };
                let scope = *self.get(id).expect("a valid path");
                if let Some(first) = self.children(Some(id)).first() {
                    path.push((*first).to_owned());
                    Outcome::Moved
                } else if scope.activates {
                    Outcome::Activate(scope.id)
                } else {
                    Outcome::None
                }
            }
            Command::Back => match path.pop() {
                Some(_) => Outcome::Moved,
                None => Outcome::None,
            },
            Command::Arrow(direction) => {
                let Some(id) = path.last().cloned() else {
                    return Outcome::None;
                };
                let scope = *self.get(&id).expect("a valid path");
                match scope.arrows {
                    Arrows::Adjust(kind) => Outcome::Adjust(Adjust {
                        scope: scope.id,
                        kind,
                        times: direction.sign() * multiplier,
                    }),
                    Arrows::Siblings => {
                        let parent = path.len().checked_sub(2).map(|i| path[i].as_str());
                        let siblings = self.children(parent);
                        let i = siblings
                            .iter()
                            .position(|s| *s == id)
                            .expect("a valid path");
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
                }
            }
            Command::Shortcuts => Outcome::None,
        }
    }
}
