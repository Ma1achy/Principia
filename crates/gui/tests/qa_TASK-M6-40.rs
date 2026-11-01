//! QA tests for TASK-M6-40, written from the requirements and R-459, not from the implementation:
//!
//! - REQ-GUI-208: the Cube view's three edges carry buttons in both modes, H (bottom-left, Chart's horizontal q₁), V
//!   (left vertical, Chart's vertical q₂) and the third axis (bottom-right, in Slice mode D, the slice normal), each
//!   reading its letter and dimension, each a dropdown of the dimensions not on the other two; picking H or V sets the
//!   new axis pair with τ₁ = τ₂ = γ = 0, zoom and z₀ kept, locked about the pin; an edit in Chart moves H and V; picking D
//!   sets the normal, a target on it following and re-tilting; each pick is one entry; a plane that takes D moves it by
//!   the slice-axis rule in the same entry; the keys: siblings after the view buttons, Enter opening.
//! - REQ-GUI-209: in Tilt mode the third axis shows X while τ₁ is dragged and Y while τ₂ is (a compass drag moving both,
//!   the angle moved further since it began, τ₁ on a tie), at rest X if τ₁ ≠ 0, else Y if τ₂ ≠ 0, else the normal; in
//!   Slice mode D; the label takes the active gesture's colour; the cube draws that gesture's plane, View::basis with the
//!   other angle at 0 in (e_a, e_b, e_T); it is that target's dropdown; the readout names both targets with their angles;
//!   with both set and X ≠ Y the "4D: see Plücker" badge, in the cube's top-right corner clear of the drawing (G8), a
//!   click or Enter switching to the Plücker view.
//!
//! The drawing is checked on what is painted, against this file's own projection (R-451's `P(x,y,z)`) and own
//! cross-section of the plane built from R-457 E1's map. The gesture colours are look choices (R-459 G7), so only their
//! distinctness and agreement are asserted. gui takes no dependency on validation (R-187), so each check runs its
//! control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-40` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::accesskit::Role;
use eframe::egui::{self, Color32, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
use engine::contract::log::Source;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Lock, SliceFrame};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::{KeyMode, Mode};
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::compass::physical::{Decoder, Matrix};
use gui::explore::compass::views::{CompassView, GridPlace};
use gui::explore::compass::{self};
use gui::explore::manifold_view::{Touch, View, AXES, TAU1, TAU2, Z0_HALF_RANGE};
use gui::explore::{COMPASS, COMPASS_SLICE, COMPASS_TILT, COMPASS_VIEW};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::MockSide;

type Z = [f64; 8];
type P3 = [f64; 3];

const NONE: Modifiers = Modifiers::NONE;
/// A key no command binds: pressed only to turn keyboard navigation on (R-460, R-464 M2).
const UNBOUND: Key = Key::F12;
const ALPHA: usize = 0;
const BETA: usize = 1;
const Q0: usize = 2;
const Q1: usize = 3;
const Q2: usize = 4;
const Q3: usize = 5;
const MU1: usize = 6;
const MU2: usize = 7;
const BADGE: &str = "4D: see Plücker";

const WIDTH_EDGE: (P3, P3) = ([-1.0, -1.0, 1.0], [1.0, -1.0, 1.0]);
const DEPTH_EDGE: (P3, P3) = ([1.0, -1.0, -1.0], [1.0, -1.0, 1.0]);
const HEIGHT_EDGE: (P3, P3) = ([-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]);

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn e(i: usize, k: f64) -> Z {
    std::array::from_fn(|j| if j == i { k } else { 0.0 })
}

fn add(a: &Z, k: f64, b: &Z) -> Z {
    std::array::from_fn(|i| a[i] + k * b[i])
}

fn scaled(a: &Z, k: f64) -> Z {
    a.map(|x| x * k)
}

fn close(a: &Z, b: &Z, tol: f64) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
}

fn frame(normal: usize, x: usize, y: usize) -> SliceFrame {
    SliceFrame {
        normal: normal as u32,
        tilt_x: x as u32,
        tilt_y: y as u32,
    }
}

/// R-457 E1's map: `u₁ = cos τ₁ e_a + sin τ₁ e_X`, `u₂ = cos τ₂ e_b + sin τ₂ e_Y`, turned by γ, scale σ. Degrees.
#[allow(clippy::too_many_arguments)] // the map's own eight arguments
fn map(a: usize, b: usize, x: usize, y: usize, t1: f64, t2: f64, g: f64, s: f64) -> (Z, Z) {
    let (t1, t2, g) = (t1.to_radians(), t2.to_radians(), g.to_radians());
    let u1 = add(&e(a, t1.cos()), t1.sin(), &e(x, 1.0));
    let u2 = add(&e(b, t2.cos()), t2.sin(), &e(y, 1.0));
    (
        scaled(&add(&scaled(&u1, g.cos()), g.sin(), &u2), s),
        scaled(&add(&scaled(&u2, g.cos()), -g.sin(), &u1), s),
    )
}

// --- this file's own drawing ------------------------------------------------------------------------------------------

/// R-451's projection, fitted to the compass's square.
#[derive(Clone, Copy)]
struct Iso {
    cx: f64,
    cy: f64,
    s: f64,
}

fn iso() -> Iso {
    let sq = compass::square(layout().compass);
    let (w, h) = (f64::from(sq.width()), f64::from(sq.height()));
    Iso {
        cx: f64::from(sq.center().x),
        cy: f64::from(sq.center().y),
        s: (0.78 * w / (2.0 * 3f64.sqrt())).min(0.78 * h / 4.0),
    }
}

impl Iso {
    fn at(&self, [x, y, z]: P3) -> [f64; 2] {
        [
            self.cx + self.s * 3f64.sqrt() / 2.0 * (x - z),
            self.cy + self.s * (0.5 * (x + z) - y),
        ]
    }
}

/// Where an edge's label goes: centred on it, pushed 20 points outward from the cube's centre (R-451 B1).
fn label_at(iso: &Iso, edge: (P3, P3)) -> Pos2 {
    let (a, b) = (iso.at(edge.0), iso.at(edge.1));
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = dx.hypot(dy);
    let m = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let (mut px, mut py) = (-dy / len, dx / len);
    if (m[0] - iso.cx) * px + (m[1] - iso.cy) * py < 0.0 {
        (px, py) = (-px, -py);
    }
    egui::pos2((m[0] + 20.0 * px) as f32, (m[1] + 20.0 * py) as f32)
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The plane of `(q₁, q₂)` through `p`, cut with `[−1, 1]³`: the crossings of the twelve edges, as a set.
fn cross_section(q1: P3, q2: P3, p: P3) -> Vec<P3> {
    let n = cross(q1, q2);
    let off = dot3(n, p);
    let mut out: Vec<P3> = Vec::new();
    let corner =
        |m: u32| -> P3 { std::array::from_fn(|k| if m >> k & 1 == 1 { 1.0 } else { -1.0 }) };
    for m in 0..8u32 {
        for k in 0..3 {
            if m >> k & 1 == 1 {
                continue;
            }
            let (a, b) = (corner(m), corner(m | 1 << k));
            let (da, db) = (dot3(n, a) - off, dot3(n, b) - off);
            if da * db > 1e-18 || (da == 0.0 && db == 0.0) {
                continue;
            }
            let t = da / (da - db);
            let at: P3 = std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t);
            if !out
                .iter()
                .any(|q| (0..3).all(|i| (q[i] - at[i]).abs() < 1e-9))
            {
                out.push(at);
            }
        }
    }
    out
}

/// The painted polygon has the vertices of `want` (in the cube's frame), as a set, within 0.05 points.
fn check_polygon(fill: &[Pos2], want: &[P3], iso: &Iso) {
    assert_eq!(fill.len(), want.len(), "the fill {fill:?}, want {want:?}");
    for p in want {
        let [x, y] = iso.at(*p);
        assert!(
            fill.iter()
                .any(|f| (f64::from(f.x) - x).hypot(f64::from(f.y) - y) < 0.05),
            "no vertex of the fill {fill:?} at {p:?} ({x}, {y})"
        );
    }
}

// --- the rig ----------------------------------------------------------------------------------------------------------

fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

fn button(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    }
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

/// One painted text: its string, its rect, and its sections' `(text, colour)`.
#[derive(Clone, Debug)]
struct Painted {
    text: String,
    rect: Rect,
    sections: Vec<(String, Color32)>,
}

fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
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

    fn settle(&mut self) {
        for _ in 0..400 {
            self.idle(1);
            if !self.app.manifold.compass.moving {
                self.idle(1);
                return;
            }
        }
        panic!("the compass did not come to rest");
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

    /// The SetFields the engine has applied so far, from its `contract` log, less the hover's.
    fn applied(&self) -> usize {
        self.app
            .console()
            .iter()
            .filter(|e| {
                e.source == Source::Contract && !e.message.starts_with("SetField Hover.at ")
            })
            .count()
    }

    fn set(&mut self, edit: SimField) {
        self.app.set_field(SetField {
            edit: Edit::Sim(edit),
            no_history: false,
        });
        self.idle(3);
    }

    /// The chart `(a, b)` at `(τ₁, τ₂, γ)`, scale σ, in the slice frame `f`, straight on the contract.
    fn chart(&mut self, (a, b): (usize, usize), f: SliceFrame, t: (f64, f64, f64), s: f64) {
        self.set(SimField::SliceFrame {
            frame: f,
            basis: None,
        });
        let (q1, q2) = map(a, b, f.tilt_x as usize, f.tilt_y as usize, t.0, t.1, t.2, s);
        self.set(SimField::Basis { q1, q2 });
        self.settle();
    }

    /// The default chart `z_α × z_β` at `(τ₁, τ₂)`, scale 1, in the slice frame `f`.
    fn tilted(&mut self, f: SliceFrame, t1: f64, t2: f64) {
        self.chart((ALPHA, BETA), f, (t1, t2, 0.0), 1.0);
    }

    /// [`Rig::tilted`] with `z₀` at the origin, so the drawn plane goes through the cube's centre.
    fn centred(&mut self, f: SliceFrame, t1: f64, t2: f64) {
        self.set(SimField::Z0([0.0; 8]));
        self.tilted(f, t1, t2);
    }

    /// A click on the compass's mode button, the real path.
    fn mode(&mut self, id: &str) {
        let at = self.app.keyboard.place_of(id).expect("placed").hit.center();
        self.click(at);
        self.settle();
    }

    fn click(&mut self, at: Pos2) {
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        let _ = self.frame(vec![button(at, true)]);
        let _ = self.frame(vec![button(at, false)]);
        self.idle(2);
    }

    /// The frame's shapes, flattened.
    fn shapes(&mut self) -> Vec<egui::Shape> {
        let out = self.frame(Vec::new());
        let mut all = Vec::new();
        for c in &out.shapes {
            flatten(&c.shape, &mut all);
        }
        all
    }

    fn painted(&mut self) -> Vec<Painted> {
        Self::texts_of(&self.shapes())
    }

    fn texts_of(shapes: &[egui::Shape]) -> Vec<Painted> {
        shapes
            .iter()
            .filter_map(|s| match s {
                egui::Shape::Text(t) => {
                    let job = &t.galley.job;
                    Some(Painted {
                        text: t.galley.text().to_owned(),
                        rect: t.galley.rect.translate(t.pos.to_vec2()),
                        sections: job
                            .sections
                            .iter()
                            .map(|sec| {
                                (
                                    job.text[sec.byte_range.start.0..sec.byte_range.end.0]
                                        .to_owned(),
                                    t.override_text_color.unwrap_or(sec.format.color),
                                )
                            })
                            .collect(),
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// The text painted at the place of `edge`'s label, within a point.
    fn on_edge(&mut self, edge: (P3, P3)) -> Painted {
        let at = label_at(&iso(), edge);
        let texts = self.painted();
        texts
            .iter()
            .find(|t| (t.rect.center() - at).length() < 1.0)
            .unwrap_or_else(|| panic!("no label painted at {at:?}: {texts:?}"))
            .clone()
    }

    fn third(&mut self) -> String {
        self.on_edge(DEPTH_EDGE).text
    }

    /// The painted polygon in the compass.
    fn fill(&mut self) -> Vec<Pos2> {
        let rect = layout().compass;
        self.shapes()
            .iter()
            .find_map(|s| match s {
                egui::Shape::Path(p)
                    if p.closed
                        && p.fill != Color32::TRANSPARENT
                        && p.points.iter().all(|q| rect.contains(*q)) =>
                {
                    Some(p.points.clone())
                }
                _ => None,
            })
            .expect("no polygon painted in the compass")
    }

    /// The readout's whole text: the compass's texts together.
    fn compass_text(&mut self) -> String {
        let rect = layout().compass;
        self.painted()
            .iter()
            .filter(|t| rect.contains(t.rect.center()))
            .map(|t| t.text.clone())
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn tree_labels(&self, id: &str) -> Vec<&'static str> {
        let tree = self.app.keyboard.tree(Mode::Explore);
        tree.children(Some(id))
            .into_iter()
            .map(|c| tree.get(c).unwrap().label)
            .collect()
    }

    /// The child of the Compass scope labelled `label`.
    fn sibling(&self, label: &str) -> &'static str {
        let tree = self.app.keyboard.tree(Mode::Explore);
        tree.children(Some(COMPASS))
            .into_iter()
            .find(|c| tree.get(c).unwrap().label == label)
            .unwrap_or_else(|| panic!("no `{label}` in the Compass scope"))
    }

    fn scope(&self, parent: &str, label: &str) -> &'static str {
        let tree = self.app.keyboard.tree(Mode::Explore);
        let mut all: Vec<&'static str> = tree.children(None);
        let mut i = 0;
        while i < all.len() {
            let kids = tree.children(Some(all[i]));
            all.extend(kids);
            i += 1;
        }
        let root = *all
            .iter()
            .find(|id| tree.get(id).unwrap().label == parent)
            .unwrap_or_else(|| panic!("no scope {parent}"));
        let mut all: Vec<&'static str> = tree.children(Some(root));
        let mut i = 0;
        while i < all.len() {
            let kids = tree.children(Some(all[i]));
            all.extend(kids);
            i += 1;
        }
        all.into_iter()
            .find(|id| tree.get(id).unwrap().label == label)
            .unwrap_or_else(|| panic!("no scope {label} under {parent}"))
    }

    fn focus(&mut self, id: &str) {
        let path = self.app.keyboard.tree(Mode::Explore).path_to(id);
        assert!(!path.is_empty(), "no scope {id} in the tree");
        self.app.view.focus.path = path;
        self.app.view.focus.mode = KeyMode::Navigation;
        self.key(UNBOUND, NONE);
    }

    fn focus_labels(&self) -> Vec<String> {
        self.app
            .keyboard
            .tree(Mode::Explore)
            .labels(&self.app.view.focus.path)
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// How many of each latent name are painted now, by index.
    fn name_counts(&mut self) -> [usize; 8] {
        let texts = self.painted();
        std::array::from_fn(|i| texts.iter().filter(|t| t.text == AXES[i]).count())
    }

    /// A click on the label on `edge`, opening its dropdown: the dimensions it now paints, each with its place.
    fn open(&mut self, edge: (P3, P3)) -> Vec<(usize, Pos2)> {
        let before = self.painted();
        let at = self.on_edge(edge).rect.center();
        self.click(at);
        let after = self.painted();
        (0..8)
            .filter_map(|i| {
                after
                    .iter()
                    .find(|t| {
                        t.text == AXES[i]
                            && !before.iter().any(|b| {
                                b.text == t.text
                                    && (b.rect.center() - t.rect.center()).length() < 0.5
                            })
                    })
                    .map(|t| (i, t.rect.center()))
            })
            .collect()
    }

    /// Picks `to` on `edge` by the mouse: the SetFields applied, the entries added, before and after.
    fn pick(&mut self, edge: (P3, P3), to: usize) -> (usize, i64, Snapshot, Snapshot) {
        let (before, n0) = (self.snap(), self.applied());
        let offered = self.open(edge);
        let at = offered
            .iter()
            .find(|(i, _)| *i == to)
            .unwrap_or_else(|| panic!("{} not offered: {offered:?}", AXES[to]))
            .1;
        self.click(at);
        self.settle();
        let after = self.snap();
        let entries = i64::from(after.history.undo_depth) - i64::from(before.history.undo_depth);
        (self.applied() - n0, entries, before, after)
    }

    fn undo(&mut self) {
        self.key(Key::Z, Modifiers::CTRL);
        self.settle();
    }

    /// The rail of the slider placed as control `id`.
    fn rail(&mut self, id: &str) -> Rect {
        let out = self.frame(Vec::new());
        let place = self.app.keyboard.place_of(id).expect("placed").rect;
        let update = out
            .platform_output
            .accesskit_update
            .expect("accesskit is on in the headless rig");
        update
            .nodes
            .iter()
            .filter(|(_, n)| n.role() == Role::Slider)
            .filter_map(|(_, n)| {
                let b = n.bounds()?;
                Some(Rect::from_min_max(
                    egui::pos2(b.x0 as f32, b.y0 as f32),
                    egui::pos2(b.x1 as f32, b.y1 as f32),
                ))
            })
            .find(|r| place.contains(r.center()))
            .unwrap_or_else(|| panic!("no slider in {id}"))
    }

    /// Presses on the slider `id`, moves it `by` points over five frames, and returns the third axis read each frame.
    fn slider_drag(&mut self, id: &str, by: f32) -> (Vec<String>, Pos2) {
        let rail = self.rail(id);
        let from = egui::pos2(rail.center().x, rail.center().y);
        let _ = self.frame(vec![Event::PointerMoved(from)]);
        let _ = self.frame(vec![button(from, true)]);
        let mut seen = Vec::new();
        for k in 1..=5 {
            let _ = self.frame(vec![Event::PointerMoved(
                from + egui::vec2(by * k as f32 / 5.0, 0.0),
            )]);
            seen.push(self.third());
        }
        (seen, from + egui::vec2(by, 0.0))
    }

    fn release(&mut self, at: Pos2) {
        let _ = self.frame(vec![button(at, false)]);
        let _ = self.frame(vec![Event::PointerMoved(layout().figure.center())]);
        self.settle();
    }

    /// A compass drag: pressed at the cube's centre, through each of the angle pairs, degrees of `(τ₁, τ₂)` since it
    /// began (R-451 B3: the width or height 180°, up raising τ₂); returns the third axis at each, and the press point.
    fn compass_drag(&mut self, moves: &[(f64, f64)]) -> (Vec<String>, Pos2) {
        let sq = compass::square(layout().compass);
        let from = sq.center();
        let _ = self.frame(vec![Event::PointerMoved(from)]);
        let _ = self.frame(vec![button(from, true)]);
        let mut seen = Vec::new();
        let mut last = from;
        for &(d1, d2) in moves {
            last = from
                + Vec2::new(
                    (d1 / 180.0 * f64::from(sq.width())) as f32,
                    (-d2 / 180.0 * f64::from(sq.height())) as f32,
                );
            let _ = self.frame(vec![Event::PointerMoved(last)]);
            seen.push(self.third());
        }
        (seen, last)
    }
}

/// One SetField, one history entry.
fn check_one(n: usize, entries: i64) {
    assert_eq!((n, entries), (1, 1), "one SetField, one entry");
}

// === REQ-GUI-208: the buttons on the edges ==========================================================================

fn check_on_edges(rig: &mut Rig, h: &str, third: &str, v: &str) {
    let iso = iso();
    let texts = rig.painted();
    for (want, edge) in [(h, WIDTH_EDGE), (third, DEPTH_EDGE), (v, HEIGHT_EDGE)] {
        let at = label_at(&iso, edge);
        let found = texts
            .iter()
            .find(|t| t.text == want)
            .unwrap_or_else(|| panic!("no label {want:?}: {texts:?}"));
        assert!(
            (found.rect.center() - at).length() < 1.0,
            "{want:?} at {:?}, its edge's place {at:?}",
            found.rect.center()
        );
    }
}

#[test]
fn qa_cube_axis_buttons_sit_on_their_edges_in_both_modes() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    check_on_edges(&mut rig, "H z_α", "D z_q3", "V z_β");
    let numbers = |rig: &mut Rig| {
        rig.painted()
            .iter()
            .filter(|t| t.text == "1.0" || t.text == "−1.0")
            .count()
    };
    assert_eq!(numbers(&mut rig), 0, "tilt mode has no corner numbers");
    rig.mode(COMPASS_SLICE);
    check_on_edges(&mut rig, "H z_α", "D z_q3", "V z_β");
    assert_eq!(numbers(&mut rig), 4, "Slice mode keeps its corner numbers");
    // Controls: the labels on each other's edges.
    rejects("H on the third axis's edge", || {
        check_on_edges(&mut rig, "D z_q3", "H z_α", "V z_β")
    });
    rejects("the old bare name", || {
        check_on_edges(&mut rig, "z_α", "D z_q3", "V z_β")
    });
}

// === REQ-GUI-208: the dropdowns ======================================================================================

/// `offered` is exactly the eight dimensions less `others`, in `AXES`'s order.
fn check_offers(offered: &[usize], others: [usize; 2], what: &str) {
    let want: Vec<usize> = (0..8).filter(|i| !others.contains(i)).collect();
    assert_eq!(offered, want, "{what}");
}

#[test]
fn qa_cube_each_dropdown_lists_the_dimensions_not_on_the_other_two() {
    for (chart, f) in [
        ((ALPHA, BETA), frame(Q3, Q3, Q3)),
        ((ALPHA, BETA), frame(Q2, Q3, MU1)),
        ((Q0, Q1), frame(MU2, MU2, MU2)),
    ] {
        let mut rig = mock();
        rig.chart(chart, f, (0.0, 0.0, 0.0), 1.0);
        let d = f.normal as usize;
        let depth = rig.snap().history.undo_depth;
        // Painted, by a click on each, then closed with Esc; and the keyboard's tree, the same lists.
        for (edge, others, what, letter) in [
            (WIDTH_EDGE, [chart.1, d], "H", "H"),
            (HEIGHT_EDGE, [chart.0, d], "V", "V"),
            (DEPTH_EDGE, [chart.0, chart.1], "D", "D"),
        ] {
            let opened: Vec<usize> = rig.open(edge).iter().map(|(i, _)| *i).collect();
            check_offers(&opened, others, what);
            rig.key(Key::Escape, NONE);
            let id = rig.sibling(letter);
            let tree: Vec<usize> = rig
                .tree_labels(id)
                .iter()
                .map(|l| AXES.iter().position(|a| a == l).unwrap())
                .collect();
            check_offers(&tree, others, what);
        }
        assert_eq!(
            rig.snap().history.undo_depth,
            depth,
            "opening edits nothing"
        );
    }
    // Control: H's list with V's dimension in it.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    let mut opened: Vec<usize> = rig.open(WIDTH_EDGE).iter().map(|(i, _)| *i).collect();
    opened.insert(1, BETA);
    rejects("a dropdown offering V's dimension in H's list", || {
        check_offers(&opened, [BETA, Q3], "H")
    });
}

#[test]
fn qa_cube_the_lists_follow_a_pick_so_no_dimension_can_appear_twice() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    let _ = rig.pick(WIDTH_EDGE, Q0);
    let opened: Vec<usize> = rig.open(HEIGHT_EDGE).iter().map(|(i, _)| *i).collect();
    check_offers(&opened, [Q0, Q3], "V after H = z_q0");
    rig.key(Key::Escape, NONE);
    let opened: Vec<usize> = rig.open(DEPTH_EDGE).iter().map(|(i, _)| *i).collect();
    check_offers(&opened, [Q0, BETA], "D after H = z_q0");
}

// === REQ-GUI-208: H and V ============================================================================================

/// Chart's row `row` ("horizontal q₁", "vertical q₂") reads a value naming `name`: the texts on that row of the panel.
fn check_chart_row(rig: &mut Rig, row: &str, name: &str) {
    let panel = layout().manifold_view;
    let texts = rig.painted();
    let label = texts
        .iter()
        .find(|t| t.text == row && panel.contains_rect(t.rect))
        .unwrap_or_else(|| panic!("no Chart row {row}"));
    let on_row: Vec<&Painted> = texts
        .iter()
        .filter(|t| {
            panel.contains_rect(t.rect)
                && (t.rect.center().y - label.rect.center().y).abs() < 6.0
                && t.text != row
        })
        .collect();
    assert!(
        on_row.iter().any(|t| t.text.contains(name)),
        "Chart's {row} reads {on_row:?}, not {name}"
    );
    for other in AXES.iter().filter(|a| **a != name) {
        assert!(
            !on_row.iter().any(|t| t.text.contains(other)),
            "Chart's {row} reads {on_row:?}, naming {other}"
        );
    }
}

#[test]
fn qa_cube_picking_h_sets_the_pair_resets_the_angles_keeps_zoom_and_z0_in_one_entry() {
    let mut rig = mock();
    let (sigma, t) = (0.5, (20.0, 15.0, 10.0));
    rig.chart((ALPHA, BETA), frame(Q3, Q3, Q3), t, sigma);
    let (n, entries, before, after) = rig.pick(WIDTH_EDGE, Q0);
    assert_eq!((n, entries), (1, 1), "one SetField, one entry");
    assert!(
        close(&after.sim.plane.q1, &e(Q0, sigma), 1e-12),
        "q₁ {:?}",
        after.sim.plane.q1
    );
    assert!(
        close(&after.sim.plane.q2, &e(BETA, sigma), 1e-12),
        "q₂ {:?}",
        after.sim.plane.q2
    );
    assert_eq!(after.sim.plane.z0, before.sim.plane.z0, "z₀ kept");
    let text = rig.compass_text();
    assert!(
        text.contains("τ₁ → z_q3 0.0°")
            && text.contains("τ₂ → z_q3 0.0°")
            && text.contains("γ +0.0°"),
        "the angles reset: {text}"
    );
    check_on_edges(&mut rig, "H z_q0", "D z_q3", "V z_β");
    check_chart_row(&mut rig, "horizontal q₁", "z_q0");
    check_chart_row(&mut rig, "vertical q₂", "z_β");
    rig.undo();
    let undone = rig.snap();
    assert_eq!(undone.sim.plane, before.sim.plane, "one Ctrl+Z");
    assert_eq!(undone.sim.slice.frame, before.sim.slice.frame);
    rejects(
        "H still reading z_α after the undo checked as z_q0",
        || check_on_edges(&mut rig, "H z_q0", "D z_q3", "V z_β"),
    );
    rejects("Chart's row checked against the wrong name", || {
        check_chart_row(&mut rig, "horizontal q₁", "z_q0")
    });
}

#[test]
fn qa_cube_picking_v_sets_chart_vertical_and_one_taking_a_target_resets_it() {
    let mut rig = mock();
    rig.chart((ALPHA, BETA), frame(Q3, MU1, Q3), (0.0, 0.0, 0.0), 1.0);
    let (n, entries, before, after) = rig.pick(HEIGHT_EDGE, MU1);
    assert_eq!((n, entries), (1, 1), "one SetField, one entry");
    assert!(close(&after.sim.plane.q2, &e(MU1, 1.0), 1e-12));
    assert_eq!(
        after.sim.slice.frame,
        frame(Q3, Q3, Q3),
        "a plane that takes X resets it to the normal (R-457 E4), in the entry"
    );
    check_on_edges(&mut rig, "H z_α", "D z_q3", "V z_μ1");
    check_chart_row(&mut rig, "vertical q₂", "z_μ1");
    rig.undo();
    assert_eq!(rig.snap().sim.slice.frame, before.sim.slice.frame);
    assert_eq!(rig.snap().sim.plane, before.sim.plane);
}

/// The anchor keeps its chart point across a basis edit (§G4): `z₀' = z₀ + (2s−1)(q₁ − q₁') + (2t−1)(q₂ − q₂')`.
#[test]
fn qa_cube_a_locked_pick_turns_about_the_pin_in_one_entry() {
    let mut rig = mock();
    rig.chart((ALPHA, BETA), frame(Q3, Q3, Q3), (0.0, 0.0, 0.0), 1.0);
    let z0 = rig.snap().sim.plane.z0;
    let (s, t) = (0.75, 0.25);
    let p = rig.snap().sim.plane;
    let anchor = add(&add(&z0, 2.0 * s - 1.0, &p.q1), 2.0 * t - 1.0, &p.q2);
    rig.set(SimField::Lock(Lock {
        locked: true,
        z_locked: anchor,
    }));
    let (_, entries, before, after) = rig.pick(WIDTH_EDGE, Q0);
    assert_eq!(entries, 1, "a locked pick is one entry");
    let want = add(
        &add(&z0, 2.0 * s - 1.0, &add(&p.q1, -1.0, &e(Q0, 1.0))),
        2.0 * t - 1.0,
        &add(&p.q2, -1.0, &e(BETA, 1.0)),
    );
    assert!(
        close(&after.sim.plane.z0, &want, 1e-12),
        "z₀ {:?}, want {want:?}",
        after.sim.plane.z0
    );
    rejects("the z₀ kept as a free pick keeps it", || {
        assert!(close(&after.sim.plane.z0, &z0, 1e-12))
    });
    rig.undo();
    assert_eq!(rig.snap().sim.plane, before.sim.plane, "one Ctrl+Z");
}

#[test]
fn qa_cube_tilting_a_picked_pair_keeps_its_h_and_v_and_reads_its_angles() {
    // H = z_q0 is no preset's pair: past 45° the tilted axis is not q₁'s largest component, and H must still be z_q0.
    let mut rig = mock();
    rig.chart((Q0, BETA), frame(Q3, Q3, MU1), (60.0, 70.0, 0.0), 1.0);
    check_on_edges(&mut rig, "H z_q0", "X z_q3", "V z_β");
    let text = rig.compass_text();
    assert!(
        text.contains("τ₁ → z_q3 +60.0°") && text.contains("τ₂ → z_μ1 +70.0°"),
        "{text}"
    );
}

/// The pair `z_q0 × z_β`, picked with H, read at each angle to the ends of the tilt's range (±90°, TILT_RANGE; R-459 G3:
/// "the plane becomes the new axis pair", H and V moving only with a pick or an edit in Chart).
#[test]
fn qa_cube_a_picked_pair_keeps_its_h_and_v_to_the_ends_of_the_range() {
    let mut wrong = Vec::new();
    for (t1, t2) in [
        (89.0, 0.0),
        (90.0, 0.0),
        (-90.0, 0.0),
        (0.0, 90.0),
        (90.0, 90.0),
        (-90.0, 45.0),
    ] {
        let mut rig = mock();
        rig.chart((Q0, BETA), frame(Q3, Q3, MU1), (t1, t2, 0.0), 1.0);
        let (h, v) = (rig.on_edge(WIDTH_EDGE).text, rig.on_edge(HEIGHT_EDGE).text);
        if (h.as_str(), v.as_str()) != ("H z_q0", "V z_β") {
            wrong.push(format!("({t1}°, {t2}°): {h}, {v}"));
        }
    }
    assert!(wrong.is_empty(), "H or V moved without a pick: {wrong:?}");
}

/// The user's path to it: pick H = z_q0, drag τ₁ to the slider's end, drag it back. The plane must come back on z_q0.
#[test]
fn qa_cube_a_tilt_dragged_to_90_and_back_stays_on_the_picked_pair() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    let _ = rig.pick(WIDTH_EDGE, Q0);
    let rail = rig.rail(TAU1);
    let (_, end) = rig.slider_drag(TAU1, rail.width());
    rig.release(end);
    let q1 = rig.snap().sim.plane.q1;
    assert!(q1[Q3] > 0.99, "τ₁ at the slider's end: q₁ {q1:?}");
    let (_, end) = rig.slider_drag(TAU1, -rail.width() / 4.0);
    rig.release(end);
    let q1 = rig.snap().sim.plane.q1;
    assert!(
        q1[Q0].abs() > 0.1 && q1[ALPHA] == 0.0,
        "dragged back, q₁ {q1:?} has left the picked z_q0 (H reads {:?})",
        rig.on_edge(WIDTH_EDGE).text
    );
}

#[test]
fn qa_cube_h_and_v_may_be_a_presets_axes_the_other_way_round() {
    // V = z_μ1, then H = z_β, then V = z_α: the pair (z_β, z_α), which is z_α × z_β's axes swapped. H and V must read
    // what was picked, and an angle set on it must read back.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    let _ = rig.pick(HEIGHT_EDGE, MU1);
    let _ = rig.pick(WIDTH_EDGE, BETA);
    let (n, entries, _, after) = rig.pick(HEIGHT_EDGE, ALPHA);
    check_one(n, entries);
    assert!(close(&after.sim.plane.q1, &e(BETA, 1.0), 1e-12));
    assert!(close(&after.sim.plane.q2, &e(ALPHA, 1.0), 1e-12));
    check_on_edges(&mut rig, "H z_β", "D z_q3", "V z_α");
    check_chart_row(&mut rig, "horizontal q₁", "z_β");
    check_chart_row(&mut rig, "vertical q₂", "z_α");
    rig.chart((BETA, ALPHA), frame(Q3, Q3, Q3), (30.0, 0.0, 0.0), 1.0);
    check_on_edges(&mut rig, "H z_β", "X z_q3", "V z_α");
    let text = rig.compass_text();
    assert!(text.contains("τ₁ → z_q3 +30.0°"), "{text}");
}

// === REQ-GUI-208: an edit in Chart moves H and V =====================================================================

fn type_component(rig: &mut Rig, id: &str, value: &str) {
    rig.focus(id);
    let at = rig
        .app
        .keyboard
        .place_of(id)
        .unwrap_or_else(|| panic!("the field {id} is not drawn"))
        .hit
        .center();
    rig.click(at);
    let _ = rig.frame(vec![Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    let _ = rig.frame(vec![Event::Text(value.to_owned())]);
    let _ = rig.frame(vec![Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: NONE,
    }]);
    rig.idle(3);
    rig.key(Key::Escape, NONE);
    rig.settle();
}

#[test]
fn qa_cube_editing_q2_in_chart_moves_v() {
    // Acceptance: editing q₂ in Chart to e_q1 moves V to z_q1. Typed in Chart's own editor.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    let q1_field = rig.scope("q₂ edit…", AXES[Q1]);
    let beta_field = rig.scope("q₂ edit…", AXES[BETA]);
    type_component(&mut rig, q1_field, "1");
    type_component(&mut rig, beta_field, "0");
    let q2 = rig.snap().sim.plane.q2;
    assert!(close(&q2, &e(Q1, 1.0), 1e-12), "q₂ {q2:?}");
    check_on_edges(&mut rig, "H z_α", "D z_q3", "V z_q1");
    rejects("V left on z_β", || {
        check_on_edges(&mut rig, "H z_α", "D z_q3", "V z_β")
    });
}

#[test]
fn qa_cube_a_preset_change_moves_h_and_v() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    let preset = rig.scope("Chart", "preset");
    rig.focus(preset);
    rig.key(Key::Enter, NONE);
    rig.settle();
    // The next preset after z_α × z_β is z_q0 × z_q1 (§G2's list).
    check_on_edges(&mut rig, "H z_q0", "D z_q3", "V z_q1");
    check_chart_row(&mut rig, "horizontal q₁", "z_q0");
}

// === REQ-GUI-208: D ==================================================================================================

#[test]
fn qa_cube_picking_d_sets_the_normal_and_retilts_a_following_target_in_one_entry() {
    // Slice mode, where the third axis is D: at τ₁ = 20° with X = D, D = z_μ1 re-tilts q₁ into e_μ1.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q2), 20.0, 0.0);
    rig.mode(COMPASS_SLICE);
    let (n, entries, before, after) = rig.pick(DEPTH_EDGE, MU1);
    check_one(n, entries);
    assert_eq!(
        after.sim.slice.frame,
        frame(MU1, MU1, Q2),
        "the normal, X following it, Y its own"
    );
    let (q1, q2) = map(ALPHA, BETA, MU1, Q2, 20.0, 0.0, 0.0, 1.0);
    assert!(
        close(&after.sim.plane.q1, &q1, 1e-12),
        "q₁ {:?}",
        after.sim.plane.q1
    );
    assert!(close(&after.sim.plane.q2, &q2, 1e-12));
    assert_eq!(after.sim.plane.z0, before.sim.plane.z0, "z₀ kept");
    check_on_edges(&mut rig, "H z_α", "D z_μ1", "V z_β");
    rig.undo();
    assert_eq!(rig.snap().sim.plane, before.sim.plane, "one Ctrl+Z: basis");
    assert_eq!(
        rig.snap().sim.slice.frame,
        before.sim.slice.frame,
        "one Ctrl+Z: frame"
    );
    // Control: the same pick made of two entries, the frame and then the basis, as a D pick must not be.
    let (d0, n0) = (rig.snap().history.undo_depth, rig.applied());
    rig.set(SimField::SliceFrame {
        frame: frame(MU1, MU1, Q2),
        basis: None,
    });
    rig.set(SimField::Basis { q1, q2 });
    let (n, entries) = (
        rig.applied() - n0,
        i64::from(rig.snap().history.undo_depth) - i64::from(d0),
    );
    rejects("a D pick that makes two entries", || check_one(n, entries));
}

#[test]
fn qa_cube_picking_d_at_rest_edits_the_frame_alone_and_a_target_off_it_keeps_its_own() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q2, Q3), 0.0, 0.0);
    let (n, entries, before, after) = rig.pick(DEPTH_EDGE, MU2);
    assert_eq!((n, entries), (1, 1));
    assert_eq!(after.sim.slice.frame, frame(MU2, Q2, MU2));
    assert_eq!(after.sim.plane, before.sim.plane, "nothing re-tilted");
    check_on_edges(&mut rig, "H z_α", "D z_μ2", "V z_β");
}

#[test]
fn qa_cube_the_third_axis_is_the_shown_targets_dropdown_in_tilt_mode() {
    // R-459 G2: while it shows Y, a pick is Y's: the normal and X keep, q₂ re-tilts, one entry.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 0.0, 15.0);
    assert_eq!(rig.third(), "Y z_μ1");
    let opened: Vec<usize> = rig.open(DEPTH_EDGE).iter().map(|(i, _)| *i).collect();
    check_offers(&opened, [ALPHA, BETA], "the target's list");
    rig.key(Key::Escape, NONE);
    let (n, entries, before, after) = rig.pick(DEPTH_EDGE, Q2);
    assert_eq!((n, entries), (1, 1));
    assert_eq!(after.sim.slice.frame, frame(Q3, Q3, Q2), "Y alone");
    let (_, q2) = map(ALPHA, BETA, Q3, Q2, 0.0, 15.0, 0.0, 1.0);
    assert!(close(&after.sim.plane.q2, &q2, 1e-12));
    assert_eq!(rig.third(), "Y z_q2");
    rig.undo();
    assert_eq!(rig.snap().sim.slice.frame, before.sim.slice.frame);
    assert_eq!(rig.snap().sim.plane, before.sim.plane);
}

// === REQ-GUI-208: the slice-axis rule ================================================================================

fn steer_cell(rig: &mut Rig, i: usize, j: usize) -> (usize, i64, Snapshot, Snapshot) {
    rig.app.manifold.compass.view = CompassView::Plucker;
    rig.idle(2);
    let at = GridPlace::fit(compass::square(layout().compass))
        .cell_rect(i, j)
        .center();
    let (before, n0) = (rig.snap(), rig.applied());
    rig.click(at);
    rig.app.manifold.compass.view = CompassView::Cube;
    rig.settle();
    let after = rig.snap();
    let entries = i64::from(after.history.undo_depth) - i64::from(before.history.undo_depth);
    (rig.applied() - n0, entries, before, after)
}

#[test]
fn qa_cube_a_plucker_steer_onto_d_moves_d_by_the_slice_axis_rule_in_one_entry() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    let (n, entries, before, after) = steer_cell(&mut rig, Q2, Q3);
    assert_eq!((n, entries), (1, 1), "one SetField, one entry");
    assert!(close(&after.sim.plane.q2, &e(Q3, 1.0), 1e-12));
    // z_q3 is in the plane, so the first axis the plane does not show: z_α; the targets on the old normal follow.
    assert_eq!(after.sim.slice.frame, frame(ALPHA, ALPHA, ALPHA));
    check_on_edges(&mut rig, "H z_q2", "D z_α", "V z_q3");
    rig.undo();
    assert_eq!(rig.snap().sim.slice.frame, before.sim.slice.frame);
    assert_eq!(rig.snap().sim.plane, before.sim.plane);
    // A steer that leaves D keeps it.
    let (_, _, _, after) = steer_cell(&mut rig, Q0, Q1);
    assert_eq!(after.sim.slice.frame, frame(Q3, Q3, Q3));
    // Control: the plane z_q2 × z_q3 with D left on z_q3, set straight on the contract, is what the rule refuses.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    rig.set(SimField::Basis {
        q1: e(Q2, 1.0),
        q2: e(Q3, 1.0),
    });
    let left = rig.snap().sim.slice.frame;
    rejects("a D left in the plane", || {
        assert_eq!(left, frame(ALPHA, ALPHA, ALPHA))
    });
}

#[test]
fn qa_cube_a_plucker_space_steer_moves_d_too() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    rig.app.manifold.compass.view = CompassView::Plucker;
    rig.idle(2);
    rig.focus(COMPASS_VIEW);
    rig.key(Key::Enter, NONE);
    assert_eq!(rig.app.view.focus.mode, KeyMode::Interaction);
    rig.app.manifold.compass.cursor = Some((Q2, Q3));
    let depth = rig.snap().history.undo_depth;
    rig.key(Key::Space, NONE);
    let after = rig.snap();
    assert_eq!(after.history.undo_depth, depth + 1, "one entry");
    assert_eq!(after.sim.slice.frame, frame(ALPHA, ALPHA, ALPHA));
}

// === REQ-GUI-208: the keys ===========================================================================================

#[test]
fn qa_cube_h_v_d_are_siblings_after_the_view_buttons() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    let got = rig.tree_labels(COMPASS);
    let check = |got: &[&str]| {
        let at = |l: &str| {
            got.iter()
                .position(|g| *g == l)
                .unwrap_or_else(|| panic!("{l} not in {got:?}"))
        };
        assert_eq!(at("Tilt") + 1, at("Slice"));
        assert_eq!(at("Shape") + 1, at("H"), "{got:?}");
        assert_eq!(at("H") + 1, at("V"), "{got:?}");
        assert_eq!(at("V") + 1, at("D"), "{got:?}");
        assert_eq!(at("D") + 1, got.len(), "{got:?}");
    };
    check(&got);
    let mut wrong = got.clone();
    wrong.swap(1, got.len() - 1);
    rejects("H before the view buttons", || check(&wrong));
    // In a grid view the cube's buttons are not drawn, so they are no siblings.
    rig.app.manifold.compass.view = CompassView::Plucker;
    rig.idle(2);
    let grid = rig.tree_labels(COMPASS);
    assert!(
        !grid.iter().any(|l| ["H", "V", "D"].contains(l)),
        "a grid view's siblings {grid:?}"
    );
    // Back, in their order again.
    rig.app.manifold.compass.view = CompassView::Cube;
    rig.idle(2);
    check(&rig.tree_labels(COMPASS));
}

#[test]
fn qa_cube_keyboard_tab_to_the_compass_enter_right_to_h_enter_down_enter_picks() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 0.0, 0.0);
    for _ in 0..12 {
        rig.key(Key::Tab, NONE);
        if rig
            .app
            .view
            .focus
            .path
            .first()
            .is_some_and(|f| f == COMPASS)
        {
            break;
        }
    }
    assert_eq!(rig.focus_labels().len(), 1, "on the Compass big scope");
    assert!(rig.app.view.focus.path[0] == COMPASS, "Tab to the Compass");
    rig.key(Key::Enter, NONE);
    for _ in 0..12 {
        if rig.focus_labels().last().is_some_and(|l| l == "H") {
            break;
        }
        rig.key(Key::ArrowRight, NONE);
    }
    assert_eq!(rig.focus_labels().last().unwrap(), "H", "→ to H");
    // Esc closes it unchanged (G9).
    let before = rig.snap();
    let closed = rig.name_counts();
    rig.key(Key::Enter, NONE);
    rig.idle(1);
    let open = rig.name_counts();
    assert_ne!(open, closed, "Enter did not open H's dropdown on screen");
    rig.key(Key::ArrowDown, NONE);
    rig.key(Key::Escape, NONE);
    assert_eq!(
        rig.snap().sim.plane,
        before.sim.plane,
        "Esc changes nothing"
    );
    assert_eq!(rig.snap().history.undo_depth, before.history.undo_depth);
    if rig.focus_labels().last().is_some_and(|l| l != "H") {
        rig.key(Key::Escape, NONE);
    }
    assert_eq!(rig.focus_labels().last().unwrap(), "H");
    let n0 = rig.applied();
    rig.key(Key::Enter, NONE);
    rig.key(Key::ArrowDown, NONE);
    rig.key(Key::Enter, NONE);
    rig.settle();
    let after = rig.snap();
    assert_eq!(rig.applied() - n0, 1, "the pick by keys, one SetField");
    assert_eq!(after.history.undo_depth, before.history.undo_depth + 1);
    assert!(
        after.sim.plane.q1 != before.sim.plane.q1,
        "the pick moved q₁: {:?}",
        after.sim.plane.q1
    );
    let picked = (0..8).find(|&i| after.sim.plane.q1[i] != 0.0).unwrap();
    assert!(![BETA, Q3].contains(&picked), "picked {}", AXES[picked]);
    assert_eq!(rig.third(), "D z_q3");
    assert_eq!(rig.on_edge(WIDTH_EDGE).text, format!("H {}", AXES[picked]));
    rig.undo();
    assert_eq!(rig.snap().sim.plane, before.sim.plane, "one Ctrl+Z");
}

#[test]
fn qa_cube_the_third_axis_scope_is_named_by_what_it_shows() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 0.0, 15.0);
    assert!(rig.tree_labels(COMPASS).contains(&"Y"));
    let id = rig.sibling("Y");
    rig.focus(id);
    assert_eq!(rig.focus_labels().last().unwrap(), "Y");
    // Enter ↓ Enter picks Y's target by keys, one entry.
    let before = rig.snap();
    rig.key(Key::Enter, NONE);
    rig.key(Key::ArrowDown, NONE);
    rig.key(Key::Enter, NONE);
    rig.settle();
    let after = rig.snap();
    assert_eq!(after.history.undo_depth, before.history.undo_depth + 1);
    assert_eq!(after.sim.slice.frame.normal, Q3 as u32, "the normal keeps");
    assert_ne!(after.sim.slice.frame.tilt_y, MU1 as u32, "Y moved");
}

// === REQ-GUI-209: the rule at rest ===================================================================================

#[test]
fn qa_cube_third_axis_at_rest_follows_the_rule_and_the_readout_names_both_targets() {
    let f = frame(Q3, Q3, MU1);
    for (mode, t1, t2, want, angles) in [
        (COMPASS_TILT, 0.0, 0.0, "D z_q3", ("0.0", "0.0")),
        (COMPASS_TILT, 20.0, 0.0, "X z_q3", ("+20.0", "0.0")),
        (COMPASS_TILT, 0.0, 15.0, "Y z_μ1", ("0.0", "+15.0")),
        (COMPASS_TILT, 20.0, 15.0, "X z_q3", ("+20.0", "+15.0")),
        (COMPASS_TILT, -20.0, 0.0, "X z_q3", ("-20.0", "0.0")),
        (COMPASS_SLICE, 0.0, 0.0, "D z_q3", ("0.0", "0.0")),
        (COMPASS_SLICE, 20.0, 15.0, "D z_q3", ("+20.0", "+15.0")),
    ] {
        let mut rig = mock();
        rig.tilted(f, t1, t2);
        rig.mode(mode);
        assert_eq!(rig.third(), want, "{mode} ({t1}°, {t2}°)");
        let text = rig.compass_text();
        let (one, two) = (
            format!("τ₁ → z_q3 {}°", angles.0),
            format!("τ₂ → z_μ1 {}°", angles.1),
        );
        assert!(
            text.contains(&one) && text.contains(&two),
            "{mode} ({t1}°, {t2}°): the readout {text}"
        );
    }
    // A normal apart from both targets: D at rest is the normal, not a target.
    let mut rig = mock();
    rig.tilted(frame(Q2, Q3, MU1), 0.0, 0.0);
    assert_eq!(rig.third(), "D z_q2");
    // Controls: a third axis staying on the normal in Tilt mode, a readout naming one target.
    let mut rig = mock();
    rig.tilted(f, 20.0, 0.0);
    rejects("a third axis on the slice normal in Tilt mode", || {
        assert_eq!(rig.third(), "D z_q3")
    });
    let text = rig.compass_text();
    rejects("a readout naming one target", || {
        assert!(text.contains("τ₁ → z_q3") && text.contains("τ₂ → z_q3"))
    });
}

// === REQ-GUI-209: while dragged ======================================================================================

#[test]
fn qa_cube_third_axis_during_a_tau2_slider_drag_reads_y_back_to_x_when_it_ends() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 20.0, 0.0);
    assert_eq!(rig.third(), "X z_q3");
    let (seen, end) = rig.slider_drag(TAU2, 30.0);
    assert!(
        seen.iter().all(|s| s == "Y z_μ1"),
        "during the τ₂ drag: {seen:?}"
    );
    let text = rig.compass_text();
    assert!(
        text.contains("τ₁ → z_q3 +20.0°") && text.contains("τ₂ → z_μ1 +"),
        "the readout names both targets during the drag: {text}"
    );
    rig.release(end);
    assert_eq!(rig.third(), "X z_q3", "back to X when the drag ends");
    rejects("the third axis left on Y after the drag", || {
        assert_eq!(rig.third(), "Y z_μ1")
    });
}

#[test]
fn qa_cube_third_axis_during_a_tau1_slider_drag_reads_x_from_slice_mode_too() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q2, MU1), 0.0, 15.0);
    rig.mode(COMPASS_SLICE);
    assert_eq!(rig.third(), "D z_q3");
    let (seen, end) = rig.slider_drag(TAU1, 30.0);
    assert!(
        seen.iter().all(|s| s == "X z_q2"),
        "a drag on a tilt slider switches to Tilt (R-458) and shows X: {seen:?}"
    );
    rig.release(end);
    assert_eq!(rig.third(), "X z_q2", "τ₁ ≠ 0 at rest");
}

#[test]
fn qa_cube_third_axis_during_a_compass_drag_shows_the_angle_moved_further_since_it_began() {
    // Acceptance: τ₁ by 10° and τ₂ by 30° reads Y.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 0.0, 0.0);
    let (seen, end) = rig.compass_drag(&[(5.0, 15.0), (10.0, 30.0)]);
    assert_eq!(seen.last().unwrap(), "Y z_μ1", "{seen:?}");
    rig.release(end);
    // At rest both are set: X.
    assert_eq!(rig.third(), "X z_q3");
    // τ₁ further: X.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 0.0, 0.0);
    let (seen, end) = rig.compass_drag(&[(15.0, 5.0), (30.0, 10.0)]);
    assert_eq!(seen.last().unwrap(), "X z_q3", "{seen:?}");
    rig.release(end);
    // A tie: τ₁.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 0.0, 0.0);
    let (seen, end) = rig.compass_drag(&[(20.0, 20.0)]);
    assert_eq!(seen.last().unwrap(), "X z_q3", "a tie is τ₁: {seen:?}");
    rig.release(end);
    // Since it began, not over its path: τ₂ out by 40° and back to 5°, τ₁ out to 10°, from a start with both set.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 20.0, 15.0);
    let (seen, end) = rig.compass_drag(&[(1.0, 40.0), (10.0, 5.0)]);
    assert_eq!(seen[0], "Y z_μ1", "{seen:?}");
    assert_eq!(seen[1], "X z_q3", "{seen:?}");
    rig.release(end);
    // Control: a rule that keeps τ₁ whatever the drag.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 0.0, 0.0);
    let (seen, end) = rig.compass_drag(&[(10.0, 30.0)]);
    rejects("a compass drag read as τ₁ however it moved", || {
        assert_eq!(seen.last().unwrap(), "X z_q3")
    });
    rig.release(end);
}

// === REQ-GUI-209: the colours ========================================================================================

/// The colour the section of `text` naming `part` is painted in.
fn section_colour(texts: &[Painted], part: &str) -> Color32 {
    texts
        .iter()
        .flat_map(|t| t.sections.iter())
        .find(|(s, _)| s.contains(part))
        .unwrap_or_else(|| panic!("no section {part} in {texts:?}"))
        .1
}

#[test]
fn qa_cube_the_third_axis_label_takes_the_active_gestures_colour() {
    let label_colour = |edge, t1, t2, mode: &str| -> (Color32, Color32, Color32, Color32) {
        let mut rig = mock();
        rig.tilted(frame(Q3, Q3, MU1), t1, t2);
        rig.mode(mode);
        let third = rig.on_edge(edge).sections[0].1;
        let h = rig.on_edge(WIDTH_EDGE).sections[0].1;
        let v = rig.on_edge(HEIGHT_EDGE).sections[0].1;
        let texts = rig.painted();
        let rect = layout().compass;
        let compass: Vec<Painted> = texts
            .into_iter()
            .filter(|t| rect.contains(t.rect.center()))
            .collect();
        let part = if t1 != 0.0 && mode == COMPASS_TILT {
            "τ₁ →"
        } else if mode == COMPASS_TILT {
            "τ₂ →"
        } else {
            "z_q3 ="
        };
        (third, h, v, section_colour(&compass, part))
    };
    let (slice, h1, v1, slice_line) = label_colour(DEPTH_EDGE, 0.0, 0.0, COMPASS_SLICE);
    let (tau1, h2, v2, tau1_line) = label_colour(DEPTH_EDGE, 20.0, 0.0, COMPASS_TILT);
    let (tau2, h3, v3, tau2_line) = label_colour(DEPTH_EDGE, 0.0, 15.0, COMPASS_TILT);
    // Three distinct gesture colours, each on its own readout line (G7).
    let distinct = |a: Color32, b: Color32, c: Color32| {
        assert!(a != b && a != c && b != c, "{a:?} {b:?} {c:?}");
    };
    distinct(slice, tau1, tau2);
    assert_eq!(tau1_line, tau1, "τ₁'s readout part takes its colour");
    assert_eq!(tau2_line, tau2, "τ₂'s readout part takes its colour");
    assert_eq!(
        slice_line, slice,
        "the slice's readout line takes its colour"
    );
    // H and V keep the theme's text colour in every case, none a gesture's.
    for c in [h1, v1, h2, v2, h3, v3] {
        assert_eq!(c, h1, "H and V take the one theme colour");
        assert!(
            ![tau1, tau2, slice].contains(&c),
            "H or V in a gesture colour"
        );
    }
    // The readout part of the other target is not in the active colour.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 20.0, 0.0);
    let texts = rig.painted();
    let other = section_colour(&texts, "τ₂ →");
    assert_ne!(other, tau1, "the other target's part in the active colour");
    // Control: one colour for all gestures.
    rejects("one colour for τ₁ and τ₂", || {
        distinct(slice, tau1, tau1)
    });
}

#[test]
fn qa_cube_a_tau2_drag_changes_the_labels_colour_while_it_lasts() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 20.0, 0.0);
    let rest = rig.on_edge(DEPTH_EDGE).sections[0].1;
    let (_, end) = rig.slider_drag(TAU2, 30.0);
    let during = rig.on_edge(DEPTH_EDGE).sections[0].1;
    assert_ne!(
        during, rest,
        "the label's colour stays τ₁'s during τ₂'s drag"
    );
    rig.release(end);
    assert_eq!(rig.on_edge(DEPTH_EDGE).sections[0].1, rest);
}

// === REQ-GUI-209: the plane ==========================================================================================

/// `(q₁, q₂)` in `(e_a, e_b, e_T)`, of R-457 E1's map at `(τ₁, τ₂)` with `X` and `Y`, in that frame's components.
fn in_frame(q: &Z, t: usize) -> P3 {
    [q[ALPHA], q[BETA], q[t]]
}

fn plane_in(x: usize, y: usize, t1: f64, t2: f64, t: usize, p: P3) -> Vec<P3> {
    let (q1, q2) = map(ALPHA, BETA, x, y, t1, t2, 0.0, 1.0);
    cross_section(in_frame(&q1, t), in_frame(&q2, t), p)
}

#[test]
fn qa_cube_draws_the_gestures_plane_with_the_other_angle_at_0() {
    let iso = iso();
    // (20°, 15°), X = z_q3, Y = z_μ1: the cube is View::basis at (20°, 0°) in (e_a, e_b, e_q3).
    let mut rig = mock();
    rig.centred(frame(Q3, Q3, MU1), 20.0, 15.0);
    let fill = rig.fill();
    let want = plane_in(Q3, MU1, 20.0, 0.0, Q3, [0.0; 3]);
    assert_eq!(want.len(), 4, "a rectangle tilted about the cube's y");
    check_polygon(&fill, &want, &iso);
    // Controls: the plane with both angles, and the plane in Y's frame.
    let both = plane_in(Q3, Q3, 20.0, 15.0, Q3, [0.0; 3]);
    rejects("the plane drawn with τ₂ too", || {
        check_polygon(&fill, &both, &iso)
    });
    let in_y = plane_in(Q3, MU1, 0.0, 15.0, MU1, [0.0; 3]);
    rejects("the other gesture's plane", || {
        check_polygon(&fill, &in_y, &iso)
    });
    // τ₁ = 0, τ₂ = 15°: the Y gesture, in (e_a, e_b, e_μ1).
    let mut rig = mock();
    rig.centred(frame(Q3, Q3, MU1), 0.0, 15.0);
    check_polygon(
        &rig.fill(),
        &plane_in(Q3, MU1, 0.0, 15.0, MU1, [0.0; 3]),
        &iso,
    );
}

#[test]
fn qa_cube_during_a_tau2_drag_it_draws_tau2s_plane() {
    let iso = iso();
    let mut rig = mock();
    rig.centred(frame(Q3, Q3, MU1), 20.0, 15.0);
    let (_, end) = rig.slider_drag(TAU2, 30.0);
    let tau2_now = {
        let q2 = rig.snap().sim.plane.q2;
        q2[MU1].atan2(q2[BETA]).to_degrees()
    };
    // Eased: run the drawing to rest with the pointer held still.
    rig.settle();
    check_polygon(
        &rig.fill(),
        &plane_in(Q3, MU1, 0.0, tau2_now, MU1, [0.0; 3]),
        &iso,
    );
    rig.release(end);
    // Back at rest, X's plane.
    let tau1_plane = plane_in(Q3, MU1, 20.0, 0.0, Q3, [0.0; 3]);
    check_polygon(&rig.fill(), &tau1_plane, &iso);
}

#[test]
fn qa_cube_the_drawn_plane_passes_through_z0s_point_on_the_shown_frame() {
    // z₀'s component on the shown target over the half-range places the plane: on z_μ1 for Y, not on the normal.
    let iso = iso();
    let mut rig = mock();
    rig.centred(frame(Q3, Q3, MU1), 0.0, 15.0);
    let mut z0 = rig.snap().sim.plane.z0;
    z0[ALPHA] = 0.0;
    z0[BETA] = 0.0;
    z0[Q3] = 0.5 * Z0_HALF_RANGE;
    z0[MU1] = 0.25 * Z0_HALF_RANGE;
    rig.set(SimField::Z0(z0));
    rig.settle();
    let want = plane_in(Q3, MU1, 0.0, 15.0, MU1, [0.0, 0.0, 0.25]);
    check_polygon(&rig.fill(), &want, &iso);
    let wrong = plane_in(Q3, MU1, 0.0, 15.0, MU1, [0.0, 0.0, 0.5]);
    let fill = rig.fill();
    rejects("the plane through the normal's component", || {
        check_polygon(&fill, &wrong, &iso)
    });
}

#[test]
fn qa_cube_with_one_target_the_whole_plane_is_drawn_and_no_badge() {
    // X = Y = z_q3 at (20°, 15°): R-451 B2's plane, both angles in (e_a, e_b, e_q3).
    let iso = iso();
    let mut rig = mock();
    rig.centred(frame(Q3, Q3, Q3), 20.0, 15.0);
    let fill = rig.fill();
    check_polygon(&fill, &plane_in(Q3, Q3, 20.0, 15.0, Q3, [0.0; 3]), &iso);
    assert!(
        !rig.painted().iter().any(|t| t.text == BADGE),
        "a badge with X = Y"
    );
    rejects("the plane with τ₂ dropped", || {
        check_polygon(&fill, &plane_in(Q3, Q3, 20.0, 0.0, Q3, [0.0; 3]), &iso)
    });
}

#[test]
fn qa_cube_slice_mode_draws_b2s_plane_in_the_normals_frame() {
    // G6: Slice mode with one angle set into a target off D: (e_a, e_b, e_D), where that tilt has no D component.
    let iso = iso();
    let mut rig = mock();
    rig.centred(frame(Q3, Q3, MU1), 0.0, 15.0);
    rig.mode(COMPASS_SLICE);
    let (q1, q2) = map(ALPHA, BETA, Q3, MU1, 0.0, 15.0, 0.0, 1.0);
    let want = cross_section(in_frame(&q1, Q3), in_frame(&q2, Q3), [0.0; 3]);
    check_polygon(&rig.fill(), &want, &iso);
    // G6: with both set and X ≠ Y, the third axis's rule's plane: X's, the badge shown.
    let mut rig = mock();
    rig.centred(frame(Q3, Q2, MU1), 20.0, 15.0);
    rig.mode(COMPASS_SLICE);
    assert!(rig.painted().iter().any(|t| t.text == BADGE), "the badge");
    check_polygon(
        &rig.fill(),
        &plane_in(Q2, MU1, 20.0, 0.0, Q2, [0.0; 3]),
        &iso,
    );
}

// === REQ-GUI-209: the badge ==========================================================================================

/// Whether the badge is painted.
fn badge(rig: &mut Rig) -> Option<Rect> {
    rig.painted()
        .iter()
        .find(|t| t.text == BADGE)
        .map(|t| t.rect)
}

#[test]
fn qa_cube_the_badge_shows_only_with_both_angles_set_into_different_targets() {
    for (f, t1, t2, want) in [
        (frame(Q3, Q3, MU1), 20.0, 15.0, true),
        (frame(Q3, Q3, MU1), -20.0, -15.0, true),
        (frame(Q3, Q3, MU1), 20.0, 0.0, false),
        (frame(Q3, Q3, MU1), 0.0, 15.0, false),
        (frame(Q3, Q3, MU1), 0.0, 0.0, false),
        (frame(Q3, Q3, Q3), 20.0, 15.0, false),
        (frame(Q3, MU1, MU1), 20.0, 15.0, false),
    ] {
        let mut rig = mock();
        rig.tilted(f, t1, t2);
        assert_eq!(badge(&mut rig).is_some(), want, "{f:?} ({t1}°, {t2}°)");
    }
    // Control: a check that reads the badge at X = Y as shown.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 20.0, 15.0);
    rejects("a badge at X = Y", || assert!(badge(&mut rig).is_some()));
}

#[test]
fn qa_cube_the_badge_click_sets_the_plucker_view_and_edits_nothing() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 20.0, 15.0);
    let at = badge(&mut rig).expect("the badge").center();
    let (before, n0) = (rig.snap(), rig.applied());
    rig.click(at);
    assert_eq!(rig.app.manifold.compass.view, CompassView::Plucker);
    assert_eq!(rig.app.manifold.compass.mode, Touch::Tilt, "the mode stays");
    assert_eq!(rig.applied(), n0, "the view is the compass's own state");
    assert_eq!(rig.snap().history.undo_depth, before.history.undo_depth);
    assert!(badge(&mut rig).is_none(), "no badge in the Plücker view");
    // Control: a click beside it does not switch.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, Q3), 20.0, 15.0);
    rig.click(at);
    rejects("the click where no badge is shown", || {
        assert_eq!(rig.app.manifold.compass.view, CompassView::Plucker)
    });
}

#[test]
fn qa_cube_the_badge_sits_in_the_top_right_corner_clear_of_the_drawing() {
    // G8: "the cube's top-right corner, clear of the drawing": it touches none of the cube's corners or edges.
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 20.0, 15.0);
    let rect = badge(&mut rig).expect("the badge");
    let sq = compass::square(layout().compass);
    assert!(
        rect.center().x > sq.center().x && rect.center().y < sq.center().y,
        "the badge at {rect:?} in the square {sq:?}"
    );
    let iso = iso();
    let check = |rect: Rect| {
        // The cube's outer edges, sampled: the silhouette's hexagon and everything inside it is the drawing.
        let corners: [P3; 6] = [
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, -1.0, 1.0],
            [-1.0, -1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        for (i, a) in corners.iter().enumerate() {
            let b = corners[(i + 1) % 6];
            for k in 0..=50 {
                let t = f64::from(k) / 50.0;
                let p: P3 = std::array::from_fn(|n| a[n] + (b[n] - a[n]) * t);
                let [px, py] = iso.at(p);
                assert!(
                    !rect.contains(egui::pos2(px as f32, py as f32)),
                    "the badge {rect:?} covers the cube's outline at {p:?}: ({px}, {py})"
                );
            }
        }
    };
    let button = rig
        .app
        .keyboard
        .place_of(gui::explore::COMPASS_BADGE)
        .expect("placed")
        .rect;
    check(button);
    rejects("a badge on the cube's centre", || {
        let [px, py] = iso.at([-1.0, 1.0, -1.0]);
        check(Rect::from_center_size(
            egui::pos2(px as f32, py as f32),
            egui::vec2(10.0, 10.0),
        ))
    });
}

#[test]
fn qa_cube_the_badge_is_the_last_sibling_and_enter_on_it_sets_the_plucker_view() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 20.0, 15.0);
    let labels = rig.tree_labels(COMPASS);
    assert_eq!(labels.last().unwrap(), &BADGE, "{labels:?}");
    let id = rig.sibling(BADGE);
    rig.focus(id);
    let depth = rig.snap().history.undo_depth;
    rig.key(Key::Enter, NONE);
    assert_eq!(rig.app.manifold.compass.view, CompassView::Plucker);
    assert_eq!(rig.snap().history.undo_depth, depth);
    let labels = rig.tree_labels(COMPASS);
    assert!(!labels.contains(&BADGE), "gone with the cube: {labels:?}");
    assert!(
        !rig.focus_labels().is_empty()
            && rig
                .app
                .keyboard
                .tree(Mode::Explore)
                .valid_prefix(&rig.app.view.focus.path)
                == rig.app.view.focus.path.len(),
        "the focus left on a scope that is gone: {:?}",
        rig.app.view.focus.path
    );
}

#[test]
fn qa_cube_a_focus_on_the_badge_when_it_goes_stays_in_the_tree() {
    let mut rig = mock();
    rig.tilted(frame(Q3, Q3, MU1), 20.0, 15.0);
    let id = rig.sibling(BADGE);
    rig.focus(id);
    rig.set(SimField::Basis {
        q1: e(ALPHA, 1.0),
        q2: e(BETA, 1.0),
    });
    let tree = rig.app.keyboard.tree(Mode::Explore);
    assert_eq!(
        tree.valid_prefix(&rig.app.view.focus.path),
        rig.app.view.focus.path.len(),
        "the focus {:?} on a scope that left",
        rig.app.view.focus.path
    );
    assert_eq!(
        rig.app.view.focus.path[0], COMPASS,
        "it stays in the Compass"
    );
}

// === REQ-GUI-208: a physical steer that takes D (R-459 G4, render_gui_spec § "The compass: look and views") ==========

/// `J` is this matrix over the ranges, so `J⁺ e_i ∝ e_i`: a cell's steer is the coordinate plane, whatever the scale.
fn identity_j() -> &'static Matrix {
    Box::leak(Box::new(std::array::from_fn(|r| e(r, 1.0))))
}

/// A full-rank `J` whose preimages are off every axis, from a fixed-seed splitmix64: planes that only read on axes.
fn dense_j() -> &'static Matrix {
    let mut x = 0x4d36_u64;
    let mut next = move || {
        x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    };
    let a: Matrix = std::array::from_fn(|r| {
        std::array::from_fn(|c| 0.4 * (next() - 0.5) + if r == c { 1.0 } else { 0.0 })
    });
    Box::leak(Box::new(a))
}

/// The slice-axis rule as G4 states it, for planes on two axes: `z_q3`, or the first axis the plane does not show.
fn slice_axis_of(i: usize, j: usize) -> usize {
    if i != Q3 && j != Q3 {
        return Q3;
    }
    (0..8).find(|k| *k != i && *k != j).unwrap()
}

/// A rig on the linear `J` in a physical view, at a tilted plane (`z_β × z_μ2`) no cell's steer leaves as it is, in `f`.
fn physical_rig(view: CompassView, j: &'static Matrix, f: SliceFrame) -> Rig {
    let mut rig = mock();
    rig.chart((BETA, MU2), f, (0.35, 0.0, 0.0), 1.0);
    rig.app.manifold.compass.decoder = Decoder::Linear { a: j, shell: false };
    rig.app.manifold.compass.view = view;
    rig.idle(2);
    rig
}

fn click_cell(rig: &mut Rig, i: usize, j: usize) {
    let at = GridPlace::fit(compass::square(layout().compass))
        .cell_rect(i, j)
        .center();
    rig.click(at);
}

fn space_cell(rig: &mut Rig, i: usize, j: usize) {
    rig.focus(COMPASS_VIEW);
    rig.key(Key::Enter, NONE);
    assert_eq!(rig.app.view.focus.mode, KeyMode::Interaction);
    rig.app.manifold.compass.cursor = Some((i, j));
    rig.key(Key::Space, NONE);
}

#[test]
fn qa_cube_a_physical_steer_moves_d_and_the_targets_by_the_rule_in_one_entry() {
    let cases = [
        // Takes D: the rule's first axis not shown, the targets on the old D following.
        ((Q2, Q3), frame(Q3, Q3, Q3), frame(ALPHA, ALPHA, ALPHA)),
        ((ALPHA, Q3), frame(Q3, Q3, Q3), frame(BETA, BETA, BETA)),
        // Takes a D that is not z_q3: back to z_q3.
        ((ALPHA, BETA), frame(ALPHA, ALPHA, ALPHA), frame(Q3, Q3, Q3)),
        // Takes D, with targets of their own off the plane: they stay.
        ((Q2, Q3), frame(Q3, MU1, Q0), frame(ALPHA, MU1, Q0)),
        // Takes a target alone: it resets to D (REQ-GUI-205).
        ((Q0, Q1), frame(Q3, Q0, MU1), frame(Q3, Q3, MU1)),
        // Takes nothing: the frame stays.
        ((Q0, Q1), frame(Q3, Q3, Q3), frame(Q3, Q3, Q3)),
        ((Q0, Q1), frame(Q3, MU1, Q2), frame(Q3, MU1, Q2)),
    ];
    for view in [CompassView::Jacobi, CompassView::Shape] {
        for by_space in [false, true] {
            for ((i, j), start, want) in cases {
                let at = format!("{view:?} ({i}, {j}) from {start:?}, Space {by_space}");
                let mut rig = physical_rig(view, identity_j(), start);
                let (before, n0) = (rig.snap(), rig.applied());
                if by_space {
                    space_cell(&mut rig, i, j);
                } else {
                    click_cell(&mut rig, i, j);
                }
                let after = rig.snap();
                assert_eq!(rig.applied() - n0, 1, "one SetField: {at}");
                check_one(
                    1,
                    i64::from(after.history.undo_depth) - i64::from(before.history.undo_depth),
                );
                assert!(close(&after.sim.plane.q1, &e(i, 1.0), 1e-9), "{at}");
                assert!(close(&after.sim.plane.q2, &e(j, 1.0), 1e-9), "{at}");
                assert_eq!(after.sim.slice.frame, want, "{at}");
                rig.undo();
                let back = rig.snap();
                assert_eq!(back.sim.slice.frame, start, "one Ctrl+Z, the frame: {at}");
                assert_eq!(
                    back.sim.plane, before.sim.plane,
                    "one Ctrl+Z, the plane: {at}"
                );
            }
        }
    }
    // The cube then reads it: D left the plane, H and V on the steered pair.
    let mut rig = physical_rig(CompassView::Jacobi, identity_j(), frame(Q3, Q3, Q3));
    click_cell(&mut rig, Q2, Q3);
    rig.app.manifold.compass.view = CompassView::Cube;
    rig.settle();
    check_on_edges(&mut rig, "H z_q2", "D z_α", "V z_q3");
    // Control: the same plane by a basis edit alone, the steer before this rule, leaves D on z_q3 in the contract.
    let mut rig = physical_rig(CompassView::Jacobi, identity_j(), frame(Q3, Q3, Q3));
    rig.set(SimField::Basis {
        q1: e(Q2, 1.0),
        q2: e(Q3, 1.0),
    });
    let left = rig.snap().sim.slice.frame;
    rejects("a physical steer that leaves D in the plane", || {
        assert_eq!(left, frame(ALPHA, ALPHA, ALPHA))
    });
}

/// Every cell of both views, the property G1 and G4 give: no dimension twice, whatever plane a steer lands on.
#[test]
fn qa_cube_no_physical_steer_leaves_d_or_a_target_on_the_plane_it_reads() {
    let distinct = |after: &Snapshot, at: &str| {
        let f = after.sim.slice.frame;
        let v = View::framed(&after.sim.plane, Some(&f));
        for (name, axis) in [("D", f.normal), ("X", f.tilt_x), ("Y", f.tilt_y)] {
            let axis = axis as usize;
            assert!(
                axis != v.a && axis != v.b,
                "{name} = {axis} on the plane read ({}, {}): {at}",
                v.a,
                v.b
            );
        }
    };
    for view in [CompassView::Jacobi, CompassView::Shape] {
        // The planes on two axes, where the rule's answer is known.
        let start = frame(Q3, Q3, Q3);
        let mut rig = physical_rig(view, identity_j(), start);
        for i in 0..8 {
            for j in i + 1..8 {
                let at = format!("identity, {view:?} ({i}, {j})");
                let before = rig.snap();
                click_cell(&mut rig, i, j);
                let after = rig.snap();
                let d = slice_axis_of(i, j);
                assert_eq!(after.sim.slice.frame, frame(d, d, d), "{at}");
                distinct(&after, &at);
                rig.undo();
                assert_eq!(rig.snap().sim.slice.frame, start, "{at}");
                assert_eq!(rig.snap().sim.plane, before.sim.plane, "{at}");
            }
        }
        // Planes off every axis, read by the reader.
        for start in [frame(Q3, Q3, Q3), frame(ALPHA, Q0, MU1)] {
            let mut rig = physical_rig(view, dense_j(), start);
            for i in 0..8 {
                for j in i + 1..8 {
                    let at = format!("dense, {view:?} ({i}, {j}) from {start:?}");
                    let before = rig.snap();
                    click_cell(&mut rig, i, j);
                    let after = rig.snap();
                    assert_ne!(after.sim.plane, before.sim.plane, "it steered: {at}");
                    check_one(
                        1,
                        i64::from(after.history.undo_depth) - i64::from(before.history.undo_depth),
                    );
                    distinct(&after, &at);
                    rig.undo();
                    assert_eq!(rig.snap().sim.slice.frame, start, "{at}");
                }
            }
        }
    }
    // Control: the property fires on a D left in the plane.
    let mut rig = physical_rig(CompassView::Jacobi, identity_j(), frame(Q3, Q3, Q3));
    rig.set(SimField::Basis {
        q1: e(Q2, 1.0),
        q2: e(Q3, 1.0),
    });
    let left = rig.snap();
    rejects("a D on the plane", || distinct(&left, "control"));
}

/// §G4: chart changes turn about the pin; the frame moving in the same edit changes neither that nor the one entry.
#[test]
fn qa_cube_a_locked_physical_steer_onto_d_turns_about_the_pin_in_one_entry() {
    let mut rig = physical_rig(CompassView::Jacobi, identity_j(), frame(Q3, Q3, Q3));
    let p = rig.snap().sim.plane;
    let (s, t) = (0.75, 0.25);
    let anchor = add(&add(&p.z0, 2.0 * s - 1.0, &p.q1), 2.0 * t - 1.0, &p.q2);
    rig.set(SimField::Lock(Lock {
        locked: true,
        z_locked: anchor,
    }));
    let before = rig.snap();
    click_cell(&mut rig, Q2, Q3);
    let after = rig.snap();
    check_one(
        1,
        i64::from(after.history.undo_depth) - i64::from(before.history.undo_depth),
    );
    assert_eq!(after.sim.slice.frame, frame(ALPHA, ALPHA, ALPHA));
    let want = add(
        &add(&p.z0, 2.0 * s - 1.0, &add(&p.q1, -1.0, &e(Q2, 1.0))),
        2.0 * t - 1.0,
        &add(&p.q2, -1.0, &e(Q3, 1.0)),
    );
    assert!(
        close(&after.sim.plane.z0, &want, 1e-9),
        "z₀ {:?}, want {want:?}",
        after.sim.plane.z0
    );
    rejects("the z₀ a free steer keeps", || {
        assert!(close(&after.sim.plane.z0, &p.z0, 1e-9))
    });
    rig.undo();
    let back = rig.snap();
    assert_eq!(back.sim.plane, before.sim.plane, "one Ctrl+Z, the plane");
    assert_eq!(back.sim.slice.frame, frame(Q3, Q3, Q3), "and the frame");
}

/// A refused cell edits nothing: not the basis, and not the frame the plane would have moved.
#[test]
fn qa_cube_a_refused_physical_steer_keeps_the_frame() {
    // Rank 7 with the shell's normal e_q3, so √(1 − n_i² − n_j²) = 0 on any cell holding z_q3: refused.
    let mut a: Matrix = *identity_j();
    a[Q3] = [0.0; 8];
    let mut rig = physical_rig(CompassView::Jacobi, identity_j(), frame(Q3, Q3, Q3));
    rig.app.manifold.compass.decoder = Decoder::Linear {
        a: Box::leak(Box::new(a)),
        shell: true,
    };
    rig.idle(2);
    let (before, n0) = (rig.snap(), rig.applied());
    click_cell(&mut rig, Q2, Q3);
    let after = rig.snap();
    assert_eq!(rig.applied(), n0, "no SetField");
    assert_eq!(after.history.undo_depth, before.history.undo_depth);
    assert_eq!(after.sim.slice.frame, before.sim.slice.frame);
    assert_eq!(after.sim.plane, before.sim.plane);
    // Control: a cell off the normal on the same J does steer, so the click was on the grid.
    click_cell(&mut rig, Q0, Q1);
    assert!(close(&rig.snap().sim.plane.q1, &e(Q0, 1.0), 1e-9));
}
