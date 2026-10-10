//! QA tests for TASK-M6-26, written from the requirements and their sources, not from the implementation:
//!
//! - REQ-GUI-170: on the mock, the Manifold view group and the compass as `01_main.png`, `08_lock.png` and the design
//!   notes give them: Chart, Navigate, centre z₀, Slice & tilt and rotation as one group, every edit a SetField on z₀
//!   or the basis; lock re-basing the sliders to the anchor (anchor plus offset), not freezing them; the compass
//!   switching between slicing and tilting by which slider was touched, with the gold pin when locked. Verify: "each
//!   control emits a SetField on z₀ or the basis, and after lock a slider reads anchor plus offset and moving it emits
//!   an excursion". render_gui_spec §G4: locking (K) recentres the view on the point and marks it with a gold reticle
//!   at the centre; `z_locked = z₀ + (2s−1)q₁ + (2t−1)q₂` on an affine chart; lock and unlock are undoable (R-69);
//!   unlocking leaves the view as it stands; the badge offers unlock and open in Inspector.
//! - REQ-GUI-171: pan and zoom are SetFields on z₀ and the basis (no camera), and the mock redraws the stand-in for
//!   the new chart. Verify: "a pan and a zoom each emit one SetField on z₀ or the basis and no other edit; the mock's
//!   next frame of the stand-in is the previous one shifted or scaled accordingly, within one pixel".
//! - REQ-GUI-081: the left panel is one 'Manifold view' group holding Chart, Navigate, the depth readout with its
//!   precision warning, Lock, Centre z₀ (eight sliders z_α, z_β, z_q0…z_q3, z_μ1, z_μ2) and Slice & tilt (slice step,
//!   τ₁, τ₂, rotation γ), in that order. R-54: the precision warning is raised by events, never by a fixed depth.
//! - REQ-GUI-091: the compass sits bottom left, shows the slice plane, switches mode by itself (slicing when a slice
//!   slider is touched, tilting when a tilt is touched), carries a gold pin when locked, and reads out the tilt and
//!   rotation angles. §G3: Compass — arrows tilt, Shift+arrows orbit.
//! - REQ-GUI-169, re-checked for this screen: Enter reaches Chart, Navigate, Centre z₀ and Slice & tilt and each
//!   control inside them; Esc backs out one level; Shift ×10 and Alt ×0.1; the breadcrumb names the path, §G3's own
//!   example "Manifold view › Navigate › zoom"; Figure — arrows pan, + / − zoom.
//!
//! The SetFields a gesture emits are counted from the contract's own log (gui_state_contract §2: every engine logs each
//! SetField it applies as one `info` entry from `contract`), and what they edited is read from the snapshots before and
//! after. gui takes no dependency on validation (R-187), so each check runs its own control (R-176) through
//! [`rejects`].
// The file name `qa_TASK-M6-26` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::sync::Arc;

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::log::Source;
use engine::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice,
};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::{Mode, Window};
use gui::app::App;
use gui::capture::{render_frame, Shot, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::headless::Headless;
use gui::keyboard::scopes::Arrows;
use gui::layout::Layout;
use gui::mock::canvas::MockCanvas;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide, RealSide};

type Z = [f64; 8];

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

/// §G2's eight `z₀` names, in chart_decoder_contract Part 2's block order.
const AXES: [&str; 8] = ["z_α", "z_β", "z_q0", "z_q1", "z_q2", "z_q3", "z_μ1", "z_μ2"];
/// §G3's sub-scopes of Manifold view, in order.
const SECTIONS: [&str; 4] = ["Chart", "Navigate", "Centre z₀", "Slice & tilt"];
/// The lock badge's text, render_gui_spec §G2 and §G4.
const BADGE: &str = "● locked at z_locked";

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

// --- the rig --------------------------------------------------------------------------------------------------------

fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

struct Rig<S: EngineSide> {
    app: App<S>,
    h: Headless,
}

fn mock() -> Rig<MockSide> {
    Rig::new(App::new(MockSide::new(MockEngine::frozen(), None), FORMAT))
}

impl<S: EngineSide> Rig<S> {
    fn new(app: App<S>) -> Self {
        let mut rig = Self { app, h: headless() };
        rig.idle(2);
        rig
    }

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

    /// The SetFields the engine has applied so far: its `contract` log entries.
    fn applied(&self) -> usize {
        self.app
            .console()
            .iter()
            .filter(|e| e.source == Source::Contract)
            .count()
    }

    /// Puts the focus on scope `id`, by its path in the tree.
    fn focus(&mut self, id: &str) {
        let path = self.app.keyboard.tree(Mode::Explore).path_to(id);
        assert!(!path.is_empty(), "no scope {id} in the tree");
        self.app.view.focus.path = path;
    }

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

    /// The shapes drawn this frame, flattened.
    fn shapes(&mut self) -> Vec<egui::Shape> {
        let out = self.frame(Vec::new());
        let mut flat = Vec::new();
        for clipped in &out.shapes {
            flatten(&clipped.shape, &mut flat);
        }
        flat
    }

    fn drag(&mut self, from: Pos2, to: Pos2, steps: usize) {
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: NONE,
        };
        let _ = self.frame(vec![Event::PointerMoved(from)]);
        let _ = self.frame(vec![button(from, true)]);
        for i in 1..=steps {
            let at = from + (to - from) * (i as f32 / steps as f32);
            let _ = self.frame(vec![Event::PointerMoved(at)]);
        }
        let _ = self.frame(vec![button(to, false)]);
        self.idle(2);
    }

    fn wheel(&mut self, at: Pos2, lines: f32) {
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        let _ = self.frame(vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, lines),
            phase: egui::TouchPhase::Move,
            modifiers: NONE,
        }]);
        self.idle(2);
    }

    /// K with the pointer over `at`.
    fn lock_at(&mut self, at: Pos2) {
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        self.key(Key::K, NONE);
    }
}

fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
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

// --- the chart, from chart_decoder_contract Part 3, written here --------------------------------------------------

fn add(a: &Z, k: f64, b: &Z) -> Z {
    std::array::from_fn(|i| a[i] + k * b[i])
}

fn dot(a: &Z, b: &Z) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm(a: &Z) -> f64 {
    dot(a, a).sqrt()
}

/// `z(s, t) = z₀ + (2s−1) q₁ + (2t−1) q₂`.
fn point(p: &Plane, s: f64, t: f64) -> Z {
    add(&add(&p.z0, 2.0 * s - 1.0, &p.q1), 2.0 * t - 1.0, &p.q2)
}

/// The figure's `(s, t)` of a point in points: `s` left to right, `t` bottom to top (Y-up).
fn st(figure: Rect, at: Pos2) -> (f64, f64) {
    (
        f64::from((at.x - figure.min.x) / figure.width()),
        f64::from((figure.max.y - at.y) / figure.height()),
    )
}

fn close(a: &Z, b: &Z, tol: f64) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
}

/// The part of `d` outside span(q₁, q₂).
fn out_of_plane(p: &Plane, d: &Z) -> f64 {
    let (g11, g12, g22) = (dot(&p.q1, &p.q1), dot(&p.q1, &p.q2), dot(&p.q2, &p.q2));
    let (c1, c2) = (dot(d, &p.q1), dot(d, &p.q2));
    let det = g11 * g22 - g12 * g12;
    let (a, b) = ((g22 * c1 - g12 * c2) / det, (g11 * c2 - g12 * c1) / det);
    norm(&add(&add(d, -a, &p.q1), -b, &p.q2))
}

// --- what an edit changed -----------------------------------------------------------------------------------------

/// Which parts of the state differ between two snapshots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Changed {
    z0: bool,
    basis: bool,
    lock: bool,
    render: bool,
}

fn changed(a: &Snapshot, b: &Snapshot) -> Changed {
    Changed {
        z0: a.sim.plane.z0 != b.sim.plane.z0,
        basis: (a.sim.plane.q1, a.sim.plane.q2) != (b.sim.plane.q1, b.sim.plane.q2),
        lock: a.sim.lock != b.sim.lock,
        render: a.render != b.render,
    }
}

/// The gesture `act` applied exactly one SetField, one undo entry, on z₀ or the basis and on nothing else.
fn check_one_set_field_on_z0_or_basis<S: EngineSide>(
    rig: &mut Rig<S>,
    what: &str,
    act: impl FnOnce(&mut Rig<S>),
) -> Changed {
    let (before, n0) = (rig.snap(), rig.applied());
    act(rig);
    let after = rig.snap();
    let c = changed(&before, &after);
    assert_eq!(
        rig.applied() - n0,
        1,
        "{what}: {} SetFields applied, the gesture is one",
        rig.applied() - n0
    );
    assert_eq!(
        after.history.undo_depth,
        before.history.undo_depth + 1,
        "{what}: one undoable entry (R-69)"
    );
    assert!(
        (c.z0 || c.basis) && !c.lock && !c.render,
        "{what}: the edit is not on z₀ or the basis alone: {c:?}"
    );
    c
}

// --- REQ-GUI-170: every control of the group is a SetField on z₀ or the basis ----------------------------------------

/// Every scope under `root`, depth first, in the tree's order.
fn scopes_under(rig: &Rig<MockSide>, root: &str) -> Vec<&'static str> {
    let tree = rig.app.keyboard.tree(Mode::Explore);
    let mut out = Vec::new();
    let mut stack: Vec<&'static str> = tree.children(Some(root)).into_iter().rev().collect();
    while let Some(id) = stack.pop() {
        out.push(id);
        stack.extend(tree.children(Some(id)).into_iter().rev());
    }
    out
}

fn manifold_view_id(rig: &Rig<MockSide>) -> &'static str {
    let tree = rig.app.keyboard.tree(Mode::Explore);
    tree.children(None)
        .into_iter()
        .find(|id| tree.get(id).is_some_and(|s| s.label == "Manifold view"))
        .expect("a Manifold view scope")
}

#[test]
fn qa_mock_manifold_view_every_value_emits_one_set_field_on_z0_or_basis() {
    let probe = mock();
    let mv = manifold_view_id(&probe);
    let tree = probe.app.keyboard.tree(Mode::Explore);
    let values: Vec<(&str, &str)> = scopes_under(&probe, mv)
        .into_iter()
        .filter(|id| matches!(tree.get(id).unwrap().arrows, Arrows::Adjust(_)))
        .map(|id| (id, tree.get(id).unwrap().label))
        .collect();
    // §G2's values: the centre (u, v), the zoom, eight z₀ in Navigate and eight in Centre z₀, the slice step, τ₁, τ₂, γ.
    let labels: Vec<&str> = values.iter().map(|(_, l)| *l).collect();
    for name in AXES {
        assert!(
            labels.iter().filter(|l| **l == name).count() >= 2,
            "{name}: Navigate's field and Centre z₀'s slider: {labels:?}"
        );
    }
    for want in ["zoom", "slice step", "τ₁", "τ₂", "γ"] {
        assert!(
            labels.iter().any(|l| l.contains(want)),
            "no {want} value: {labels:?}"
        );
    }
    for (id, label) in &values {
        for (key, modifiers) in [(Key::ArrowUp, NONE), (Key::ArrowDown, SHIFT)] {
            let mut rig = mock();
            rig.focus(id);
            check_one_set_field_on_z0_or_basis(&mut rig, &format!("{key:?} on {label}"), |r| {
                r.key(key, modifiers)
            });
        }
    }
    // The preset, a control Enter activates, is one basis edit.
    let preset = scopes_under(&probe, mv)
        .into_iter()
        .find(|id| tree.get(id).unwrap().label.contains("preset"))
        .expect("Chart's preset");
    let mut rig = mock();
    rig.focus(preset);
    let c = check_one_set_field_on_z0_or_basis(&mut rig, "Enter on the preset", |r| {
        r.key(Key::Enter, NONE)
    });
    assert!(c.basis, "a preset is a basis");
    // Control: a lock edit is not a SetField on z₀ or the basis.
    let mut rig = mock();
    rejects("a lock edit", || {
        check_one_set_field_on_z0_or_basis(&mut rig, "a lock edit", |r| {
            let z_locked = r.snap().sim.plane.z0;
            r.app.set_field(SetField {
                edit: Edit::Sim(SimField::Lock(Lock {
                    locked: true,
                    z_locked,
                })),
                no_history: false,
            });
            r.idle(2);
        });
    });
}

// --- REQ-GUI-170 and §G4: K locks at the point, recentres, re-bases the sliders ------------------------------------

/// The pointer's point in the figure the lock tests use: off centre in both directions.
fn lock_point(figure: Rect) -> Pos2 {
    figure.min + egui::vec2(figure.width() * 0.3, figure.height() * 0.25)
}

/// K over [`lock_point`] on a fresh mock; returns the rig and the snapshot before.
fn locked_rig() -> (Rig<MockSide>, Snapshot) {
    let mut rig = mock();
    let before = rig.snap();
    rig.lock_at(lock_point(layout().figure));
    (rig, before)
}

/// The lock after K over `at` on `before`'s chart: the anchor is the chart point under the pointer, the view recentred
/// on it, the basis as it was.
fn check_lock(before: &Snapshot, after: &Snapshot, figure: Rect, at: Pos2) {
    let (s, t) = st(figure, at);
    let want = point(&before.sim.plane, s, t);
    assert!(after.sim.lock.locked, "K did not lock");
    // A point's position on the figure is known to a pixel; the anchor to that.
    let tol = 2.0 * norm(&before.sim.plane.q1) / f64::from(figure.width() * PIXELS_PER_POINT);
    assert!(
        close(&after.sim.lock.z_locked, &want, tol),
        "z_locked {:?} is not z(s, t) = {want:?} at (s, t) = ({s}, {t})",
        after.sim.lock.z_locked
    );
    assert_eq!(
        after.sim.plane.z0, after.sim.lock.z_locked,
        "the view is not recentred on the anchor"
    );
    assert_eq!(
        (after.sim.plane.q1, after.sim.plane.q2),
        (before.sim.plane.q1, before.sim.plane.q2),
        "locking changed the basis"
    );
}

#[test]
fn qa_mock_lock_badge_k_locks_at_the_point_and_recentres() {
    let figure = layout().figure;
    let (mut rig, before) = locked_rig();
    let after = rig.snap();
    check_lock(&before, &after, figure, lock_point(figure));
    // Control: the anchor read Y-down, the coordinate convention's one flip missed.
    let flipped = figure.min + egui::vec2(figure.width() * 0.3, figure.height() * 0.75);
    rejects("the anchor of the point mirrored in t", || {
        check_lock(&before, &after, figure, flipped)
    });
    // Lock and recentring are undoable (R-69): undoing everything K did returns the free view as it was.
    for _ in before.history.undo_depth..after.history.undo_depth {
        rig.app.undo();
        rig.idle(1);
    }
    let undone = rig.snap();
    assert_eq!(undone.sim, before.sim, "undo did not undo the lock");
}

/// The gold shapes' centres: circles in gold.
fn gold_circles(shapes: &[egui::Shape]) -> Vec<Pos2> {
    let gold = gui::explore::lock::GOLD;
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Circle(c) if c.fill == gold || c.stroke.color == gold => Some(c.center),
            _ => None,
        })
        .collect()
}

fn check_marks(shapes: &[egui::Shape], layout: &Layout, locked: bool) {
    let circles = gold_circles(shapes);
    let centre = layout.figure.center();
    let reticle = circles.iter().any(|c| (*c - centre).length() < 1.0);
    let pin = circles.iter().any(|c| layout.compass.contains(*c));
    assert_eq!(
        (reticle, pin),
        (locked, locked),
        "locked {locked}: the gold reticle at the figure's centre {centre:?} and the gold pin in the compass \
         {:?}: {circles:?}",
        layout.compass
    );
}

#[test]
fn qa_mock_lock_badge_reticle_at_the_centre_and_pin_on_the_compass() {
    let lay = layout();
    let mut free = mock();
    let free_shapes = free.shapes();
    check_marks(&free_shapes, &lay, false);
    let (mut rig, _) = locked_rig();
    let locked_shapes = rig.shapes();
    check_marks(&locked_shapes, &lay, true);
    rejects("the free view read as locked", || {
        check_marks(&free_shapes, &lay, true)
    });
    // The badge reads §G4's text while locked, and not while free; unlock and open in Inspector beside it.
    let texts: Vec<String> = rig.texts().into_iter().map(|(t, _)| t).collect();
    for want in [BADGE, "unlock", "open in Inspector"] {
        assert!(
            texts.iter().any(|t| t == want),
            "no `{want}` while locked: {texts:?}"
        );
    }
    let free_texts: Vec<String> = free.texts().into_iter().map(|(t, _)| t).collect();
    assert!(
        !free_texts.iter().any(|t| t == BADGE),
        "the badge reads locked while free"
    );
    rejects("the free texts read for the locked badge", || {
        assert!(free_texts.iter().any(|t| t == BADGE))
    });
}

/// A number as the panel writes it: a true minus or a plus, spaces ignored.
fn number(text: &str) -> Option<f64> {
    let t: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == '−' { '-' } else { c })
        .collect();
    t.trim_start_matches('+').parse().ok()
}

/// The decimals a number's text shows.
fn decimals(text: &str) -> i32 {
    text.split('.').nth(1).map_or(0, |d| {
        d.chars().take_while(char::is_ascii_digit).count() as i32
    })
}

/// The rows of Centre z₀: each axis label's rect, below the section's title and above Slice & tilt's.
fn centre_rows(texts: &[(String, Rect)], panel: Rect) -> Vec<(usize, Rect)> {
    let title_y = |t: &str| {
        texts
            .iter()
            .find(|(s, r)| s == t && panel.contains_rect(*r))
            .map(|(_, r)| r.center().y)
            .unwrap_or_else(|| panic!("no `{t}` in the panel"))
    };
    let (top, bottom) = (title_y("Centre z₀"), title_y("Slice & tilt"));
    AXES.iter()
        .enumerate()
        .map(|(i, name)| {
            let r = texts
                .iter()
                .find(|(s, r)| s == name && r.center().y > top && r.center().y < bottom)
                .unwrap_or_else(|| panic!("no Centre z₀ row {name}"))
                .1;
            (i, r)
        })
        .collect()
}

/// Each Centre z₀ row shows the offset `δ = z₀ − z_locked` of its axis.
fn check_offsets(texts: &[(String, Rect)], panel: Rect, z0: &Z, anchor: &Z) {
    for (i, row) in centre_rows(texts, panel) {
        let want = z0[i] - anchor[i];
        let shown = texts.iter().any(|(t, r)| {
            (r.center().y - row.center().y).abs() < row.height() / 2.0
                && r.min.x > row.max.x
                && (t.starts_with('+') || t.starts_with('−'))
                && number(t)
                    .is_some_and(|v| (v - want).abs() <= 0.5 * 10f64.powi(-decimals(t)) + 1e-12)
        });
        assert!(
            shown,
            "row {}: no offset reading {want} (anchor plus offset)",
            AXES[i]
        );
    }
}

#[test]
fn qa_mock_manifold_view_locked_sliders_read_anchor_plus_offset_and_move_by_excursion() {
    let panel = layout().manifold_view;
    let (mut rig, _) = locked_rig();
    let s = rig.snap();
    let anchor = s.sim.lock.z_locked;
    check_offsets(&rig.texts(), panel, &s.sim.plane.z0, &anchor);
    // Each slider moves: an excursion, a SetField on z₀ alone, the anchor kept; the row then reads the offset.
    let tree = rig.app.keyboard.tree(Mode::Explore);
    let centre = tree
        .children(Some(manifold_view_id(&rig)))
        .into_iter()
        .find(|id| tree.get(id).unwrap().label == "Centre z₀")
        .expect("Centre z₀");
    let sliders = tree.children(Some(centre));
    assert_eq!(sliders.len(), 8, "eight sliders");
    for (i, slider) in sliders.into_iter().enumerate() {
        let before = rig.snap();
        rig.focus(slider);
        let c =
            check_one_set_field_on_z0_or_basis(&mut rig, AXES[i], |r| r.key(Key::ArrowUp, NONE));
        assert!(c.z0 && !c.basis, "{}: an excursion is on z₀", AXES[i]);
        let after = rig.snap();
        for j in 0..8 {
            assert_eq!(
                after.sim.plane.z0[j] != before.sim.plane.z0[j],
                j == i,
                "{}'s slider moved z₀[{j}]",
                AXES[i]
            );
        }
        assert!(
            after.sim.plane.z0[i] > before.sim.plane.z0[i],
            "↑ moved {} down",
            AXES[i]
        );
        assert_eq!(
            after.sim.lock, before.sim.lock,
            "the anchor moved: frozen or re-locked"
        );
    }
    let after = rig.snap();
    assert!(
        (0..8).all(|i| after.sim.plane.z0[i] != anchor[i]),
        "not every axis left the anchor"
    );
    let texts = rig.texts();
    check_offsets(&texts, panel, &after.sim.plane.z0, &anchor);
    rejects(
        "the offsets read against z₀ instead of the anchor",
        || check_offsets(&texts, panel, &after.sim.plane.z0, &after.sim.plane.z0),
    );
}

#[test]
fn qa_mock_lock_badge_unlock_keeps_the_view_and_inspector_opens_a_window() {
    let (mut rig, _) = locked_rig();
    let mv = manifold_view_id(&rig);
    let find = |rig: &Rig<MockSide>, want: &str| {
        let tree = rig.app.keyboard.tree(Mode::Explore);
        scopes_under(rig, mv)
            .into_iter()
            .find(|id| tree.get(id).unwrap().label == want)
            .unwrap_or_else(|| panic!("no `{want}` scope"))
    };
    // Open in Inspector requests the Inspector window and edits nothing.
    let inspector = find(&rig, "open in Inspector");
    let (before, n0) = (rig.snap(), rig.applied());
    rig.focus(inspector);
    rig.key(Key::Enter, NONE);
    assert_eq!(rig.applied(), n0, "open in Inspector applied a SetField");
    assert_eq!(rig.snap().sim, before.sim);
    assert!(
        rig.app.view.windows.open.contains(&Window::Inspector),
        "the Inspector was not requested"
    );
    // Unlock: one SetField, the lock cleared, the view as it stands the new free start (§G4).
    let unlock = find(&rig, "unlock");
    let (before, n0) = (rig.snap(), rig.applied());
    rig.focus(unlock);
    rig.key(Key::Enter, NONE);
    let after = rig.snap();
    assert_eq!(rig.applied() - n0, 1, "unlock is one SetField");
    assert!(!after.sim.lock.locked, "still locked");
    assert_eq!(
        after.sim.plane, before.sim.plane,
        "unlocking moved the view"
    );
    assert_eq!(
        after.history.undo_depth,
        before.history.undo_depth + 1,
        "unlock is undoable"
    );
    rejects("a lock left set", || assert!(!before.sim.lock.locked));
}

// --- REQ-GUI-171: pan and zoom on the figure ----------------------------------------------------------------------

/// A drag on the figure from `from` to `to`: one SetField on z₀ alone, in the plane, and the picture follows the
/// pointer: the chart point under `from` before is under `to` after.
fn check_pan(before: &Snapshot, after: &Snapshot, figure: Rect, from: Pos2, to: Pos2) {
    let c = changed(before, after);
    assert!(
        c.z0 && !c.basis && !c.lock && !c.render,
        "a pan edits z₀ alone: {c:?}"
    );
    let d = add(&after.sim.plane.z0, -1.0, &before.sim.plane.z0);
    assert!(
        out_of_plane(&before.sim.plane, &d) < 1e-12,
        "a pan left the plane: Δz₀ = {d:?}"
    );
    let (s0, t0) = st(figure, from);
    let (s1, t1) = st(figure, to);
    let grabbed = point(&before.sim.plane, s0, t0);
    let under = point(&after.sim.plane, s1, t1);
    // Within one pixel of the chart: the pointer's position is known to a pixel.
    let pixel = 2.0 * norm(&before.sim.plane.q1) / f64::from(figure.width() * PIXELS_PER_POINT);
    assert!(
        close(&grabbed, &under, pixel),
        "the picture does not follow the pointer: {grabbed:?} grabbed, {under:?} under it after"
    );
}

#[test]
fn qa_mock_figure_navigation_a_pan_is_one_set_field_on_z0() {
    let figure = layout().figure;
    let from = figure.center() + egui::vec2(-30.0, 40.0);
    let to = from + egui::vec2(-50.0, -25.0);
    let mut rig = mock();
    let before = rig.snap();
    check_one_set_field_on_z0_or_basis(&mut rig, "a pan", |r| r.drag(from, to, 1));
    let after = rig.snap();
    check_pan(&before, &after, figure, from, to);
    rejects("a pan read the other way", || {
        check_pan(&before, &after, figure, to, from)
    });
    // A drag over several frames edits z₀ and nothing else, ending where the pointer does.
    let mut rig = mock();
    let (before, n0) = (rig.snap(), rig.applied());
    rig.drag(from, to, 4);
    assert!(rig.applied() > n0, "no pan applied");
    check_pan(&before, &rig.snap(), figure, from, to);
}

/// A zoom about the figure's centre: the basis scaled by `k`, z₀ and the lock kept.
fn check_zoom(before: &Snapshot, after: &Snapshot) -> f64 {
    let c = changed(before, after);
    assert!(
        c.basis && !c.z0 && !c.lock && !c.render,
        "a zoom edits the basis alone: {c:?}"
    );
    let (p, q) = (&before.sim.plane, &after.sim.plane);
    let k = norm(&q.q1) / norm(&p.q1);
    assert!(
        close(&q.q1, &p.q1.map(|v| v * k), 1e-12) && close(&q.q2, &p.q2.map(|v| v * k), 1e-12),
        "a zoom is not a scaling of the basis"
    );
    assert!((k - 1.0).abs() > 1e-6, "no zoom");
    k
}

#[test]
fn qa_mock_figure_navigation_a_zoom_is_one_set_field_on_the_basis() {
    let figure = layout().figure;
    let mut rig = mock();
    let before = rig.snap();
    check_one_set_field_on_z0_or_basis(&mut rig, "the wheel up", |r| r.wheel(figure.center(), 1.0));
    let k_in = check_zoom(&before, &rig.snap());
    let before = rig.snap();
    check_one_set_field_on_z0_or_basis(&mut rig, "the wheel down", |r| {
        r.wheel(figure.center(), -1.0)
    });
    let k_out = check_zoom(&before, &rig.snap());
    assert!(
        k_in < 1.0 && k_out > 1.0,
        "up zooms in, down out: {k_in}, {k_out}"
    );
    // Control: a pan is not a zoom.
    let mut rig = mock();
    let before = rig.snap();
    rig.drag(figure.center(), figure.center() + egui::vec2(10.0, 0.0), 1);
    let after = rig.snap();
    rejects("a pan", || {
        check_zoom(&before, &after);
    });
}

#[test]
fn qa_mock_figure_navigation_figure_keys_pan_and_zoom() {
    let figure_id = || {
        let rig = mock();
        let tree = rig.app.keyboard.tree(Mode::Explore);
        tree.children(None)
            .into_iter()
            .find(|id| tree.get(id).unwrap().label == "Figure")
            .expect("the Figure scope")
    };
    let id = figure_id();
    for key in [
        Key::ArrowLeft,
        Key::ArrowRight,
        Key::ArrowUp,
        Key::ArrowDown,
    ] {
        let mut rig = mock();
        rig.focus(id);
        let before = rig.snap();
        let c =
            check_one_set_field_on_z0_or_basis(&mut rig, &format!("{key:?} on the figure"), |r| {
                r.key(key, NONE)
            });
        assert!(c.z0 && !c.basis, "{key:?} pans");
        let d = add(&rig.snap().sim.plane.z0, -1.0, &before.sim.plane.z0);
        assert!(
            out_of_plane(&before.sim.plane, &d) < 1e-12,
            "{key:?} left the plane"
        );
    }
    for (key, zoom_in) in [(Key::Plus, true), (Key::Minus, false)] {
        let mut rig = mock();
        rig.focus(id);
        let before = rig.snap();
        check_one_set_field_on_z0_or_basis(&mut rig, &format!("{key:?} on the figure"), |r| {
            r.key(key, NONE)
        });
        let k = check_zoom(&before, &rig.snap());
        assert_eq!(k < 1.0, zoom_in, "{key:?}: scale ×{k}");
    }
    // Control: + with the figure unfocused zooms nothing.
    let mut rig = mock();
    let n0 = rig.applied();
    rig.key(Key::Plus, NONE);
    rejects("+ with no focus", || assert_eq!(rig.applied(), n0 + 1));
}

// --- REQ-GUI-171: the stand-in redrawn for the new chart, within one pixel --------------------------------------------

struct GpuRig {
    canvas: Arc<MockCanvas>,
    rig: Rig<MockSide>,
}

fn gpu() -> GpuRig {
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    let rig = Rig::new(App::new(
        MockSide::new(MockEngine::frozen(), Some(canvas.clone())),
        FORMAT,
    ));
    GpuRig { canvas, rig }
}

impl GpuRig {
    fn shot(&mut self) -> Shot {
        render_frame(&self.canvas, &mut self.rig.app, &mut self.rig.h)
    }
}

fn px(shot: &Shot, x: i64, y: i64) -> [u8; 4] {
    let i = (y as usize * shot.size[0] as usize + x as usize) * 4;
    shot.rgba[i..i + 4].try_into().unwrap()
}

/// Every pixel of `after`'s figure on a grid, whose chart point `before` shows, matches a pixel of `before` within one
/// pixel of where `before` shows it. The chart point of a pixel centre is found from each snapshot's chart, so the
/// expected shift or scale is the chart's, not the gesture's. Returns the pixels checked and those matching nothing.
fn within_one_pixel(
    before: &Shot,
    b: &Plane,
    after: &Shot,
    a: &Plane,
    figure: Rect,
) -> (usize, Vec<(i64, i64)>) {
    let p = f64::from(PIXELS_PER_POINT);
    let [x0, y0, x1, y1] =
        [figure.min.x, figure.min.y, figure.max.x, figure.max.y].map(|v| f64::from(v) * p);
    let (w, h) = (x1 - x0, y1 - y0);
    let (mut checked, mut strays) = (0, Vec::new());
    let mut y = y0.ceil() as i64 + 2;
    while (y as f64) < y1 - 2.0 {
        let mut x = x0.ceil() as i64 + 2;
        while (x as f64) < x1 - 2.0 {
            let s = (x as f64 + 0.5 - x0) / w;
            let t = (y1 - (y as f64 + 0.5)) / h;
            let z = point(a, s, t);
            // (s, t) on `before`'s chart of the same point: z = z₀ + (2s−1) q₁ + (2t−1) q₂.
            let r = add(&z, -1.0, &b.z0);
            let (u, v) = (
                dot(&r, &b.q1) / dot(&b.q1, &b.q1),
                dot(&r, &b.q2) / dot(&b.q2, &b.q2),
            );
            let (sb, tb) = ((u + 1.0) / 2.0, (v + 1.0) / 2.0);
            let (bx, by) = (x0 + sb * w - 0.5, y1 - tb * h - 0.5);
            // Only a source on a pixel centre has a pixel to match; one between pixels has none of its colour.
            let on_grid = (bx - bx.round()).abs() < 0.01 && (by - by.round()).abs() < 0.01;
            if on_grid && bx >= x0 + 2.0 && bx < x1 - 3.0 && by >= y0 + 2.0 && by < y1 - 3.0 {
                checked += 1;
                let want = px(after, x, y);
                let (cx, cy) = (bx.round() as i64, by.round() as i64);
                let found = (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| {
                        let got = px(before, cx + dx, cy + dy);
                        got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 1)
                    })
                });
                if !found {
                    strays.push((x, y));
                }
            }
            x += 1;
        }
        y += 1;
    }
    (checked, strays)
}

fn check_redrawn(before: &Shot, b: &Plane, after: &Shot, a: &Plane, figure: Rect, what: &str) {
    let (checked, strays) = within_one_pixel(before, b, after, a, figure);
    assert!(checked > 10_000, "{what}: only {checked} pixels checked");
    assert!(
        strays.is_empty(),
        "{what}: {} of {checked} pixels match nothing within one pixel of their source, e.g. {:?}",
        strays.len(),
        &strays[..strays.len().min(8)]
    );
}

#[test]
fn qa_mock_figure_navigation_the_stand_in_is_redrawn_shifted_and_scaled() {
    let figure = layout().figure;
    let mut g = gpu();
    let s0 = g.shot();
    let p0 = g.rig.snap().sim.plane;
    // A pan up and left by whole pixels: 40 × 20 points.
    let from = figure.center();
    g.rig.drag(from, from + egui::vec2(-40.0, -20.0), 1);
    let s1 = g.shot();
    let p1 = g.rig.snap().sim.plane;
    assert_ne!(p0, p1, "the pan edited nothing");
    check_redrawn(&s0, &p0, &s1, &p1, figure, "after a pan");
    // A zoom in about the centre by three: a third of the pixels then have their source on a pixel centre.
    let lines = 3f64.log2() / gui::explore::manifold_view::navigate::OCTAVES_PER_LINE;
    g.rig.wheel(figure.center(), lines as f32);
    let s2 = g.shot();
    let p2 = g.rig.snap().sim.plane;
    assert_ne!(p1, p2, "the zoom edited nothing");
    check_redrawn(&s1, &p1, &s2, &p2, figure, "after a zoom");
    // Controls: the frame before the pan is not the frame after it; the zoom read as no zoom.
    rejects("the picture unshifted", || {
        check_redrawn(&s0, &p0, &s1, &p0, figure, "unshifted")
    });
    rejects("the picture unscaled", || {
        check_redrawn(&s1, &p1, &s2, &p1, figure, "unscaled")
    });
}

// --- REQ-GUI-171: the axis labels follow a pan --------------------------------------------------------------------

/// The axis labels' texts in the strip under the figure: the low end at the left, the high end at the right.
fn horizontal_ends(texts: &[(String, Rect)], lay: &Layout) -> (String, String, Vec<String>) {
    let strip = Rect::from_min_max(
        egui::pos2(lay.figure.min.x - 2.0, lay.figure.max.y - 1.0),
        egui::pos2(lay.figure.max.x + 2.0, lay.compass.min.y + 1.0),
    );
    let mut row: Vec<&(String, Rect)> = texts
        .iter()
        .filter(|(_, r)| strip.contains_rect(*r))
        .collect();
    row.sort_by(|a, b| a.1.min.x.total_cmp(&b.1.min.x));
    let all = row.iter().map(|(t, _)| t.clone()).collect();
    let numbers: Vec<&String> = row
        .iter()
        .map(|(t, _)| t)
        .filter(|t| number(t).is_some())
        .collect();
    assert!(
        numbers.len() >= 2,
        "no two end values under the figure: {all:?}"
    );
    (numbers[0].clone(), numbers[numbers.len() - 1].clone(), all)
}

fn check_ends(texts: &[(String, Rect)], lay: &Layout, plane: &Plane) {
    let (low, high, all) = horizontal_ends(texts, lay);
    assert!(
        all.iter().any(|t| t.contains("z_α")),
        "the axis's short name z_α is not under the figure: {all:?}"
    );
    // The chart "z_α × z_β": z_α runs from z(0, ½) to z(1, ½) across the figure.
    let (lo, hi) = (point(plane, 0.0, 0.5)[0], point(plane, 1.0, 0.5)[0]);
    for (text, want) in [(&low, lo), (&high, hi)] {
        let got = number(text).unwrap();
        assert!(
            (got - want).abs() <= 0.5 * 10f64.powi(-decimals(text)) + 1e-12,
            "the end `{text}` is not {want}"
        );
    }
}

#[test]
fn qa_mock_axis_labels_ends_follow_a_pan() {
    let lay = layout();
    let mut rig = mock();
    let p0 = rig.snap().sim.plane;
    let t0 = rig.texts();
    check_ends(&t0, &lay, &p0);
    rig.drag(
        lay.figure.center(),
        lay.figure.center() + egui::vec2(-120.0, 0.0),
        1,
    );
    let p1 = rig.snap().sim.plane;
    assert_ne!(p0.z0[0], p1.z0[0], "the pan did not move z_α");
    let t1 = rig.texts();
    check_ends(&t1, &lay, &p1);
    rejects("the ends left as they were before the pan", || {
        check_ends(&t1, &lay, &p0)
    });
}

// --- REQ-GUI-081: one group, its sections in order, eight named sliders ---------------------------------------------

/// The top of the first text in `panel` reading `want` exactly (or containing it, with `contains`).
fn y_of(texts: &[(String, Rect)], panel: Rect, want: &str, contains: bool) -> f32 {
    texts
        .iter()
        .filter(|(t, r)| {
            panel.contains_rect(*r)
                && if contains {
                    t.contains(want)
                } else {
                    t == want
                }
        })
        .map(|(_, r)| r.min.y)
        .fold(f32::INFINITY, f32::min)
}

fn check_order(texts: &[(String, Rect)], panel: Rect) {
    let ys = [
        y_of(texts, panel, "Manifold view", false),
        y_of(texts, panel, "Chart", false),
        y_of(texts, panel, "Navigate", false),
        y_of(texts, panel, "depth", true),
        y_of(texts, panel, "locked", true),
        y_of(texts, panel, "Centre z₀", false),
        y_of(texts, panel, "Slice & tilt", false),
    ];
    assert!(
        ys.iter().all(|y| y.is_finite()) && ys.windows(2).all(|w| w[0] < w[1]),
        "the left panel's parts are not Manifold view, Chart, Navigate, depth, Lock, Centre z₀, Slice & tilt, top \
         to bottom: {ys:?}"
    );
}

#[test]
fn qa_mock_manifold_view_one_group_sections_in_order() {
    let lay = layout();
    let mut rig = mock();
    let texts = rig.texts();
    let panel = lay.manifold_view;
    check_order(&texts, panel);
    // Chart: the preset named by its axes, never a nickname; Chart builder…; the chart's kind.
    for want in ["z_α × z_β", "Chart builder", "affine"] {
        assert!(
            texts
                .iter()
                .any(|(t, r)| panel.contains_rect(*r) && t.contains(want)),
            "no `{want}` in the Manifold view"
        );
    }
    // Centre z₀: the eight named sliders, in order, top to bottom.
    let rows = centre_rows(&texts, panel);
    assert!(
        rows.windows(2).all(|w| w[0].1.min.y < w[1].1.min.y),
        "the sliders out of order"
    );
    // Slice & tilt: the slice step (its row's "step", under the section's title), τ₁, τ₂ and γ.
    let slice_top = y_of(&texts, panel, "Slice & tilt", false);
    for want in ["step", "τ₁", "τ₂", "γ"] {
        assert!(
            texts
                .iter()
                .any(|(t, r)| panel.contains_rect(*r) && r.min.y > slice_top && t.contains(want)),
            "no `{want}` in Slice & tilt"
        );
    }
    // §G3's sub-scopes, in order.
    let tree = rig.app.keyboard.tree(Mode::Explore);
    let subs: Vec<&str> = tree
        .children(Some(manifold_view_id(&rig)))
        .into_iter()
        .map(|id| tree.get(id).unwrap().label)
        .collect();
    assert_eq!(subs, SECTIONS);
    // Control: Centre z₀ taken for Slice & tilt.
    let swapped: Vec<(String, Rect)> = texts
        .iter()
        .map(|(t, r)| {
            let t = match t.as_str() {
                "Centre z₀" => "Slice & tilt".to_owned(),
                "Slice & tilt" => "Centre z₀".to_owned(),
                _ => t.clone(),
            };
            (t, *r)
        })
        .collect();
    rejects("the sections swapped", || check_order(&swapped, panel));
}

// --- R-54: the precision warning is raised by events, not by depth --------------------------------------------------

fn real_app_at_depth(octaves: f64) -> Rig<RealSide> {
    let k = (-octaves).exp2();
    let mut q1 = [0.0; 8];
    let mut q2 = [0.0; 8];
    q1[0] = k;
    q2[1] = k;
    let sim = SimConfig {
        chart: Chart {},
        plane: Plane {
            z0: [0.1; 8],
            q1,
            q2,
        },
        slice: Slice {},
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links {},
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon {},
        collision: Collision {},
        quality: Quality {},
    };
    let render = RenderState {
        stain_graph: StainGraph {},
        overlays: Overlays {},
        palette: Palette {},
        playhead: Playhead { t: 0.0 },
    };
    Rig::new(App::new(RealSide::new(sim, render), FORMAT))
}

fn warns(texts: &[(String, Rect)], panel: Rect) -> bool {
    texts
        .iter()
        .any(|(t, r)| panel.contains_rect(*r) && t.contains("linearise decode"))
}

#[test]
fn qa_mock_manifold_view_precision_warning_by_events_not_depth() {
    let panel = layout().manifold_view;
    // Forty octaves deep, past any fixed depth, on an engine that raises no event: no warning.
    let mut real = real_app_at_depth(40.0);
    let s = real.snap();
    assert!(!s.precision.decode_switchover && !s.precision.at_f32_floor);
    let real_texts = real.texts();
    assert!(
        !warns(&real_texts, panel),
        "a warning with no event raised, from the depth alone"
    );
    // The mock at the same depth raises DECODE_SWITCHOVER: the warning shows.
    let mut rig = mock();
    let z0 = rig.snap().sim.plane.z0;
    let k = (-40.0f64).exp2();
    rig.app.set_field(SetField {
        edit: Edit::Sim(SimField::Basis {
            q1: std::array::from_fn(|i| if i == 0 { k } else { 0.0 }),
            q2: std::array::from_fn(|i| if i == 1 { k } else { 0.0 }),
        }),
        no_history: false,
    });
    rig.idle(2);
    assert_eq!(rig.snap().sim.plane.z0, z0);
    assert!(
        rig.snap().precision.decode_switchover,
        "the mock raised no event at depth"
    );
    let texts = rig.texts();
    assert!(warns(&texts, panel), "the event raised no warning");
    // And none at the mock's start, with no event.
    let mut fresh = mock();
    assert!(!fresh.snap().precision.decode_switchover);
    assert!(!warns(&fresh.texts(), panel));
    rejects("the real engine's texts read for the event", || {
        assert!(warns(&real_texts, panel))
    });
}

// --- REQ-GUI-091: the compass ---------------------------------------------------------------------------------------

fn scope_by_label(rig: &Rig<MockSide>, label: &str) -> &'static str {
    let tree = rig.app.keyboard.tree(Mode::Explore);
    let mut all: Vec<&'static str> = tree.children(None);
    let mut i = 0;
    while i < all.len() {
        let kids = tree.children(Some(all[i]));
        all.extend(kids);
        i += 1;
    }
    all.into_iter()
        .find(|id| tree.get(id).unwrap().label == label)
        .unwrap_or_else(|| panic!("no scope `{label}`"))
}

/// The compass's mode, as it draws it: the selected one of "slicing" and "tilting".
fn compass_mode(texts: &[(String, Rect)], compass: Rect) -> Vec<String> {
    texts
        .iter()
        .filter(|(t, r)| compass.contains_rect(*r) && t.starts_with('●'))
        .map(|(t, _)| t.clone())
        .filter(|t| t.contains("slicing") || t.contains("tilting"))
        .collect()
}

fn check_mode(texts: &[(String, Rect)], compass: Rect, want: &str) {
    let shown = compass_mode(texts, compass);
    assert!(
        shown.len() == 1 && shown[0].contains(want),
        "the compass shows {shown:?}, not {want}"
    );
}

#[test]
fn qa_mock_compass_switches_mode_by_the_slider_touched() {
    let compass = layout().compass;
    let mut rig = mock();
    let tau1 = scope_by_label(&rig, "τ₁");
    let tau2 = scope_by_label(&rig, "τ₂");
    let step = scope_by_label(&rig, "slice step");
    let mv = manifold_view_id(&rig);
    let tree = rig.app.keyboard.tree(Mode::Explore);
    let centre = tree
        .children(Some(mv))
        .into_iter()
        .find(|id| tree.get(id).unwrap().label == "Centre z₀")
        .unwrap();
    let slider = tree.children(Some(centre))[3];
    let mut seen = Vec::new();
    for (id, want) in [
        (tau1, "tilting"),
        (slider, "slicing"),
        (tau2, "tilting"),
        (step, "slicing"),
    ] {
        rig.focus(id);
        rig.key(Key::ArrowUp, NONE);
        let texts = rig.texts();
        check_mode(&texts, compass, want);
        seen.push(texts);
    }
    rejects("tilting read after a slice slider", || {
        check_mode(&seen[1], compass, "tilting")
    });
}

/// The angle after `name` in the compass's readout, in degrees.
fn readout(texts: &[(String, Rect)], compass: Rect, name: &str) -> f64 {
    let line = texts
        .iter()
        .find(|(t, r)| compass.contains_rect(*r) && t.contains(name) && t.contains('°'))
        .unwrap_or_else(|| panic!("no {name} readout in the compass"))
        .0
        .clone();
    let after = &line[line.find(name).unwrap() + name.len()..];
    let digits: String = after
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '−')
        .collect();
    number(&digits).unwrap_or_else(|| panic!("no angle after {name} in {line:?}"))
}

/// The angle of `v` out of span(e_α, e_β), in degrees: the tilt of a basis vector of the chart "z_α × z_β".
fn tilt_of(v: &Z) -> f64 {
    let inside = v[0].hypot(v[1]);
    let outside = v[2..].iter().map(|x| x * x).sum::<f64>().sqrt();
    outside.atan2(inside).to_degrees()
}

#[test]
fn qa_mock_compass_reads_out_the_tilt_and_rotation_angles() {
    let compass = layout().compass;
    // τ₁ turned by ten base steps, Shift: the compass reads q₁'s tilt out of the chart's plane.
    let mut rig = mock();
    let tau1 = scope_by_label(&rig, "τ₁");
    rig.focus(tau1);
    rig.key(Key::ArrowUp, SHIFT);
    let q1 = rig.snap().sim.plane.q1;
    let want = tilt_of(&q1);
    assert!(want > 1.0, "τ₁ did not tilt q₁: {q1:?}");
    let texts = rig.texts();
    let got = readout(&texts, compass, "τ₁");
    assert!(
        (got.abs() - want).abs() < 0.06,
        "the compass reads τ₁ {got}°, q₁ is tilted {want}°"
    );
    // γ turned: the compass reads q₁'s angle in the plane.
    let mut rig = mock();
    let gamma = scope_by_label(&rig, "γ");
    rig.focus(gamma);
    rig.key(Key::ArrowUp, SHIFT);
    rig.key(Key::ArrowUp, SHIFT);
    let q1 = rig.snap().sim.plane.q1;
    let want = q1[1].atan2(q1[0]).to_degrees().abs();
    assert!(want > 1.0, "γ did not turn q₁: {q1:?}");
    let got = readout(&rig.texts(), compass, "γ");
    assert!(
        (got.abs() - want).abs() < 0.06,
        "the compass reads γ {got}°, q₁ is turned {want}°"
    );
    rejects("the τ₁ readout taken for γ's angle", || {
        assert!((readout(&texts, compass, "τ₁").abs() - want).abs() < 0.06)
    });
}

#[test]
fn qa_mock_compass_arrows_tilt_and_shift_arrows_orbit() {
    let rig = mock();
    let compass = scope_by_label(&rig, "Compass");
    // ← → ↑ ↓ tilt: one SetField on the basis, and the compass shows tilting.
    for key in [
        Key::ArrowLeft,
        Key::ArrowRight,
        Key::ArrowUp,
        Key::ArrowDown,
    ] {
        let mut rig = mock();
        rig.focus(compass);
        let c =
            check_one_set_field_on_z0_or_basis(&mut rig, &format!("{key:?} on the compass"), |r| {
                r.key(key, NONE)
            });
        assert!(c.basis && !c.z0, "{key:?} tilts the plane");
        check_mode(&rig.texts(), layout().compass, "tilting");
    }
    // Shift+arrows orbit the cube: the GUI's own view of it, no edit.
    let mut rig = mock();
    rig.focus(compass);
    let (before, n0) = (rig.snap(), rig.applied());
    let drawn_before = rig.shapes();
    rig.key(Key::ArrowRight, SHIFT);
    assert_eq!(rig.applied(), n0, "an orbit applied a SetField");
    assert_eq!(rig.snap().sim, before.sim, "an orbit edited the chart");
    let compass_rect = layout().compass;
    let in_compass = |shapes: &[egui::Shape]| -> Vec<egui::Shape> {
        shapes
            .iter()
            .filter(|s| compass_rect.contains_rect(s.visual_bounding_rect()))
            .cloned()
            .collect()
    };
    let drawn_after = rig.shapes();
    assert_ne!(
        in_compass(&drawn_before),
        in_compass(&drawn_after),
        "Shift+→ did not turn the cube"
    );
    rejects("a tilt taken for an orbit", || {
        let mut rig = mock();
        rig.focus(compass);
        let n0 = rig.applied();
        rig.key(Key::ArrowRight, NONE);
        assert_eq!(rig.applied(), n0);
    });
}

// --- REQ-GUI-169, for this screen ------------------------------------------------------------------------------------

/// Walks `section`'s controls from its first: Enter into the section, then ↓ to each next sibling. §G3 gives the
/// arrows both jobs, between siblings and on a focused value; the PR's decision (applied per R-369): a value reached by
/// Enter adjusts, Enter on it turns its arrows to its siblings, and an arrow lands on the next in that state. Returns
/// the labels landed on.
fn walk(rig: &mut Rig<MockSide>, section: &str) -> Vec<String> {
    let id = scope_by_label(rig, section);
    rig.focus(id);
    rig.key(Key::Enter, NONE);
    let tree = rig.app.keyboard.tree(Mode::Explore);
    let n = tree.children(Some(id)).len();
    let mut seen = vec![rig.labels().last().unwrap().clone()];
    let first = rig.app.view.focus.path.last().unwrap().clone();
    if matches!(
        rig.app
            .keyboard
            .tree(Mode::Explore)
            .get(&first)
            .unwrap()
            .arrows,
        Arrows::Adjust(_)
    ) {
        rig.key(Key::Enter, NONE);
    }
    for _ in 1..n {
        let before = rig.snap();
        rig.key(Key::ArrowDown, NONE);
        assert_eq!(
            rig.snap().sim,
            before.sim,
            "walking {section} edited the chart"
        );
        seen.push(rig.labels().last().unwrap().clone());
    }
    seen
}

#[test]
fn qa_mock_keyboard_enter_reaches_every_control_of_each_section() {
    for section in SECTIONS {
        let mut rig = mock();
        let seen = walk(&mut rig, section);
        let tree = rig.app.keyboard.tree(Mode::Explore);
        let id = scope_by_label(&rig, section);
        let want: Vec<String> = tree
            .children(Some(id))
            .into_iter()
            .map(|c| tree.get(c).unwrap().label.to_owned())
            .collect();
        assert_eq!(
            seen, want,
            "the keys did not reach every control of {section}"
        );
        // Esc backs out one level, to the section.
        rig.key(Key::Escape, NONE);
        assert_eq!(
            rig.labels(),
            ["Manifold view", section],
            "Esc from a control of {section}"
        );
    }
    // Centre z₀'s eight are §G2's eight names.
    let mut rig = mock();
    let seen = walk(&mut rig, "Centre z₀");
    assert_eq!(seen, AXES);
    rejects("the sliders out of order", || {
        let mut swapped = AXES;
        swapped.swap(0, 1);
        assert_eq!(seen, swapped)
    });
}

#[test]
fn qa_mock_keyboard_shift_and_alt_steps_on_this_screen() {
    // A z₀ slider, the slice step, τ₁, τ₂ and γ: Shift is ten steps, Alt a tenth.
    for label in ["z_q1", "slice step", "τ₁", "τ₂", "γ"] {
        let step = |modifiers: Modifiers| -> f64 {
            let mut rig = mock();
            let id = if label == "z_q1" {
                let tree = rig.app.keyboard.tree(Mode::Explore);
                let mv = manifold_view_id(&rig);
                let centre = tree
                    .children(Some(mv))
                    .into_iter()
                    .find(|id| tree.get(id).unwrap().label == "Centre z₀")
                    .unwrap();
                tree.children(Some(centre))[3]
            } else {
                scope_by_label(&rig, label)
            };
            rig.focus(id);
            let before = rig.snap().sim.plane;
            rig.key(Key::ArrowUp, modifiers);
            let after = rig.snap().sim.plane;
            if label == "z_q1" || label == "slice step" {
                norm(&add(&after.z0, -1.0, &before.z0))
            } else if label == "γ" {
                let a = |q: &Z| q[1].atan2(q[0]).to_degrees();
                (a(&after.q1) - a(&before.q1)).abs()
            } else {
                let q = |p: &Plane| if label == "τ₁" { p.q1 } else { p.q2 };
                (tilt_of(&q(&after)) - tilt_of(&q(&before))).abs()
            }
        };
        let (one, ten, tenth) = (step(NONE), step(SHIFT), step(ALT));
        assert!(one > 0.0, "{label}: ↑ moved nothing");
        assert!(
            (ten / one - 10.0).abs() < 1e-6 && (tenth / one - 0.1).abs() < 1e-6,
            "{label}: ↑ {one}, Shift+↑ {ten}, Alt+↑ {tenth}"
        );
    }
}

/// The breadcrumb drawn in the top bar reads `want`.
fn check_breadcrumb(texts: &[(String, Rect)], top_bar: Rect, want: &str) {
    assert!(
        texts
            .iter()
            .any(|(t, r)| top_bar.contains_rect(*r) && t.contains(want)),
        "no breadcrumb `{want}` in the top bar"
    );
}

#[test]
fn qa_mock_keyboard_breadcrumb_names_manifold_view_navigate_zoom() {
    let top_bar = layout().top_bar;
    let mut rig = mock();
    let zoom = scope_by_label(&rig, "zoom");
    rig.focus(zoom);
    let texts = rig.texts();
    // §G3's own example.
    check_breadcrumb(&texts, top_bar, "Manifold view › Navigate › zoom");
    rejects("another path", || {
        check_breadcrumb(&texts, top_bar, "Manifold view › Chart › zoom")
    });
}
