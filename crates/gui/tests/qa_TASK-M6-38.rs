//! QA tests for TASK-M6-38, written from the requirements and R-457, R-459 G4 and gui_state_contract §2, not from the
//! implementation:
//!
//! - REQ-GUI-204: on the mock, the Navigate section holds centre (u, v) and zoom, with the depth readout, the precision
//!   warning and the lock row, and no grid of z₀ value boxes; the section that was 'Centre z₀' is 'Slice centre', its
//!   eight sliders z_α, z_β, z_q0…z_q3, z_μ1, z_μ2 each with its number box beside it; the section that was 'Slice &
//!   tilt' is 'Tilt' and holds τ₁, τ₂ and γ, each slider with its number box, and no step slider; the breadcrumb and
//!   the keyboard's sub-scopes use the new names. Verify (mock_manifold_sections): Navigate's widgets, no
//!   single-coordinate z₀ box there (centre u, v pan within the plane); Slice centre's eight sliders, each number box
//!   editing the same component; Tilt's τ₁, τ₂ and γ with number boxes and no widget that edits z₀; the breadcrumb
//!   'Manifold view › Tilt › τ₁'; Enter on Manifold view then ↓ reaching Chart, Navigate, Slice centre and Tilt;
//!   controls: the eight z₀ boxes under Navigate, a step slider in Tilt, and the old names each fail.
//! - REQ-GUI-205: each tilt angle has its own target, 'tilt τ₁ into [X]' and 'tilt τ₂ into [Y]', offering only the
//!   latent dimensions not e_a or e_b; `u₁ = cos τ₁ e_a + sin τ₁ e_X`, `u₂ = cos τ₂ e_b + sin τ₂ e_Y`, then turned by γ
//!   as before (R-457 E1); both targets default to the slice normal, so the defaults give the old basis exactly; the
//!   targets with the normal are SimConfig's slice frame, undoable; a target change at a non-zero angle re-tilts, one
//!   history entry editing the frame and the basis, at a zero angle one entry editing the frame alone, undo restoring
//!   both; the normal is never one of the plane's axes: a preset change or a Chart basis edit whose plane takes it
//!   moves it by the slice-axis rule (z_q3, or the first axis the plane does not show) in the same entry (R-459 G4); a
//!   target equal to the normal follows it, and a plane change that takes a target resets it to the normal; the
//!   compass's readout names both targets with their angles ("τ₁ → z_q3 +12.0°", "τ₂ → z_μ1 0.0°", R-457 E6).
//!
//! Edits are counted from the contract's own log (every engine logs each SetField it applies as one `info` entry from
//! `contract`, gui_state_contract §2) and from the history depth; what they edited is read from the snapshots. The map
//! before R-457, which the defaults must reproduce, is written here from the Manifold view's frame as R-457 E1 restates
//! it, not taken from the code. gui takes no dependency on validation (R-187), so each check runs its own control
//! (R-176) through [`rejects`].
// The file name `qa_TASK-M6-38` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::log::Source;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::SliceFrame;
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::{KeyMode, Mode};
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::manifold_view::View;
use gui::headless::Headless;
use gui::keyboard::scopes::Arrows;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::MockSide;

type Z = [f64; 8];

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
/// A key no command binds: pressed only to turn keyboard navigation on (R-460, R-464 M2).
const UNBOUND: Key = Key::F12;

/// §G2's eight `z₀` names, in chart_decoder_contract Part 2's block order.
const AXES: [&str; 8] = ["z_α", "z_β", "z_q0", "z_q1", "z_q2", "z_q3", "z_μ1", "z_μ2"];
const ALPHA: usize = 0;
const BETA: usize = 1;
const Q0: usize = 2;
const Q2: usize = 4;
const Q3: usize = 5;
const MU1: usize = 6;
const MU2: usize = 7;
/// §G3's sub-scopes of Manifold view, R-457's names, in order.
const SECTIONS: [&str; 4] = ["Chart", "Navigate", "Slice centre", "Tilt"];
/// The names R-457 retires.
const OLD_NAMES: [&str; 2] = ["Centre z₀", "Slice & tilt"];
/// The breadcrumb's separator, §G3's "Manifold view › Navigate › zoom".
const SEP: &str = " › ";

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

// --- vectors --------------------------------------------------------------------------------------------------------

fn e(i: usize, k: f64) -> Z {
    std::array::from_fn(|j| if j == i { k } else { 0.0 })
}

fn add(a: &Z, k: f64, b: &Z) -> Z {
    std::array::from_fn(|i| a[i] + k * b[i])
}

fn scaled(a: &Z, k: f64) -> Z {
    a.map(|x| x * k)
}

fn dot(a: &Z, b: &Z) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn max_diff(a: &Z, b: &Z) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// The latent indices where `v` is not zero.
fn support(v: &Z) -> Vec<usize> {
    (0..8).filter(|&i| v[i].abs() > 1e-12).collect()
}

/// The map before R-457, written from the Manifold view's frame (R-457 E1, "then γ as now"): one shared hidden
/// direction `e_d`, `u₁ = cos τ₁ e_a + sin τ₁ e_d`, `u₂ = cos τ₂ e_b + sin τ₂ e_d`, `q₁ = σ(cos γ u₁ + sin γ u₂)`,
/// `q₂ = σ(cos γ u₂ − sin γ u₁)`.
fn old_map(a: usize, b: usize, d: usize, t1: f64, t2: f64, g: f64, s: f64) -> (Z, Z) {
    let u1 = add(&e(a, t1.cos()), t1.sin(), &e(d, 1.0));
    let u2 = add(&e(b, t2.cos()), t2.sin(), &e(d, 1.0));
    let q1 = scaled(&add(&scaled(&u1, g.cos()), g.sin(), &u2), s);
    let q2 = scaled(&add(&scaled(&u2, g.cos()), -g.sin(), &u1), s);
    (q1, q2)
}

/// The view on the plane `(a, b)` with the normal `d`, targets `x`, `y`, at `(τ₁, τ₂, γ)` in degrees and scale `σ`.
#[allow(clippy::too_many_arguments)] // the view's nine coordinates; a struct of them would be `View` itself, built here
fn view(
    a: usize,
    b: usize,
    d: usize,
    x: usize,
    y: usize,
    t1: f64,
    t2: f64,
    g: f64,
    s: f64,
) -> View {
    View {
        preset: None,
        a,
        b,
        d,
        x,
        y,
        scale: s,
        tau1: t1.to_radians(),
        tau2: t2.to_radians(),
        gamma: g.to_radians(),
    }
}

// --- the rig --------------------------------------------------------------------------------------------------------

fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

struct Rig {
    app: App<MockSide>,
    h: Headless,
}

fn mock() -> Rig {
    let mut rig = Rig {
        app: App::new(MockSide::new(MockEngine::frozen(), None), FORMAT),
        h: headless(),
    };
    rig.idle(2);
    rig
}

impl Rig {
    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.h.frame(&mut self.app, events)
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            let _ = self.frame(Vec::new());
        }
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        for pressed in [true, false] {
            let _ = self.frame(vec![Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            }]);
        }
        self.idle(2);
    }

    fn snap(&self) -> Snapshot {
        self.app.snapshot().clone()
    }

    /// The SetFields the engine has applied so far: its `contract` log entries, less the hover's (a pointer over the
    /// figure writes `Hover.at`, a no-history SetField, R-101).
    fn applied(&self) -> usize {
        self.app
            .console()
            .iter()
            .filter(|e| {
                e.source == Source::Contract && !e.message.starts_with("SetField Hover.at ")
            })
            .count()
    }

    /// An edit sent straight to the contract, as the test's setup: the GUI's own path is not under test there.
    fn set(&mut self, edit: SimField) {
        self.app.set_field(SetField {
            edit: Edit::Sim(edit),
            no_history: false,
        });
        self.idle(3);
    }

    /// The scope labelled `label` under the scope labelled `parent` (anywhere below it), breadth first.
    fn scope(&self, parent: &str, label: &str) -> &'static str {
        let parent = self.find(None, parent);
        self.find(Some(parent), label)
    }

    fn find(&self, root: Option<&str>, label: &str) -> &'static str {
        let tree = self.app.keyboard.tree(Mode::Explore);
        let mut all: Vec<&'static str> = tree.children(root);
        let mut i = 0;
        while i < all.len() {
            let kids = tree.children(Some(all[i]));
            all.extend(kids);
            i += 1;
        }
        all.into_iter()
            .find(|id| tree.get(id).unwrap().label == label)
            .unwrap_or_else(|| panic!("no scope `{label}` under {root:?}"))
    }

    /// The labels of `id`'s children, in the tree's order.
    fn child_labels(&self, id: &str) -> Vec<&'static str> {
        let tree = self.app.keyboard.tree(Mode::Explore);
        tree.children(Some(id))
            .into_iter()
            .map(|c| tree.get(c).unwrap().label)
            .collect()
    }

    /// Puts the focus on scope `id`, in navigation mode, as a landing is (R-409 point 4); a key no command binds then
    /// turns keyboard navigation on (R-460, R-464 M2).
    fn focus(&mut self, id: &str) {
        let path = self.app.keyboard.tree(Mode::Explore).path_to(id);
        assert!(!path.is_empty(), "no scope {id} in the tree");
        self.app.view.focus.path = path;
        self.app.view.focus.mode = KeyMode::Navigation;
        self.key(UNBOUND, NONE);
    }

    /// The focus on value `id`, then Enter: interaction mode, where the arrows adjust it.
    fn interact(&mut self, id: &str) {
        self.focus(id);
        self.key(Key::Enter, NONE);
        assert_eq!(
            self.app.view.focus.mode,
            KeyMode::Interaction,
            "Enter on {id} did not start interaction mode"
        );
    }

    /// The focus's labels, outermost first.
    fn labels(&self) -> Vec<String> {
        self.app
            .keyboard
            .tree(Mode::Explore)
            .labels(&self.app.view.focus.path)
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// The texts drawn this frame, with their rects in points.
    fn texts(&mut self) -> Vec<(String, Rect)> {
        let out = self.frame(Vec::new());
        let mut found = Vec::new();
        for clipped in &out.shapes {
            flatten_texts(&clipped.shape, &mut found);
        }
        found
    }

    fn click(&mut self, at: Pos2) {
        let button = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: NONE,
        };
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        let _ = self.frame(vec![button(true)]);
        let _ = self.frame(vec![button(false)]);
        self.idle(2);
    }

    /// Types `text` into the field under the focus, replacing what it holds, then Enter.
    fn type_in(&mut self, text: &str) {
        let select_all = Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        };
        let _ = self.frame(vec![select_all]);
        let _ = self.frame(vec![Event::Text(text.to_owned())]);
        let _ = self.frame(vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: NONE,
        }]);
        self.idle(3);
    }

    /// Runs frames until the compass has come to rest (R-451 B4: its readout eases toward the plane's values).
    fn settle(&mut self) {
        for _ in 0..400 {
            if !self.app.manifold.compass.moving {
                return;
            }
            self.idle(1);
        }
        panic!("the compass did not come to rest");
    }
}

fn flatten_texts(shape: &egui::Shape, out: &mut Vec<(String, Rect)>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten_texts(s, out)),
        egui::Shape::Text(t) => {
            out.push((t.galley.text().to_owned(), shape.visual_bounding_rect()))
        }
        _ => {}
    }
}

/// The texts inside `rect`.
fn inside(texts: &[(String, Rect)], rect: Rect) -> Vec<(String, Rect)> {
    texts
        .iter()
        .filter(|(_, r)| rect.contains_rect(*r))
        .cloned()
        .collect()
}

/// The top of the first text in the Manifold view panel reading `title` exactly.
fn title_y(texts: &[(String, Rect)], title: &str) -> f32 {
    let panel = layout().manifold_view;
    texts
        .iter()
        .filter(|(t, r)| t == title && panel.contains_rect(*r))
        .map(|(_, r)| r.min.y)
        .fold(f32::INFINITY, f32::min)
}

/// The texts of the panel's section titled `title`: below its title and above the next section's (or the panel's
/// foot for the last).
fn section_texts(texts: &[(String, Rect)], title: &str) -> Vec<(String, Rect)> {
    let panel = layout().manifold_view;
    let i = SECTIONS.iter().position(|s| *s == title).unwrap();
    let top = title_y(texts, title);
    assert!(top.is_finite(), "no section titled `{title}` in the panel");
    let bottom = SECTIONS
        .get(i + 1)
        .map_or(panel.max.y, |next| title_y(texts, next));
    texts
        .iter()
        .filter(|(_, r)| panel.contains_rect(*r) && r.min.y > top && r.min.y < bottom)
        .cloned()
        .collect()
}

fn number(text: &str) -> Option<f64> {
    let t: String = text
        .replace('−', "-")
        .chars()
        .filter(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
        .collect();
    t.trim_start_matches('+').parse().ok()
}

/// One gesture on `rig`: the SetFields applied, the undo entries added, and the snapshots before and after.
fn gesture(rig: &mut Rig, act: impl FnOnce(&mut Rig)) -> (usize, i64, Snapshot, Snapshot) {
    let (before, n0) = (rig.snap(), rig.applied());
    act(rig);
    let after = rig.snap();
    let entries = i64::from(after.history.undo_depth) - i64::from(before.history.undo_depth);
    (rig.applied() - n0, entries, before, after)
}

/// The mock on the plane `z_α × z_β` at scale 1, tilted to `(τ₁, τ₂)` degrees with γ = 0 into the frame's targets.
fn tilted(rig: &mut Rig, frame: SliceFrame, t1: f64, t2: f64) {
    rig.set(SimField::SliceFrame { frame, basis: None });
    let (x, y) = (frame.tilt_x as usize, frame.tilt_y as usize);
    let (t1, t2) = (t1.to_radians(), t2.to_radians());
    let q1 = add(&e(ALPHA, t1.cos()), t1.sin(), &e(x, 1.0));
    let q2 = add(&e(BETA, t2.cos()), t2.sin(), &e(y, 1.0));
    rig.set(SimField::Basis { q1, q2 });
}

fn frame(normal: usize, x: usize, y: usize) -> SliceFrame {
    SliceFrame {
        normal: normal as u32,
        tilt_x: x as u32,
        tilt_y: y as u32,
    }
}

// === REQ-GUI-205: the map ===========================================================================================

/// `View::basis` against the old map at the default targets, every component within `tol`.
fn check_defaults_are_the_old_map(map: impl Fn(&View) -> (Z, Z), tol: f64) {
    let angles = [-90.0, -61.0, -30.0, -7.5, 0.0, 12.0, 45.0, 73.25, 90.0];
    let gammas = [-180.0, -135.0, -20.0, 0.0, 33.0, 90.0, 179.0, 180.0];
    let scales = [1e-6, 1e-3, 0.25, 1.0, 2.0, 4.0];
    for (a, b) in [(ALPHA, BETA), (Q0, 3), (Q2, Q3), (MU1, MU2), (ALPHA, MU1)] {
        let d = if a != Q3 && b != Q3 {
            Q3
        } else {
            (0..8).find(|i| *i != a && *i != b).unwrap()
        };
        for t1 in angles {
            for t2 in angles {
                for g in gammas {
                    for s in scales {
                        let v = view(a, b, d, d, d, t1, t2, g, s);
                        let (q1, q2) = map(&v);
                        let (w1, w2) =
                            old_map(a, b, d, t1.to_radians(), t2.to_radians(), g.to_radians(), s);
                        let err = max_diff(&q1, &w1).max(max_diff(&q2, &w2));
                        assert!(
                            err <= tol,
                            "({a}, {b}) d {d} at τ₁ {t1}°, τ₂ {t2}°, γ {g}°, σ {s}: off the old map by {err}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn qa_tilt_targets_the_defaults_give_the_old_map_to_1e_15() {
    check_defaults_are_the_old_map(View::basis, 1e-15);
    // The mock's own frame at start: both targets the slice normal, z_q3 on its z_α × z_β (gui_state_contract §2).
    let rig = mock();
    let f = rig.snap().sim.slice.frame;
    assert_eq!(f, frame(Q3, Q3, Q3), "the slice frame's defaults");
    // Control: a map turning γ the other way.
    rejects("γ turned the other way", || {
        check_defaults_are_the_old_map(
            |v| {
                View {
                    gamma: -v.gamma,
                    ..*v
                }
                .basis()
            },
            1e-15,
        )
    });
}

/// R-457 E1's two-target form at X = z_q3, Y = z_μ1, τ₁ = 30°, τ₂ = 20°, γ = 0 on z_α × z_β, at scale `s`.
fn check_two_targets(q1: &Z, q2: &Z, s: f64) {
    let (c30, s30) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    let (c20, s20) = (20f64.to_radians().cos(), 20f64.to_radians().sin());
    let w1 = scaled(&add(&e(ALPHA, c30), s30, &e(Q3, 1.0)), s);
    let w2 = scaled(&add(&e(BETA, c20), s20, &e(MU1, 1.0)), s);
    assert!(
        max_diff(q1, &w1) <= 1e-15 * s.max(1.0),
        "q₁ {q1:?}, R-457 E1 gives {w1:?}"
    );
    assert!(
        max_diff(q2, &w2) <= 1e-15 * s.max(1.0),
        "q₂ {q2:?}, R-457 E1 gives {w2:?}"
    );
    assert!(
        dot(q1, q2).abs() <= 1e-15 * s * s,
        "q₁·q₂ = {}",
        dot(q1, q2)
    );
}

#[test]
fn qa_tilt_targets_two_targets_tilt_each_axis_into_its_own() {
    for s in [0.5, 1.0, 3.0] {
        let (q1, q2) = view(ALPHA, BETA, Q3, Q3, MU1, 30.0, 20.0, 0.0, s).basis();
        check_two_targets(&q1, &q2, s);
    }
    // Control: the one shared target, both tilted into z_q3.
    let (q1, q2) = view(ALPHA, BETA, Q3, Q3, Q3, 30.0, 20.0, 0.0, 1.0).basis();
    rejects("one shared target", || check_two_targets(&q1, &q2, 1.0));
    // Through the GUI: Y picked as z_μ1 in the Tilt section's dropdown, then τ₁ and τ₂ turned by their keys: q₁ stays in
    // span(e_α, e_q3) and q₂ in span(e_β, e_μ1).
    let mut rig = mock();
    let menu = rig.find(None, "tilt τ₂ into");
    let entry = rig.scope("tilt τ₂ into", AXES[MU1]);
    let _ = menu;
    rig.focus(entry);
    rig.key(Key::Enter, NONE);
    assert_eq!(rig.snap().sim.slice.frame, frame(Q3, Q3, MU1), "Y picked");
    for label in ["τ₁", "τ₂"] {
        let id = rig.scope("Tilt", label);
        rig.interact(id);
        rig.key(Key::ArrowUp, SHIFT);
        rig.key(Key::Escape, NONE);
    }
    let p = rig.snap().sim.plane;
    assert_eq!(support(&p.q1), vec![ALPHA, Q3], "q₁ {:?}", p.q1);
    assert_eq!(support(&p.q2), vec![BETA, MU1], "q₂ {:?}", p.q2);
    assert!(
        dot(&p.q1, &p.q2).abs() < 1e-15,
        "q₁·q₂ = {}",
        dot(&p.q1, &p.q2)
    );
}

// === REQ-GUI-205: the dropdowns ======================================================================================

/// Each dropdown, as the keyboard reaches it (R-457 E5), lists exactly the six dimensions not `e_a`, `e_b`, in order.
fn check_offers(offered: &[&str], a: usize, b: usize) {
    let want: Vec<&str> = (0..8)
        .filter(|i| *i != a && *i != b)
        .map(|i| AXES[i])
        .collect();
    assert_eq!(offered, want, "on the plane {} × {}", AXES[a], AXES[b]);
}

#[test]
fn qa_tilt_targets_each_dropdown_lists_the_six_dimensions_not_in_the_plane() {
    let mut rig = mock();
    for menu in ["tilt τ₁ into", "tilt τ₂ into"] {
        let id = rig.find(None, menu);
        check_offers(&rig.child_labels(id), ALPHA, BETA);
    }
    // Drawn: the Tilt section names each dropdown, "tilt τ₁ into" and "tilt τ₂ into", with its target, z_q3.
    let texts = rig.texts();
    let tilt = section_texts(&texts, "Tilt");
    for want in ["tilt τ₁ into", "tilt τ₂ into"] {
        assert!(
            tilt.iter().any(|(t, _)| t.contains(want)),
            "no `{want}` in Tilt: {tilt:?}"
        );
    }
    assert!(
        tilt.iter().filter(|(t, _)| t == AXES[Q3]).count() >= 2,
        "the two dropdowns do not show z_q3: {tilt:?}"
    );
    // On another plane the lists follow it: z_μ1 × z_μ2.
    rig.set(SimField::Basis {
        q1: e(MU1, 1.0),
        q2: e(MU2, 1.0),
    });
    for menu in ["tilt τ₁ into", "tilt τ₂ into"] {
        let id = rig.find(None, menu);
        check_offers(&rig.child_labels(id), MU1, MU2);
    }
    // Controls: a dropdown offering e_a; one leaving out the slice normal.
    rejects("a dropdown that offers e_a", || {
        check_offers(
            &["z_α", "z_q0", "z_q1", "z_q2", "z_q3", "z_μ1"],
            ALPHA,
            BETA,
        )
    });
    rejects("a dropdown without the slice normal", || {
        check_offers(&["z_q0", "z_q1", "z_q2", "z_μ1", "z_μ2"], ALPHA, BETA)
    });
}

#[test]
fn qa_tilt_targets_the_dropdown_by_keys_enter_opens_esc_closes_unchanged_enter_picks() {
    // R-457 E5: Enter opens it, ↑ ↓ choose, Enter picks, Esc closes it unchanged.
    let mut rig = mock();
    let menu = rig.find(None, "tilt τ₁ into");
    rig.focus(menu);
    let (n, entries, before, after) = gesture(&mut rig, |r| {
        r.key(Key::Enter, NONE);
        r.key(Key::ArrowDown, NONE);
        r.key(Key::Escape, NONE);
    });
    assert_eq!((n, entries), (0, 0), "Esc on the open dropdown edited");
    assert_eq!(after.sim.slice.frame, before.sim.slice.frame);
    rig.focus(menu);
    let (n, entries, before, after) = gesture(&mut rig, |r| {
        r.key(Key::Enter, NONE);
        r.key(Key::ArrowDown, NONE);
        r.key(Key::Enter, NONE);
    });
    let x = after.sim.slice.frame.tilt_x as usize;
    assert_eq!(
        (n, entries),
        (1, 1),
        "a pick by keys is one SetField, one entry"
    );
    assert!(
        x != before.sim.slice.frame.tilt_x as usize && ![ALPHA, BETA].contains(&x),
        "the pick by keys set X to {}",
        AXES[x]
    );
}

// === REQ-GUI-205: a target change, undoable ========================================================================

/// A change of X from z_q3 to z_q2 at τ₁ = 30°: one SetField and one entry, q₁ re-tilted into e_q2 at the same angle,
/// q₂ kept; the frame's X z_q2.
fn check_retilt(n: usize, entries: i64, before: &Snapshot, after: &Snapshot) {
    assert_eq!(
        (n, entries),
        (1, 1),
        "the target change is one SetField, one entry"
    );
    let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    let want = add(&e(ALPHA, c), s, &e(Q2, 1.0));
    assert!(
        max_diff(&after.sim.plane.q1, &want) < 1e-12,
        "q₁ {:?} is not re-tilted into e_q2 at 30°: {want:?}",
        after.sim.plane.q1
    );
    assert_eq!(after.sim.plane.q2, before.sim.plane.q2, "q₂ moved");
    assert_eq!(after.sim.plane.z0, before.sim.plane.z0, "z₀ moved");
    assert_eq!(after.sim.slice.frame, frame(Q3, Q2, Q3));
}

/// One undo restores the basis and X = z_q3.
fn check_undone(undone: &Snapshot, before: &Snapshot) {
    assert!(
        max_diff(&undone.sim.plane.q1, &before.sim.plane.q1) < 1e-15
            && max_diff(&undone.sim.plane.q2, &before.sim.plane.q2) < 1e-15,
        "one Ctrl+Z did not restore the basis"
    );
    assert_eq!(
        undone.sim.slice.frame, before.sim.slice.frame,
        "one Ctrl+Z did not restore the target"
    );
}

#[test]
fn qa_tilt_targets_a_change_at_a_non_zero_angle_retilts_in_one_entry_undone_by_one_ctrl_z() {
    let mut rig = mock();
    tilted(&mut rig, frame(Q3, Q3, Q3), 30.0, 0.0);
    let entry = rig.scope("tilt τ₁ into", AXES[Q2]);
    rig.focus(entry);
    let (n, entries, before, after) = gesture(&mut rig, |r| r.key(Key::Enter, NONE));
    check_retilt(n, entries, &before, &after);
    rig.key(Key::Z, Modifiers::CTRL);
    let undone = rig.snap();
    check_undone(&undone, &before);
    // Controls: two entries; a target change that does not re-tilt; an undo restoring the basis but not the target.
    rejects("a target change of two entries", || {
        check_retilt(2, 2, &before, &after)
    });
    let mut kept = after.clone();
    kept.sim.plane = before.sim.plane.clone();
    rejects("a target change that does not re-tilt", || {
        check_retilt(1, 1, &before, &kept)
    });
    let mut half = undone.clone();
    half.sim.slice.frame = after.sim.slice.frame;
    rejects("an undo restoring the basis but not the target", || {
        check_undone(&half, &before)
    });
}

/// A target change at a zero angle: one SetField and one entry, the frame now `want`, the plane untouched.
fn check_frame_alone(
    n: usize,
    entries: i64,
    before: &Snapshot,
    after: &Snapshot,
    want: SliceFrame,
) {
    assert_eq!((n, entries), (1, 1), "one SetField, one entry");
    assert_eq!(after.sim.slice.frame, want);
    assert_eq!(
        after.sim.plane, before.sim.plane,
        "a zero-angle target change edited the plane"
    );
}

#[test]
fn qa_tilt_targets_a_change_at_a_zero_angle_edits_the_frame_alone() {
    let mut rig = mock();
    tilted(&mut rig, frame(Q3, Q3, Q3), 0.0, 25.0);
    let entry = rig.scope("tilt τ₁ into", AXES[Q2]);
    rig.focus(entry);
    let (n, entries, before, after) = gesture(&mut rig, |r| r.key(Key::Enter, NONE));
    check_frame_alone(n, entries, &before, &after, frame(Q3, Q2, Q3));
    // Controls: a zero-angle change that also edits the basis; one of two entries.
    let mut moved = after.clone();
    moved.sim.plane.q1 = add(&moved.sim.plane.q1, 1e-3, &e(Q2, 1.0));
    rejects("a zero-angle change editing the basis", || {
        check_frame_alone(n, entries, &before, &moved, frame(Q3, Q2, Q3))
    });
    rejects("a zero-angle change of two entries", || {
        check_frame_alone(2, 2, &before, &after, frame(Q3, Q2, Q3))
    });
    // The log names the slice frame alone, not the plane (gui_state_contract §2's path).
    let line = rig
        .app
        .console()
        .iter()
        .rev()
        .find(|e| e.source == Source::Contract && !e.message.starts_with("SetField Hover.at "))
        .map(|e| e.message.clone())
        .unwrap();
    assert!(
        line.contains("Slice") && !line.contains("Plane") && !line.contains("q1"),
        "the zero-angle change logged `{line}`"
    );
    rig.key(Key::Z, Modifiers::CTRL);
    let undone = rig.snap();
    assert_eq!(undone.sim.slice.frame, before.sim.slice.frame, "one Ctrl+Z");
    assert_eq!(undone.sim.plane, before.sim.plane);
    assert_eq!(undone.history.undo_depth, before.history.undo_depth);
    // And τ₂'s target at its non-zero angle re-tilts q₂ alone.
    let entry = rig.scope("tilt τ₂ into", AXES[MU2]);
    rig.focus(entry);
    let (n, entries, before, after) = gesture(&mut rig, |r| r.key(Key::Enter, NONE));
    assert_eq!((n, entries), (1, 1));
    assert_eq!(after.sim.plane.q1, before.sim.plane.q1, "q₁ moved");
    assert_eq!(
        support(&after.sim.plane.q2),
        vec![BETA, MU2],
        "q₂ {:?}",
        after.sim.plane.q2
    );
    assert!(
        (after.sim.plane.q2[MU2] - before.sim.plane.q2[Q3]).abs() < 1e-12,
        "q₂ not re-tilted at the same angle"
    );
}

// === REQ-GUI-205 and R-459 G4: the normal never in the plane ========================================================

/// The preset scope, which Enter steps to the next preset.
fn preset(rig: &Rig) -> &'static str {
    rig.scope("Chart", "preset")
}

/// The normal and both targets are off the axis pair `(a, b)` the basis lies along.
fn check_off_the_plane(s: &Snapshot) {
    let (q1, q2) = (&s.sim.plane.q1, &s.sim.plane.q2);
    let (a, b) = (support(q1), support(q2));
    assert!(
        a.len() == 1 && b.len() == 1,
        "not an untilted preset: {q1:?} {q2:?}"
    );
    let f = s.sim.slice.frame;
    for (what, i) in [("the normal", f.normal), ("X", f.tilt_x), ("Y", f.tilt_y)] {
        assert!(
            i as usize != a[0] && i as usize != b[0],
            "{what}, {}, is in the plane {} × {}",
            AXES[i as usize],
            AXES[a[0]],
            AXES[b[0]]
        );
    }
}

#[test]
fn qa_tilt_targets_a_preset_taking_x_resets_x_to_the_normal_and_the_angles_to_0() {
    let mut rig = mock();
    // On z_α × z_β, X = z_q0 at τ₁ = 25°; the next preset, z_q0 × z_q1, takes it.
    tilted(&mut rig, frame(Q3, Q0, Q3), 25.0, -10.0);
    let id = preset(&rig);
    rig.focus(id);
    let (n, entries, _, after) = gesture(&mut rig, |r| r.key(Key::Enter, NONE));
    assert_eq!(
        (n, entries),
        (1, 1),
        "the preset change is one SetField, one entry"
    );
    let (q1, q2) = (after.sim.plane.q1, after.sim.plane.q2);
    assert_eq!(
        (support(&q1), support(&q2)),
        (vec![Q0], vec![3]),
        "the preset z_q0 × z_q1"
    );
    let f = after.sim.slice.frame;
    assert_eq!(f.tilt_x, f.normal, "X was not reset to the slice normal");
    assert_eq!(
        f.normal as usize, Q3,
        "the normal moved, the plane does not take it"
    );
    check_off_the_plane(&after);
    // The angles are 0: the basis is the preset's, untilted (each along one axis, γ = 0).
    assert!(q1[Q0] > 0.0 && q2[3] > 0.0);
}

/// The preset z_q3 × z_μ1 reached from z_α × z_μ1, the normal z_q3: the normal moves off the plane by the slice-axis
/// rule (z_q3 is shown, so the first axis the plane does not show, z_α), the targets equal to z_q3 follow it, one the
/// user moved off it keeps its own; all in one entry, undone by one Ctrl+Z.
fn check_normal_moved(n: usize, entries: i64, after: &Snapshot) {
    assert_eq!(
        (n, entries),
        (1, 1),
        "the preset change is one SetField, one entry"
    );
    assert_eq!(
        (support(&after.sim.plane.q1), support(&after.sim.plane.q2)),
        (vec![Q3], vec![MU1]),
        "the preset z_q3 × z_μ1"
    );
    assert_eq!(
        after.sim.slice.frame,
        frame(ALPHA, ALPHA, MU2),
        "the normal by the slice-axis rule, X following it, Y kept"
    );
    check_off_the_plane(after);
}

#[test]
fn qa_tilt_targets_a_preset_taking_the_normal_moves_it_in_the_same_entry() {
    let mut rig = mock();
    rig.set(SimField::SliceFrame {
        frame: frame(Q3, Q3, MU2),
        basis: None,
    });
    rig.set(SimField::Basis {
        q1: e(ALPHA, 1.0),
        q2: e(MU1, 1.0),
    });
    let id = preset(&rig);
    rig.focus(id);
    let (n, entries, before, after) = gesture(&mut rig, |r| r.key(Key::Enter, NONE));
    check_normal_moved(n, entries, &after);
    rig.key(Key::Z, Modifiers::CTRL);
    let undone = rig.snap();
    assert_eq!(
        undone.sim.plane, before.sim.plane,
        "one Ctrl+Z did not restore the preset"
    );
    assert_eq!(
        undone.sim.slice.frame,
        frame(Q3, Q3, MU2),
        "one Ctrl+Z did not restore the normal and the targets"
    );
    // Control: a preset change that leaves the normal in the plane.
    let mut left = after.clone();
    left.sim.slice.frame = frame(Q3, Q3, MU2);
    rejects(
        "a preset change that leaves the normal in the plane",
        || check_normal_moved(n, entries, &left),
    );
    rejects("the normal in the plane", || check_off_the_plane(&left));
}

/// Types `value` into the basis editor's field `id` (Chart's "edit…", drawn while the focus is in it): one edit.
fn type_component(rig: &mut Rig, id: &str, value: &str) -> (usize, i64, Snapshot, Snapshot) {
    rig.focus(id);
    let at = rig
        .app
        .keyboard
        .place_of(id)
        .unwrap_or_else(|| panic!("the field {id} is not drawn"))
        .hit
        .center();
    gesture(rig, |r| {
        r.click(at);
        r.type_in(value);
    })
}

#[test]
fn qa_tilt_targets_a_chart_basis_edit_taking_the_normal_moves_it() {
    // Chart's q₂ editor, typed: q₂ from e_β to e_q3, so the plane becomes z_α × z_q3, which takes the normal z_q3. Its
    // z_q3 typed first, 1: q₂ = e_β + e_q3, a plane z_α × z_β tilted into z_q3, which leaves the normal where it is;
    // then its z_β, 0: the plane z_α × z_q3, and that edit moves the normal by the slice-axis rule, to z_β, the first
    // axis the plane does not show, X and Y following it, in its own one entry.
    let mut rig = mock();
    let q3 = rig.scope("q₂ edit…", AXES[Q3]);
    let beta = rig.scope("q₂ edit…", AXES[BETA]);
    let (n, entries, _, after) = type_component(&mut rig, q3, "1");
    assert_eq!((n, entries), (1, 1), "typing q₂'s z_q3");
    assert_eq!(after.sim.plane.q2[Q3], 1.0, "q₂ {:?}", after.sim.plane.q2);
    assert_eq!(
        after.sim.slice.frame,
        frame(Q3, Q3, Q3),
        "a plane that does not take the normal moved it"
    );
    let (n, entries, before, after) = type_component(&mut rig, beta, "0");
    assert_eq!(
        (n, entries),
        (1, 1),
        "the edit moving the normal is one SetField, one entry"
    );
    assert_eq!(
        (support(&after.sim.plane.q1), support(&after.sim.plane.q2)),
        (vec![ALPHA], vec![Q3]),
        "the basis edit: q₂ {:?}",
        after.sim.plane.q2
    );
    assert_eq!(
        after.sim.slice.frame,
        frame(BETA, BETA, BETA),
        "the normal by the slice-axis rule"
    );
    check_off_the_plane(&after);
    rig.key(Key::Escape, NONE);
    rig.key(Key::Z, Modifiers::CTRL);
    let undone = rig.snap();
    assert_eq!(undone.sim.plane, before.sim.plane, "one Ctrl+Z: the basis");
    assert_eq!(
        undone.sim.slice.frame,
        frame(Q3, Q3, Q3),
        "one Ctrl+Z: the normal and the targets"
    );
}

// === REQ-GUI-205: the readout ========================================================================================

/// The compass's readout names `τ₁ → <X> <angle>°` and `τ₂ → <Y> <angle>°` (R-457 E6).
fn check_readout(texts: &[(String, Rect)], t1: &str, t2: &str) {
    let compass = inside(texts, layout().compass);
    for want in [t1, t2] {
        assert!(
            compass
                .iter()
                .any(|(t, _)| t.replace('−', "-").contains(want)),
            "no `{want}` in the compass's readout: {:?}",
            compass.iter().map(|(t, _)| t).collect::<Vec<_>>()
        );
    }
}

#[test]
fn qa_tilt_targets_the_readout_names_both_targets_with_their_angles_in_either_mode() {
    let mut rig = mock();
    tilted(&mut rig, frame(Q3, Q3, MU1), 12.0, 0.0);
    rig.settle();
    let tilt_mode = rig.texts();
    check_readout(&tilt_mode, "τ₁ → z_q3 +12.0°", "τ₂ → z_μ1 0.0°");
    // Slice mode, by a Slice centre slider touched (R-409): the targets still named.
    let slider = rig.scope("Slice centre", AXES[MU2]);
    rig.interact(slider);
    rig.key(Key::ArrowUp, NONE);
    rig.key(Key::Escape, NONE);
    rig.settle();
    let slice_mode = rig.texts();
    check_readout(&slice_mode, "τ₁ → z_q3 +12.0°", "τ₂ → z_μ1 0.0°");
    // Untilted, slice mode: both still named, at 0.
    let mut rig = mock();
    rig.set(SimField::SliceFrame {
        frame: frame(Q3, MU2, MU1),
        basis: None,
    });
    let slider = rig.scope("Slice centre", AXES[ALPHA]);
    rig.interact(slider);
    rig.key(Key::ArrowUp, NONE);
    rig.settle();
    check_readout(&rig.texts(), "τ₁ → z_μ2 0.0°", "τ₂ → z_μ1 0.0°");
    // Controls: the targets swapped; the one shared target.
    rejects("the targets swapped", || {
        check_readout(&tilt_mode, "τ₁ → z_μ1 +12.0°", "τ₂ → z_q3 0.0°")
    });
    rejects("one shared target", || {
        check_readout(&tilt_mode, "τ₁ → z_q3 +12.0°", "τ₂ → z_q3 0.0°")
    });
}

// === REQ-GUI-204: the sections ======================================================================================

/// Navigate holds no single-coordinate z₀ box: no field named by an axis among its texts or its scopes, which are
/// centre u, centre v, zoom and the lock row's unlock and open in Inspector.
fn check_navigate(texts: &[(String, Rect)], scopes: &[&str]) {
    for name in AXES {
        assert!(
            !texts.iter().any(|(t, _)| t == name),
            "a z₀ box `{name}` under Navigate"
        );
    }
    let mut got = scopes.to_vec();
    got.sort_unstable();
    let mut want = vec![
        "centre u",
        "centre v",
        "zoom",
        "unlock",
        "open in Inspector",
    ];
    want.sort_unstable();
    assert_eq!(got, want, "Navigate's widgets");
}

#[test]
fn qa_mock_manifold_sections_navigate_has_centre_zoom_depth_and_lock_and_no_z0_boxes() {
    let mut rig = mock();
    let texts = rig.texts();
    let nav = section_texts(&texts, "Navigate");
    let id = rig.find(None, "Navigate");
    let scopes = rig.child_labels(id);
    check_navigate(&nav, &scopes);
    for want in ["centre u, v", "zoom", "depth"] {
        assert!(
            nav.iter().any(|(t, _)| t.contains(want)),
            "no `{want}` in Navigate: {nav:?}"
        );
    }
    // The lock row: locked, its badge with unlock in Navigate.
    let f = layout().figure;
    let _ = rig.frame(vec![Event::PointerMoved(f.center())]);
    rig.key(Key::K, NONE);
    let nav = section_texts(&rig.texts(), "Navigate");
    assert!(
        nav.iter().any(|(t, _)| t.contains("locked")),
        "no lock row in Navigate: {nav:?}"
    );
    // Centre u and v pan within the plane: on a tilted plane their ↑ moves z₀ along q₁ or q₂, two coordinates, never
    // one alone.
    let mut rig = mock();
    tilted(&mut rig, frame(Q3, Q3, MU1), 30.0, 20.0);
    for (label, q) in [("centre u", 0), ("centre v", 1)] {
        let id = rig.scope("Navigate", label);
        rig.interact(id);
        let (n, _, before, after) = gesture(&mut rig, |r| r.key(Key::ArrowUp, NONE));
        rig.key(Key::Escape, NONE);
        assert_eq!(n, 1, "{label}: one SetField");
        let dz = add(&after.sim.plane.z0, -1.0, &before.sim.plane.z0);
        let qv = [before.sim.plane.q1, before.sim.plane.q2][q];
        let k = dot(&dz, &qv) / dot(&qv, &qv);
        assert!(
            k != 0.0 && max_diff(&dz, &scaled(&qv, k)) < 1e-12,
            "{label}: Δz₀ {dz:?} is not along q"
        );
        assert_eq!(support(&dz).len(), 2, "{label}: Δz₀ {dz:?}");
    }
    // Control: Navigate with the eight z₀ boxes.
    let mut with_grid = nav.clone();
    for name in AXES {
        with_grid.push((name.to_owned(), Rect::NOTHING));
    }
    rejects("the eight z₀ boxes under Navigate", || {
        check_navigate(&with_grid, &scopes)
    });
    let mut grid_scopes = scopes.clone();
    grid_scopes.extend(AXES);
    rejects("the eight z₀ scopes under Navigate", || {
        check_navigate(&[], &grid_scopes)
    });
}

#[test]
fn qa_mock_manifold_sections_slice_centre_eight_sliders_each_number_box_edits_its_component() {
    let mut rig = mock();
    let id = rig.find(None, "Slice centre");
    assert_eq!(
        rig.child_labels(id),
        AXES,
        "Slice centre's eight sliders, in order"
    );
    for (i, name) in AXES.iter().enumerate() {
        // Typed into the number box beside the slider: the value on the slider's row, right of its name.
        let texts = rig.texts();
        let centre = section_texts(&texts, "Slice centre");
        let label = centre
            .iter()
            .find(|(t, _)| t == name)
            .unwrap_or_else(|| panic!("no slider {name}"))
            .1;
        let boxed = centre
            .iter()
            .filter(|(t, r)| {
                (r.center().y - label.center().y).abs() < label.height()
                    && r.min.x > label.max.x
                    && number(t).is_some()
            })
            .max_by(|a, b| a.1.min.x.total_cmp(&b.1.min.x))
            .unwrap_or_else(|| panic!("no number box beside {name}"))
            .1;
        let want = -0.375 + 0.0625 * i as f64;
        let (n, entries, before, after) = gesture(&mut rig, |r| {
            r.click(boxed.center());
            r.type_in(&format!("{want}"));
        });
        assert!(
            n >= 1 && entries == 1,
            "{name}: typing is one entry ({n}, {entries})"
        );
        for j in 0..8 {
            let (b, a) = (before.sim.plane.z0[j], after.sim.plane.z0[j]);
            if j == i {
                assert!(
                    (a - want).abs() < 1e-12,
                    "{name}'s box set z₀[{j}] to {a}, not {want}"
                );
            } else {
                assert_eq!(a, b, "{name}'s box moved z₀[{j}]");
            }
        }
        assert_eq!(after.sim.plane.q1, before.sim.plane.q1);
        assert_eq!(after.sim.plane.q2, before.sim.plane.q2);
    }
}

/// Tilt's values are τ₁, τ₂ and γ, its menus the two targets, and nothing named a step.
fn check_tilt(scopes: &[&str]) {
    assert_eq!(
        scopes,
        ["τ₁", "tilt τ₁ into", "τ₂", "tilt τ₂ into", "γ"],
        "Tilt's widgets"
    );
}

#[test]
fn qa_mock_manifold_sections_tilt_has_tau1_tau2_gamma_with_number_boxes_and_nothing_editing_z0() {
    let mut rig = mock();
    let id = rig.find(None, "Tilt");
    let scopes = rig.child_labels(id);
    check_tilt(&scopes);
    // No widget there edits z₀: each value's ↑ edits the basis alone, each target pick the frame (and the basis).
    let tree_values: Vec<&'static str> = {
        let tree = rig.app.keyboard.tree(Mode::Explore);
        tree.children(Some(id))
            .into_iter()
            .filter(|c| matches!(tree.get(c).unwrap().arrows, Arrows::Adjust(_)))
            .collect()
    };
    assert_eq!(tree_values.len(), 3, "τ₁, τ₂, γ");
    tilted(&mut rig, frame(Q3, Q3, Q3), 10.0, 10.0);
    for v in tree_values {
        rig.interact(v);
        let (n, _, before, after) = gesture(&mut rig, |r| r.key(Key::ArrowUp, NONE));
        rig.key(Key::Escape, NONE);
        assert_eq!(n, 1);
        assert_eq!(
            after.sim.plane.z0, before.sim.plane.z0,
            "a Tilt value edited z₀"
        );
        assert_ne!(
            (after.sim.plane.q1, after.sim.plane.q2),
            (before.sim.plane.q1, before.sim.plane.q2),
            "a Tilt value left the basis"
        );
    }
    for menu in ["tilt τ₁ into", "tilt τ₂ into"] {
        let entry = rig.scope(menu, AXES[MU2]);
        rig.focus(entry);
        let (_, _, before, after) = gesture(&mut rig, |r| r.key(Key::Enter, NONE));
        assert_eq!(after.sim.plane.z0, before.sim.plane.z0, "{menu} edited z₀");
    }
    // Drawn: τ₁, τ₂ and γ, each with its number box in degrees, and no step.
    let texts = rig.texts();
    let tilt = section_texts(&texts, "Tilt");
    for name in ["τ₁", "τ₂", "γ"] {
        let label = tilt
            .iter()
            .find(|(t, _)| t == name)
            .unwrap_or_else(|| panic!("no {name} in Tilt: {tilt:?}"))
            .1;
        // The box's value and its unit, "11.0" and "°", drawn on the slider's row, right of its name.
        let on_row = |t: &str| {
            tilt.iter().any(|(s, r)| {
                (r.center().y - label.center().y).abs() < label.height()
                    && r.min.x > label.max.x
                    && (if t == "°" {
                        s.ends_with('°')
                    } else {
                        number(s).is_some()
                    })
            })
        };
        assert!(
            on_row("number") && on_row("°"),
            "no number box in degrees beside {name}: {tilt:?}"
        );
    }
    assert!(
        !tilt.iter().any(|(t, _)| t.to_lowercase().contains("step")),
        "a step in Tilt"
    );
    // τ₁'s number box, typed: τ₁ = 40° into z_μ2, from τ₁ = τ₂ = 10°, γ = 0.
    tilted(&mut rig, frame(Q3, MU2, MU2), 10.0, 10.0);
    let tilt = section_texts(&rig.texts(), "Tilt");
    let label = tilt.iter().find(|(t, _)| t == "τ₁").unwrap().1;
    let boxed = tilt
        .iter()
        .filter(|(t, r)| {
            (r.center().y - label.center().y).abs() < label.height()
                && r.min.x > label.max.x
                && number(t).is_some()
        })
        .max_by(|a, b| a.1.min.x.total_cmp(&b.1.min.x))
        .unwrap()
        .1;
    let (_, entries, _, after) = gesture(&mut rig, |r| {
        r.click(boxed.center());
        r.type_in("40");
    });
    assert_eq!(entries, 1, "typing τ₁ is one entry");
    let q1 = after.sim.plane.q1;
    let tau1 = q1[MU2].atan2(q1[ALPHA]).to_degrees();
    assert!((tau1 - 40.0).abs() < 1e-9, "τ₁'s box set τ₁ to {tau1}°");
    // Controls: a Tilt section with a step slider; the old section.
    rejects("a Tilt section with a step slider", || {
        check_tilt(&[
            "slice step",
            "τ₁",
            "tilt τ₁ into",
            "τ₂",
            "tilt τ₂ into",
            "γ",
        ])
    });
    rejects("the old Slice & tilt", || {
        check_tilt(&["slice step", "τ₁", "τ₂", "γ"])
    });
}

/// The names: §G3's sub-scopes in order, the panel's titles, and none of the old names anywhere in the panel or tree.
fn check_names(subs: &[&str], titles: &[&str], all_labels: &[&str]) {
    assert_eq!(subs, SECTIONS, "Manifold view's sub-scopes");
    for t in SECTIONS {
        assert!(titles.contains(&t), "no section titled {t}: {titles:?}");
    }
    for old in OLD_NAMES {
        assert!(
            !titles.contains(&old) && !all_labels.contains(&old),
            "the old name `{old}`"
        );
    }
}

#[test]
fn qa_mock_manifold_sections_names_breadcrumb_and_keys() {
    let mut rig = mock();
    let mv = rig.find(None, "Manifold view");
    let subs = rig.child_labels(mv);
    let texts = rig.texts();
    let titles: Vec<&str> = inside(&texts, layout().manifold_view)
        .iter()
        .map(|(t, _)| t.as_str())
        .filter(|t| SECTIONS.contains(t) || OLD_NAMES.contains(t))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|t| {
            *SECTIONS
                .iter()
                .chain(&OLD_NAMES)
                .find(|s| **s == t)
                .unwrap()
        })
        .collect();
    let all_labels: Vec<&str> = {
        let tree = rig.app.keyboard.tree(Mode::Explore);
        let mut all: Vec<&'static str> = tree.children(None);
        let mut i = 0;
        while i < all.len() {
            let kids = tree.children(Some(all[i]));
            all.extend(kids);
            i += 1;
        }
        all.into_iter()
            .map(|id| tree.get(id).unwrap().label)
            .collect()
    };
    check_names(&subs, &titles, &all_labels);
    // The breadcrumb on τ₁.
    let tau1 = rig.scope("Tilt", "τ₁");
    rig.focus(tau1);
    let crumb = ["Manifold view", "Tilt", "τ₁"].join(SEP);
    let top = inside(&rig.texts(), layout().top_bar);
    assert!(
        top.iter().any(|(t, _)| t.contains(&crumb)),
        "no breadcrumb `{crumb}` in the top bar: {top:?}"
    );
    // Enter on Manifold view, then ↓: Chart, Navigate, Slice centre, Tilt.
    let mut rig = mock();
    let mv = rig.find(None, "Manifold view");
    rig.focus(mv);
    rig.key(Key::Enter, NONE);
    let mut seen = vec![rig.labels()];
    for _ in 0..3 {
        rig.key(Key::ArrowDown, NONE);
        seen.push(rig.labels());
    }
    for (path, sub) in seen.iter().zip(SECTIONS) {
        assert_eq!(path, &["Manifold view", sub], "Enter, then ↓");
    }
    // Controls: the old names, as sub-scopes and as titles.
    rejects("the old sub-scope names", || {
        check_names(
            &["Chart", "Navigate", "Centre z₀", "Slice & tilt"],
            &titles,
            &all_labels,
        )
    });
    let mut old_titles = titles.clone();
    old_titles.push("Slice & tilt");
    rejects("an old title in the panel", || {
        check_names(&subs, &old_titles, &all_labels)
    });
}

#[test]
fn qa_tilt_targets_a_retilt_keeps_the_other_angle_gamma_and_the_scale() {
    // From X = Y = z_q3 at τ₁ = 30°, τ₂ = 20°, γ = 15°, σ = 0.5 (the old map, set through the contract), X picked as
    // z_q2: the basis is R-457 E1's map at the same angles and scale with X = z_q2, Y = z_q3.
    let mut rig = mock();
    rig.set(SimField::SliceFrame {
        frame: frame(Q3, Q3, Q3),
        basis: None,
    });
    let (t1, t2, g, s) = (
        30f64.to_radians(),
        20f64.to_radians(),
        15f64.to_radians(),
        0.5,
    );
    let (q1, q2) = old_map(ALPHA, BETA, Q3, t1, t2, g, s);
    rig.set(SimField::Basis { q1, q2 });
    let entry = rig.scope("tilt τ₁ into", AXES[Q2]);
    rig.focus(entry);
    let (n, entries, _, after) = gesture(&mut rig, |r| r.key(Key::Enter, NONE));
    assert_eq!((n, entries), (1, 1));
    let u1 = add(&e(ALPHA, t1.cos()), t1.sin(), &e(Q2, 1.0));
    let u2 = add(&e(BETA, t2.cos()), t2.sin(), &e(Q3, 1.0));
    let w1 = scaled(&add(&scaled(&u1, g.cos()), g.sin(), &u2), s);
    let w2 = scaled(&add(&scaled(&u2, g.cos()), -g.sin(), &u1), s);
    let check = |p: &engine::contract::sim_config::Plane| {
        assert!(
            max_diff(&p.q1, &w1) < 1e-12 && max_diff(&p.q2, &w2) < 1e-12,
            "the re-tilt: q₁ {:?} q₂ {:?}, R-457 E1 gives {w1:?} {w2:?}",
            p.q1,
            p.q2
        );
    };
    check(&after.sim.plane);
    assert_eq!(after.sim.slice.frame, frame(Q3, Q2, Q3));
    // Control: the re-tilt that drops γ.
    let mut no_gamma = after.sim.plane.clone();
    (no_gamma.q1, no_gamma.q2) = (scaled(&u1, s), scaled(&u2, s));
    rejects("a re-tilt that drops γ", || check(&no_gamma));
}

/// From the frame `from` on the preset `(a, b)`, Enter on Chart's preset steps to the next: one entry, the frame then.
fn preset_step(from: SliceFrame, a: usize, b: usize) -> Snapshot {
    let mut rig = mock();
    rig.set(SimField::SliceFrame {
        frame: from,
        basis: None,
    });
    rig.set(SimField::Basis {
        q1: e(a, 1.0),
        q2: e(b, 1.0),
    });
    let id = preset(&rig);
    rig.focus(id);
    let (n, entries, _, after) = gesture(&mut rig, |r| r.key(Key::Enter, NONE));
    assert_eq!(
        (n, entries),
        (1, 1),
        "the preset step is one SetField, one entry"
    );
    after
}

#[test]
fn qa_tilt_targets_a_plane_taking_a_target_resets_it_to_the_normal_moved_first() {
    // A target alone taken: normal z_q0, X = z_q3, Y = z_μ2 on z_α × z_μ1; the next preset, z_q3 × z_μ1, takes X, which
    // resets to the normal, z_q0; Y, off the plane, keeps its own.
    let after = preset_step(frame(Q0, Q3, MU2), ALPHA, MU1);
    assert_eq!(
        (support(&after.sim.plane.q1), support(&after.sim.plane.q2)),
        (vec![Q3], vec![MU1])
    );
    assert_eq!(
        after.sim.slice.frame,
        frame(Q0, Q0, MU2),
        "X taken: reset to the normal; Y kept"
    );
    // The normal and a target taken together: normal z_q3, X = z_q2, Y = z_μ2 on z_q0 × z_q1; the next preset,
    // z_q2 × z_q3, takes both: the normal moves first, by the slice-axis rule, to z_α (z_q3 is shown), and X resets to
    // it, not to z_q3; Y keeps its own.
    let after = preset_step(frame(Q3, Q2, MU2), Q0, 3);
    assert_eq!(
        (support(&after.sim.plane.q1), support(&after.sim.plane.q2)),
        (vec![Q2], vec![Q3])
    );
    assert_eq!(
        after.sim.slice.frame,
        frame(ALPHA, ALPHA, MU2),
        "the normal moved first, X reset to it"
    );
    check_off_the_plane(&after);
    // Control: X reset to the normal before it moved, z_q3, which the plane takes.
    let mut stale = after.clone();
    stale.sim.slice.frame = frame(ALPHA, Q3, MU2);
    rejects("a target reset to the normal before it moved", || {
        check_off_the_plane(&stale)
    });
}
