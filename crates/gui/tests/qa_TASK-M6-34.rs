//! QA tests for TASK-M6-34, written from the requirements and R-452, not from the implementation:
//!
//! - REQ-GUI-194: the compass's view switcher, Cube (the default), Plücker, Jacobi and Shape, independent of its Tilt
//!   and Slice modes; a drag in a grid view does nothing; the view is GUI state, not undoable (R-452 C1).
//! - REQ-GUI-195: the Plücker view, `p_ij = q₁ᵢ q₂ⱼ − q₁ⱼ q₂ᵢ` (i < j) of the decoder's `q₁`, `q₂` orthonormalised:
//!   `Σ p² = 1`, unchanged by a rotation within the plane, an axis plane lighting one cell at ±1; the dominant cell
//!   outlined above 0.02 and named in the readout with `Σ p²`; the names on both edges, no number in a cell, the value
//!   on hover (C2, C3).
//! - REQ-GUI-196: the position readout, each bar `z₀ₖ` over the half-range clamped to ±1, the slice axis highlighted;
//!   a readout only.
//! - REQ-GUI-197: a click on cell (i, j) sets the plane to `z_i × z_j`, the tilts and γ 0, the zoom and `z₀` kept, one
//!   SetField, logged, undone by one Ctrl+Z; locked, about the pin (C5).
//! - REQ-GUI-198 and the re-checked REQ-GUI-183: the Compass scope's seven siblings, Enter on a view button, the cell
//!   cursor over the upper triangle, Space steering, Esc back (C6).
//!
//! The Plücker coordinates are computed here two ways apart from the code: by Gram–Schmidt, and by the raw Plücker
//! vector of `(q₁, q₂)` over its Gram area `√(|q₁|²|q₂|² − (q₁·q₂)²)`, which needs no orthonormalisation. Each check runs
//! its control (R-176) through [`rejects`]. Random planes come from a fixed-seed generator, a bounded count of each.
// The file name `qa_TASK-M6-34` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::log::Source;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Lock, Plane};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::{KeyMode, Mode};
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::compass::views::{self, CompassView, GridPlace};
use gui::explore::compass::{self};
use gui::explore::manifold_view::{Touch, View, AXES, Z0_HALF_RANGE};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::MockSide;

type Z = [f64; 8];
type Grid = [[f64; 8]; 8];

const NONE: Modifiers = Modifiers::NONE;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;
/// A key no command binds: pressed only to turn keyboard navigation on (R-460, R-464 M2).
const UNBOUND: Key = Key::F12;
/// REQ-GUI-195's tolerance.
const TOL: f64 = 1e-12;
/// R-452 C3's outline threshold.
const OUTLINE: f64 = 0.02;

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

// --- vectors and this file's own Plücker coordinates ---------------------------------------------------------------

fn dot(a: &Z, b: &Z) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn add(a: &Z, k: f64, b: &Z) -> Z {
    std::array::from_fn(|i| a[i] + k * b[i])
}

fn norm(a: &Z) -> f64 {
    dot(a, a).sqrt()
}

fn e(i: usize, k: f64) -> Z {
    std::array::from_fn(|j| if i == j { k } else { 0.0 })
}

/// The raw Plücker coordinates of `(q₁, q₂)`, not normalised.
fn raw(q1: &Z, q2: &Z) -> Grid {
    let mut g = [[0.0; 8]; 8];
    for i in 0..8 {
        for j in i + 1..8 {
            g[i][j] = q1[i] * q2[j] - q1[j] * q2[i];
        }
    }
    g
}

/// The plane's Plücker coordinates by the Gram area: the raw coordinates over `√(|q₁|²|q₂|² − (q₁·q₂)²)`.
fn by_area(q1: &Z, q2: &Z) -> Grid {
    let area = (dot(q1, q1) * dot(q2, q2) - dot(q1, q2).powi(2)).sqrt();
    raw(q1, q2).map(|row| row.map(|p| p / area))
}

/// The plane's Plücker coordinates by this file's own Gram–Schmidt.
fn by_gram_schmidt(q1: &Z, q2: &Z) -> Grid {
    let e1 = q1.map(|x| x / norm(q1));
    let w = add(q2, -dot(q2, &e1), &e1);
    let e2 = w.map(|x| x / norm(&w));
    raw(&e1, &e2)
}

fn upper() -> Vec<(usize, usize)> {
    (0..8)
        .flat_map(|i| (i + 1..8).map(move |j| (i, j)))
        .collect()
}

fn sum_sq(g: &Grid) -> f64 {
    upper().iter().map(|&(i, j)| g[i][j] * g[i][j]).sum()
}

fn same_grid(got: &Grid, want: &Grid, tol: f64, what: &str) {
    for (i, j) in upper() {
        assert!(
            (got[i][j] - want[i][j]).abs() <= tol,
            "{what}: cell ({i}, {j}) is {}, want {}",
            got[i][j],
            want[i][j]
        );
    }
}

/// The largest |p| over the upper triangle, the first of equals row by row.
fn my_dominant(g: &Grid) -> (usize, usize) {
    let mut best = (0, 1);
    for (i, j) in upper() {
        if g[i][j].abs() > g[best.0][best.1].abs() {
            best = (i, j);
        }
    }
    best
}

/// A fixed-seed generator (64-bit LCG), uniform in `[-1, 1)`.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
    }

    fn z(&mut self) -> Z {
        std::array::from_fn(|_| self.next())
    }
}

fn plane(z0: Z, q1: Z, q2: Z) -> Plane {
    Plane { z0, q1, q2 }
}

/// A random plane: random basis vectors, scaled and sheared, so not orthonormal on input.
fn random_plane(rng: &mut Rng) -> Plane {
    let q1 = rng.z().map(|x| x * 3.0);
    let q2 = add(&rng.z().map(|x| x * 0.4), 0.7, &q1);
    plane(rng.z(), q1, q2)
}

// === REQ-GUI-195: the Plücker coordinates ===========================================================================

#[test]
fn qa_compass_plucker_sum_of_squares_is_one_over_random_non_orthonormal_planes() {
    let mut rng = Rng(0x5eed_0034);
    let mut worst: f64 = 0.0;
    for _ in 0..2000 {
        let p = random_plane(&mut rng);
        assert!(
            dot(&p.q1, &p.q2).abs() > 1e-3 || (norm(&p.q1) - 1.0).abs() > 1e-3,
            "the input is orthonormal, so the test proves nothing"
        );
        let grid = views::plucker(&p);
        worst = worst.max((sum_sq(&grid) - 1.0).abs());
        same_grid(
            &grid,
            &by_area(&p.q1, &p.q2),
            1e-12,
            "against the Gram area",
        );
        same_grid(
            &grid,
            &by_gram_schmidt(&p.q1, &p.q2),
            1e-12,
            "against Gram–Schmidt",
        );
    }
    assert!(worst <= TOL, "Σ p² is {worst} from 1");
    // Control: the raw, un-orthonormalised basis's coordinates.
    let p = random_plane(&mut Rng(7));
    rejects("a grid from an un-orthonormalised basis", || {
        let g = raw(&p.q1, &p.q2);
        assert!((sum_sq(&g) - 1.0).abs() <= TOL, "Σ p² {}", sum_sq(&g));
    });
    rejects(
        "a grid from an un-orthonormalised basis, cell by cell",
        || same_grid(&raw(&p.q1, &p.q2), &by_area(&p.q1, &p.q2), 1e-12, "raw"),
    );
}

#[test]
fn qa_compass_plucker_is_the_basis_not_the_tilt_angles() {
    // A plane out of every preset's frame: the angles the panel reads it by do not rebuild it. The grid must be the
    // basis's, which this file computes apart.
    let mut rng = Rng(0xa11);
    let mut differs = 0;
    for _ in 0..200 {
        let p = random_plane(&mut rng);
        let grid = views::plucker(&p);
        same_grid(&grid, &by_area(&p.q1, &p.q2), 1e-12, "the basis's grid");
        let (r1, r2) = View::of(&p).basis();
        let from_angles = by_area(&r1, &r2);
        if upper()
            .iter()
            .any(|&(i, j)| (from_angles[i][j] - grid[i][j]).abs() > 1e-6)
        {
            differs += 1;
        }
    }
    assert!(
        differs > 100,
        "the angles rebuild the plane: the control is empty"
    );
    // Control: a grid from the tilt angles fails the basis's check.
    let p = random_plane(&mut Rng(0xa11));
    rejects("a grid from the tilt angles", || {
        let (r1, r2) = View::of(&p).basis();
        same_grid(&by_area(&r1, &r2), &by_area(&p.q1, &p.q2), 1e-12, "angles")
    });
}

#[test]
fn qa_compass_plucker_an_axis_plane_lights_exactly_its_cell_at_plus_or_minus_one() {
    for (i, j) in upper() {
        for (sigma, swap) in [(1.0, false), (0.37, false), (2.5, true)] {
            let (q1, q2) = if swap {
                (e(j, sigma), e(i, sigma))
            } else {
                (e(i, sigma), e(j, sigma))
            };
            let grid = views::plucker(&plane([0.1; 8], q1, q2));
            let want = if swap { -1.0 } else { 1.0 };
            for (a, b) in upper() {
                let w = if (a, b) == (i, j) { want } else { 0.0 };
                assert!(
                    (grid[a][b] - w).abs() <= TOL,
                    "the plane z_{i} × z_{j} (σ {sigma}, swapped {swap}): cell ({a}, {b}) is {}, want {w}",
                    grid[a][b]
                );
            }
        }
    }
    // Control: a plane tilted off the axis plane lights a second cell.
    rejects("a tilted plane checked as an axis plane", || {
        let q1 = add(&e(0, 1.0), 0.2, &e(5, 1.0));
        let grid = views::plucker(&plane([0.0; 8], q1, e(1, 1.0)));
        for (a, b) in upper() {
            let w = if (a, b) == (0, 1) { 1.0 } else { 0.0 };
            assert!((grid[a][b] - w).abs() <= TOL);
        }
    });
}

#[test]
fn qa_compass_plucker_a_rotation_within_the_plane_leaves_every_cell() {
    let mut rng = Rng(0x0_2047);
    for _ in 0..300 {
        let p = random_plane(&mut rng);
        let g0 = views::plucker(&p);
        for k in 0..12 {
            let t = f64::from(k) * 0.53 - 3.0;
            let (c, s) = (t.cos(), t.sin());
            let q1 = add(&p.q1.map(|x| x * c), s, &p.q2);
            let q2 = add(&p.q2.map(|x| x * c), -s, &p.q1);
            same_grid(
                &views::plucker(&plane(p.z0, q1, q2)),
                &g0,
                TOL,
                &format!("rotated by {t} rad"),
            );
        }
    }
    // Control: a reflection within the plane (q₁, q₂ swapped) flips every sign.
    let p = random_plane(&mut Rng(3));
    rejects("a reflection checked as a rotation", || {
        same_grid(
            &views::plucker(&plane(p.z0, p.q2, p.q1)),
            &views::plucker(&p),
            TOL,
            "swapped",
        )
    });
}

#[test]
fn qa_compass_plucker_dominant_cell_and_readout_name_the_largest_p() {
    let mut rng = Rng(0xd0);
    for _ in 0..300 {
        let p = random_plane(&mut rng);
        let grid = views::plucker(&p);
        let (i, j) = my_dominant(&grid);
        assert_eq!(views::dominant(&grid), Some((i, j)), "the dominant cell");
        let text = views::readout(&grid);
        let pair = format!("{} × {}", AXES[i], AXES[j]);
        assert!(text.contains(&pair), "readout {text:?} lacks {pair}");
        let after = text.split(&pair).nth(1).unwrap();
        let value = number(after.split_whitespace().next().unwrap());
        assert!(
            (value - grid[i][j]).abs() <= 0.0005,
            "readout {text:?}: the value is not {:.3}",
            grid[i][j]
        );
        assert!(
            text.contains("Σ p²") && text.contains("1.000"),
            "readout {text:?}"
        );
        let d = views::plucker_drawing(square(), &p, None);
        assert_eq!(d.outline, Some((i, j)), "the outlined cell");
    }
    // The threshold, on hand-filled grids: 0.03 outlined, 0.015 not, and no pair in the readout then.
    let mut g = [[0.0; 8]; 8];
    g[2][6] = -0.03;
    g[1][3] = 0.01;
    assert_eq!(views::dominant(&g), Some((2, 6)), "|p| 0.03 > 0.02");
    g[2][6] = -0.015;
    assert_eq!(views::dominant(&g), None, "|p| 0.015 ≤ 0.02 is outlined");
    let text = views::readout(&g);
    assert!(
        !AXES.iter().any(|a| text.contains(a)),
        "below 0.02 the readout names a pair: {text:?}"
    );
    // At C3's threshold itself: just above is outlined, just below is not.
    g[2][6] = OUTLINE * 1.001;
    assert_eq!(views::dominant(&g), Some((2, 6)), "|p| just above 0.02");
    g[2][6] = OUTLINE * 0.999;
    assert_eq!(views::dominant(&g), None, "|p| just below 0.02 is outlined");
    // Control: the second largest |p| is not the dominant cell.
    let p = random_plane(&mut Rng(11));
    let grid = views::plucker(&p);
    let first = my_dominant(&grid);
    let mut second = if first == (0, 1) { (0, 2) } else { (0, 1) };
    for (i, j) in upper() {
        if (i, j) != first && grid[i][j].abs() > grid[second.0][second.1].abs() {
            second = (i, j);
        }
    }
    rejects("the second largest |p| as the dominant cell", || {
        assert_eq!(views::dominant(&grid), Some(second))
    });
}

/// A number as printed, with an ASCII hyphen or a Unicode minus.
fn number(s: &str) -> f64 {
    s.replace('−', "-")
        .trim_end_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .unwrap_or_else(|_| panic!("no number in {s:?}"))
}

// === REQ-GUI-196: the position readout ==============================================================================

#[test]
fn qa_compass_position_gauge_bars_are_z0_over_the_half_range_clamped() {
    let h = Z0_HALF_RANGE;
    let z0: Z = [0.5 * h, -0.25 * h, h, -h, 1.7 * h, -3.0 * h, 0.0, 0.999 * h];
    let want: Z = [0.5, -0.25, 1.0, -1.0, 1.0, -1.0, 0.0, 0.999];
    let bars = views::gauge(&z0);
    for k in 0..8 {
        assert!(
            (bars[k] - want[k]).abs() <= TOL,
            "bar {k}: {} for z₀ {}, want {}",
            bars[k],
            z0[k],
            want[k]
        );
    }
    // Control: an unclamped bar.
    rejects("an unclamped bar", || {
        assert!((bars[4] - 1.7).abs() <= TOL);
    });
}

#[test]
fn qa_compass_position_gauge_slice_axis_follows_the_preset_and_a_slice_edit_keeps_the_grid() {
    // Two presets of different slice axes: the default chart z_α × z_β, slice axis z_q3 (§G2), and one with z_q3
    // shown, whose slice axis is another.
    let a = plane([0.0; 8], e(0, 1.0), e(1, 1.0));
    let b = plane([0.0; 8], e(5, 1.0), e(6, 1.0));
    let da = views::plucker_drawing(square(), &a, None);
    let db = views::plucker_drawing(square(), &b, None);
    assert_eq!(
        da.slice_axis,
        View::of(&a).d,
        "the default preset's slice axis"
    );
    assert_eq!(da.slice_axis, 5, "the default chart's slice axis is z_q3");
    assert_eq!(db.slice_axis, View::of(&b).d);
    assert_ne!(da.slice_axis, db.slice_axis, "the highlight did not move");
    // A slice edit: z₀ along the slice axis, the plane's heading kept.
    let mut rng = Rng(0x51);
    for _ in 0..100 {
        let p = random_plane(&mut rng);
        let d = View::of(&p).d;
        let mut moved = p.clone();
        moved.z0[d] += 0.3 * Z0_HALF_RANGE;
        let before = views::plucker_drawing(square(), &p, None);
        let after = views::plucker_drawing(square(), &moved, None);
        assert_eq!(after.grid, before.grid, "a slice edit changed the grid");
        for k in 0..8 {
            let want = (moved.z0[k] / Z0_HALF_RANGE).clamp(-1.0, 1.0);
            assert!((after.bars[k] - want).abs() <= TOL, "bar {k}");
        }
        let both_clamped = before.bars[d].abs() == 1.0 && after.bars[d].abs() == 1.0;
        assert!(
            both_clamped || after.bars[d] != before.bars[d],
            "the slice axis's bar did not move"
        );
    }
    // Control: a highlight fixed on z_q3 fails on the second preset.
    rejects("a highlight that does not follow the preset", || {
        assert_eq!(db.slice_axis, 5)
    });
}

// --- the rig ---------------------------------------------------------------------------------------------------------

fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

/// The compass's square.
fn square() -> Rect {
    compass::square(layout().compass)
}

/// The Plücker grid's cell (i, j) on screen.
fn cell_at(i: usize, j: usize) -> Pos2 {
    GridPlace::fit(square()).cell_rect(i, j).center()
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

    /// The SetFields applied so far, less the hover's no-history ones (gui_state_contract §2; R-101).
    fn applied(&self) -> usize {
        self.app
            .console()
            .iter()
            .filter(|e| {
                e.source == Source::Contract && !e.message.starts_with("SetField Hover.at ")
            })
            .count()
    }

    fn undo_depth(&self) -> u32 {
        self.app.snapshot().history.undo_depth
    }

    fn view(&self) -> CompassView {
        self.app.manifold.compass.view
    }

    fn mode(&self) -> Touch {
        self.app.manifold.compass.mode
    }

    /// The focus: its scopes' labels, outermost first, and its mode.
    fn at(&self) -> (Vec<String>, KeyMode) {
        let labels = self
            .app
            .keyboard
            .tree(Mode::Explore)
            .labels(&self.app.view.focus.path)
            .into_iter()
            .map(str::to_owned)
            .collect();
        (labels, self.app.view.focus.mode)
    }

    fn shapes(&mut self) -> Vec<egui::Shape> {
        let out = self.frame(Vec::new());
        let mut flat = Vec::new();
        for clipped in &out.shapes {
            flatten(&clipped.shape, &mut flat);
        }
        flat
    }

    fn pointer_away(&mut self) {
        let away = layout().figure.center();
        let _ = self.frame(vec![Event::PointerMoved(away)]);
        self.idle(1);
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
        self.idle(1);
        self.pointer_away();
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
        self.pointer_away();
    }

    /// Clicks the button labelled `label` beside the compass.
    fn click_button(&mut self, label: &str) {
        let shapes = self.shapes();
        let at = button_at(&shapes, label);
        self.click(at);
    }

    /// Sets the plane through the contract, undoably, as a user's edit is.
    fn set_plane(&mut self, p: &Plane) {
        for edit in [SimField::Z0(p.z0), SimField::Basis { q1: p.q1, q2: p.q2 }] {
            self.app.set_field(SetField {
                edit: Edit::Sim(edit),
                no_history: false,
            });
        }
        self.idle(4);
    }

    /// Tab to the Compass and Enter onto the compass, navigation mode.
    fn onto_compass(&mut self) {
        self.key(UNBOUND, NONE);
        for _ in 0..5 {
            self.key(Key::Tab, NONE);
        }
        check_at(&self.at(), &["Compass"], NAV, "Tab onto the Compass");
        self.key(Key::Enter, NONE);
        check_at(
            &self.at(),
            &["Compass", "compass"],
            NAV,
            "Enter on the Compass",
        );
    }
}

fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
}

/// The centre of the text `label` drawn inside the compass.
fn button_at(shapes: &[egui::Shape], label: &str) -> Pos2 {
    let compass = layout().compass;
    shapes
        .iter()
        .find_map(|s| match s {
            egui::Shape::Text(t) if t.galley.text() == label => {
                Some(s.visual_bounding_rect()).filter(|r| compass.contains_rect(*r))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no {label} button beside the compass"))
        .center()
}

fn check_at(got: &(Vec<String>, KeyMode), want: &[&str], mode: KeyMode, what: &str) {
    let ok = got.0.len() == want.len()
        && got
            .0
            .iter()
            .zip(want)
            .all(|(g, w)| g.to_lowercase() == w.to_lowercase())
        && got.1 == mode;
    assert!(
        ok,
        "{what}: the focus is {got:?}, want {want:?} in {mode:?}"
    );
}

/// The texts drawn this frame inside the compass.
fn compass_texts(shapes: &[egui::Shape]) -> Vec<(String, Rect)> {
    let compass = layout().compass;
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => {
                let r = s.visual_bounding_rect();
                compass
                    .intersects(r)
                    .then(|| (t.galley.text().to_owned(), r))
            }
            _ => None,
        })
        .collect()
}

// === REQ-GUI-194: the view switcher =================================================================================

#[test]
fn qa_mock_compass_views_open_on_cube_and_views_and_modes_are_independent() {
    let mut rig = mock();
    assert_eq!(rig.view(), CompassView::Cube, "the compass opens on Cube");
    let labels: Vec<String> = compass_texts(&rig.shapes())
        .into_iter()
        .map(|t| t.0)
        .collect();
    for l in ["Cube", "Plücker", "Jacobi", "Shape", "Tilt", "Slice"] {
        assert!(
            labels.iter().any(|t| t == l),
            "no {l} button beside the compass: {labels:?}"
        );
    }
    let (n0, d0) = (rig.applied(), rig.undo_depth());
    let mut seen = Vec::new();
    for mode in ["Slice", "Tilt"] {
        rig.click_button(mode);
        let m = rig.mode();
        for (label, view) in [
            ("Plücker", CompassView::Plucker),
            ("Jacobi", CompassView::Jacobi),
            ("Shape", CompassView::Shape),
            ("Cube", CompassView::Cube),
        ] {
            rig.click_button(label);
            assert_eq!(rig.view(), view, "a click on {label}");
            assert_eq!(rig.mode(), m, "a click on {label} changed the mode");
            seen.push((rig.view(), rig.mode()));
        }
    }
    // Tilt and Slice in a grid view leave it.
    rig.click_button("Plücker");
    for (mode, want) in [("Slice", Touch::Slice), ("Tilt", Touch::Tilt)] {
        rig.click_button(mode);
        assert_eq!(rig.mode(), want, "a click on {mode}");
        assert_eq!(
            rig.view(),
            CompassView::Plucker,
            "a click on {mode} changed the view"
        );
    }
    // Jacobi and Shape say they wait on physics review.
    rig.click_button("Jacobi");
    let texts = compass_texts(&rig.shapes());
    assert!(
        texts.iter().any(|t| t.0.contains("RQ-341")),
        "the Jacobi view does not say it waits on physics review"
    );
    assert_eq!(
        rig.applied(),
        n0,
        "switching views or modes emitted a SetField"
    );
    assert_eq!(
        rig.undo_depth(),
        d0,
        "switching views or modes added to undo"
    );
    // Controls: a view switch that changes the mode, a mode switch that changes the view.
    rejects("a view switch that changes the mode", || {
        assert_eq!(seen[0].1, Touch::Tilt, "Plücker in Slice mode read as Tilt")
    });
    rejects("a mode switch that changes the view", || {
        assert_eq!(rig.view(), CompassView::Cube)
    });
}

#[test]
fn qa_mock_compass_views_a_slice_slider_in_plucker_sets_slice_and_keeps_the_view() {
    let mut rig = mock();
    rig.click_button("Plücker");
    assert_eq!(rig.mode(), Touch::Tilt);
    // Touch the slice slider z_q1 from the keyboard, as REQ-GUI-181's test does.
    let tree = rig.app.keyboard.tree(Mode::Explore);
    let mut all: Vec<&'static str> = tree.children(None);
    let mut i = 0;
    while i < all.len() {
        let kids = tree.children(Some(all[i]));
        all.extend(kids);
        i += 1;
    }
    let id = all
        .into_iter()
        .find(|id| tree.get(id).unwrap().label == "z_q1")
        .expect("the slice slider z_q1");
    rig.app.view.focus.path = tree.path_to(id);
    rig.app.view.focus.mode = NAV;
    rig.key(UNBOUND, NONE);
    rig.key(Key::Enter, NONE);
    rig.key(Key::ArrowUp, NONE);
    assert_eq!(
        rig.mode(),
        Touch::Slice,
        "touching a slice slider sets Slice"
    );
    assert_eq!(
        rig.view(),
        CompassView::Plucker,
        "touching a slice slider left Plücker"
    );
    rejects("a slider that sends the compass to Cube", || {
        assert_eq!(rig.view(), CompassView::Cube)
    });
}

#[test]
fn qa_mock_compass_views_a_drag_in_a_grid_view_does_nothing() {
    let sq = square();
    let (from, to) = (
        sq.center() + egui::vec2(-0.2 * sq.width(), 0.1 * sq.height()),
        sq.center() + egui::vec2(0.3 * sq.width(), -0.25 * sq.height()),
    );
    let mut deltas = Vec::new();
    for (view, label) in [
        (CompassView::Plucker, "Plücker"),
        (CompassView::Jacobi, "Jacobi"),
        (CompassView::Shape, "Shape"),
        (CompassView::Cube, "Cube"),
    ] {
        for mode in ["Tilt", "Slice"] {
            let mut rig = mock();
            rig.click_button(mode);
            rig.click_button(label);
            assert_eq!(rig.view(), view);
            let (n0, d0, p0) = (rig.applied(), rig.undo_depth(), rig.snap().sim.plane);
            rig.drag(from, to, 8);
            deltas.push((
                view,
                rig.applied() - n0,
                rig.undo_depth() - d0,
                rig.snap().sim.plane != p0,
            ));
        }
    }
    for &(view, n, d, moved) in &deltas {
        if view != CompassView::Cube {
            assert!(
                n == 0 && d == 0 && !moved,
                "a drag in the {view:?} view: {n} SetFields, undo +{d}, plane moved {moved}"
            );
        }
    }
    // Control: the same drag on the Cube edits the plane, so the measure can fire.
    let cube: Vec<_> = deltas.iter().filter(|d| d.0 == CompassView::Cube).collect();
    assert!(
        cube.iter().all(|d| d.1 > 0 && d.2 == 1 && d.3),
        "the Cube drag did not edit: {cube:?}"
    );
    rejects("the Cube drag read as a grid view's", || {
        let d = cube[0];
        assert!(d.1 == 0 && d.2 == 0 && !d.3)
    });
}

// === REQ-GUI-195 on screen: the names, no numbers in cells, the hover ===============================================

#[test]
fn qa_mock_compass_plucker_names_on_both_edges_no_numbers_in_cells_and_hover() {
    let mut rig = mock();
    let p = random_plane(&mut Rng(0x42));
    rig.set_plane(&p);
    rig.click_button("Plücker");
    let shapes = rig.shapes();
    let texts = compass_texts(&shapes);
    let place = GridPlace::fit(square());
    let grid_rect = place.rect();
    for name in AXES {
        let left = texts.iter().any(|(t, r)| {
            t == name && r.max.x <= grid_rect.min.x + 0.5 && r.center().y > grid_rect.min.y
        });
        let top = texts
            .iter()
            .any(|(t, r)| t == name && r.max.y <= grid_rect.min.y + 0.5);
        assert!(
            left && top,
            "{name}: on the left edge {left}, on the top edge {top}"
        );
    }
    for (i, j) in upper() {
        let cell = place.cell_rect(i, j);
        let inside: Vec<_> = texts
            .iter()
            .filter(|(_, r)| cell.contains(r.center()))
            .collect();
        assert!(inside.is_empty(), "text in cell ({i}, {j}): {inside:?}");
    }
    // Hover: the names and the value.
    let grid = by_area(&p.q1, &p.q2);
    let mut hovers = Vec::new();
    for (i, j) in [(0, 1), (2, 5), (6, 7), my_dominant(&grid)] {
        let at = cell_at(i, j);
        let mut seen = Vec::new();
        let _ = rig.frame(vec![Event::PointerMoved(at)]);
        for _ in 0..10 {
            let out = rig.frame(Vec::new());
            let mut flat = Vec::new();
            for c in &out.shapes {
                flatten(&c.shape, &mut flat);
            }
            for s in flat {
                if let egui::Shape::Text(t) = s {
                    seen.push(t.galley.text().to_owned());
                }
            }
        }
        hover_shows(&seen, i, j, &grid);
        hovers.push(seen);
        rig.pointer_away();
    }
    // Control: hovering (0, 1) read as showing (6, 7).
    rejects("another cell's names and value on hover", || {
        hover_shows(&hovers[0], 6, 7, &grid)
    });
}

/// The texts `seen` while hovering include one naming cell (i, j) with its value.
fn hover_shows(seen: &[String], i: usize, j: usize, grid: &Grid) {
    let pair = format!("{} × {}", AXES[i], AXES[j]);
    let hit = seen
        .iter()
        .find(|t| t.contains(&pair) && !t.starts_with("strongest"))
        .unwrap_or_else(|| panic!("hovering ({i}, {j}): no {pair} shown"));
    let v = number(hit.split_whitespace().last().unwrap());
    assert!(
        (v - grid[i][j]).abs() <= 0.0005,
        "hover {hit:?}, want {:.3}",
        grid[i][j]
    );
}

// === REQ-GUI-197: the steer ==========================================================================================

/// The plane's grid lights exactly cell (i, j) at ±1.
fn lights(p: &Plane, i: usize, j: usize, what: &str) {
    let g = by_area(&p.q1, &p.q2);
    for (a, b) in upper() {
        let ok = if (a, b) == (i, j) {
            (g[a][b].abs() - 1.0).abs() <= 1e-12
        } else {
            g[a][b].abs() <= 1e-12
        };
        assert!(ok, "{what}: cell ({a}, {b}) is {}", g[a][b]);
    }
}

fn check_steered(before: &Snapshot, after: &Snapshot, i: usize, j: usize, what: &str) {
    let p = &after.sim.plane;
    lights(p, i, j, what);
    let v = View::of(p);
    assert_eq!((v.a, v.b), (i, j), "{what}: the Manifold view's chart");
    assert!(
        v.tau1 == 0.0 && v.tau2 == 0.0 && v.gamma == 0.0,
        "{what}: the tilts {} {} {}",
        v.tau1,
        v.tau2,
        v.gamma
    );
    let s0 = View::of(&before.sim.plane).scale;
    assert!(
        (v.scale - s0).abs() <= 1e-12 * s0,
        "{what}: the zoom {} is not {s0}",
        v.scale
    );
}

#[test]
fn qa_mock_compass_steer_latent_every_cell_steers_once_and_undoes_exactly() {
    let mut rig = mock();
    let mut rng = Rng(0x57ee);
    let mut start = random_plane(&mut rng);
    start.z0 = start.z0.map(|x| x * 0.5 * Z0_HALF_RANGE);
    rig.set_plane(&start);
    rig.click_button("Plücker");
    let mut logged = Vec::new();
    for (i, j) in upper() {
        let before = rig.snap();
        let (n0, d0, rows0) = (rig.applied(), rig.undo_depth(), rig.app.console().len());
        rig.click(cell_at(i, j));
        let after = rig.snap();
        let what = format!("a click on ({i}, {j})");
        assert_eq!(rig.applied() - n0, 1, "{what}: SetFields");
        assert_eq!(rig.undo_depth(), d0 + 1, "{what}: undo depth");
        assert!(rig.app.console().len() > rows0, "{what}: nothing logged");
        logged.push(
            rig.app.console()[rows0..]
                .iter()
                .any(|e| e.source == Source::Contract),
        );
        check_steered(&before, &after, i, j, &what);
        assert_eq!(after.sim.plane.z0, before.sim.plane.z0, "{what}: z₀ moved");
        assert_eq!(rig.view(), CompassView::Plucker, "{what}: the view changed");
        rig.key(Key::Z, Modifiers::CTRL);
        assert_eq!(
            rig.snap().sim.plane,
            before.sim.plane,
            "{what}: one Ctrl+Z did not restore it"
        );
        assert_eq!(rig.undo_depth(), d0, "{what}: undo depth after Ctrl+Z");
    }
    assert!(
        logged.iter().all(|l| *l),
        "a steer not logged by the contract"
    );
    // The diagonal and the lower triangle steer nothing.
    for (i, j) in [(0, 0), (3, 3), (7, 7), (1, 0), (6, 2)] {
        let (n0, p0) = (rig.applied(), rig.snap().sim.plane);
        rig.click(cell_at(i, j));
        assert_eq!(
            rig.applied(),
            n0,
            "a click on ({i}, {j}) emitted a SetField"
        );
        assert_eq!(
            rig.snap().sim.plane,
            p0,
            "a click on ({i}, {j}) moved the plane"
        );
    }
    // Controls: a steer that leaves a tilt, two SetFields, a diagonal click that steers.
    let before = rig.snap();
    let mut tilted = before.clone();
    tilted.sim.plane = plane(
        before.sim.plane.z0,
        add(&e(2, 1.0), 0.1, &e(5, 1.0)),
        e(4, 1.0),
    );
    rejects("a steer that leaves a tilt", || {
        check_steered(&before, &tilted, 2, 4, "tilted")
    });
    // Two clicks without an undo between are two SetFields, which the one-SetField check rejects.
    let n0 = rig.applied();
    rig.click(cell_at(1, 4));
    rig.click(cell_at(1, 4));
    let two = rig.applied() - n0;
    rejects("a steer of two SetFields", || {
        assert_eq!(two, 1, "SetFields")
    });
    rejects("a diagonal click that steers", || {
        lights(&plane([0.0; 8], e(3, 1.0), e(3, 1.0)), 3, 3, "diagonal")
    });
}

#[test]
fn qa_mock_compass_steer_latent_locked_keeps_the_anchor() {
    let mut rig = mock();
    // Lock with K over the figure, off its centre, as a user does.
    let f = layout().figure;
    let at = f.min + egui::vec2(f.width() * 0.3, f.height() * 0.6);
    let _ = rig.frame(vec![Event::PointerMoved(at)]);
    rig.key(Key::K, NONE);
    let lock0: Lock = rig.snap().sim.lock;
    assert!(lock0.locked, "K did not lock");
    rig.pointer_away();
    rig.click_button("Plücker");
    on_plane(&rig.snap().sim.plane, &lock0.z_locked, "before the steer");
    let mut off_pin = Vec::new();
    for (i, j) in [(2, 3), (0, 7), (4, 6)] {
        let before = rig.snap();
        let n0 = rig.applied();
        rig.click(cell_at(i, j));
        let after = rig.snap();
        let what = format!("locked, a click on ({i}, {j})");
        assert_eq!(rig.applied() - n0, 1, "{what}: SetFields");
        assert_eq!(after.sim.lock, lock0, "{what}: the anchor moved");
        lights(&after.sim.plane, i, j, &what);
        // The plane turns about the pin: the anchor stays on it.
        on_plane(&after.sim.plane, &lock0.z_locked, &what);
        // The control: the same plane moved off the pin, across itself.
        let k = (0..8).find(|&k| k != i && k != j).unwrap();
        off_pin.push(plane(
            add(&after.sim.plane.z0, 0.3, &e(k, 1.0)),
            after.sim.plane.q1,
            after.sim.plane.q2,
        ));
        let _ = before;
    }
    // Control: the steered plane moved off the pin fails the check.
    rejects("a locked steer that leaves the pin", || {
        on_plane(&off_pin[0], &lock0.z_locked, "off the pin")
    });
}

/// `point` lies on `p`, within 1e-9.
fn on_plane(p: &Plane, point: &Z, what: &str) {
    let mut r = add(point, -1.0, &p.z0);
    let e1 = p.q1.map(|x| x / norm(&p.q1));
    r = add(&r, -dot(&r, &e1), &e1);
    let w = add(&p.q2, -dot(&p.q2, &e1), &e1);
    let e2 = w.map(|x| x / norm(&w));
    r = add(&r, -dot(&r, &e2), &e2);
    assert!(
        norm(&r) <= 1e-9,
        "{what}: the anchor is {} off the plane",
        norm(&r)
    );
}

// === REQ-GUI-198 and REQ-GUI-183: the keys ===========================================================================

#[test]
fn qa_mock_compass_view_keys_seven_siblings_in_order_both_ways() {
    let mut rig = mock();
    rig.onto_compass();
    let mut seen = vec![rig.at()];
    for _ in 0..6 {
        rig.key(Key::ArrowRight, NONE);
        seen.push(rig.at());
    }
    for _ in 0..6 {
        rig.key(Key::ArrowLeft, NONE);
        seen.push(rig.at());
    }
    let order = [
        "compass", "Tilt", "Slice", "Cube", "Plücker", "Jacobi", "Shape",
    ];
    let want: Vec<&str> = order
        .iter()
        .chain(order.iter().rev().skip(1))
        .copied()
        .collect();
    for (k, (got, w)) in seen.iter().zip(&want).enumerate() {
        check_at(got, &["Compass", w], NAV, &format!("step {k}"));
    }
    // Control: another order (the view buttons before Tilt and Slice).
    rejects("the view buttons before Tilt", || {
        check_at(&seen[1], &["Compass", "Cube"], NAV, "→ from the compass")
    });
}

#[test]
fn qa_mock_compass_view_keys_enter_on_a_view_button_switches_in_navigation_mode() {
    let mut rig = mock();
    rig.onto_compass();
    let (n0, d0) = (rig.applied(), rig.undo_depth());
    let mut views_seen = Vec::new();
    // To Shape, then back, Enter on each.
    for _ in 0..6 {
        rig.key(Key::ArrowRight, NONE);
    }
    for (label, view) in [
        ("Shape", CompassView::Shape),
        ("Jacobi", CompassView::Jacobi),
        ("Plücker", CompassView::Plucker),
        ("Cube", CompassView::Cube),
    ] {
        check_at(&rig.at(), &["Compass", label], NAV, "on the button");
        rig.key(Key::Enter, NONE);
        views_seen.push(rig.view());
        assert_eq!(rig.view(), view, "Enter on {label}");
        check_at(
            &rig.at(),
            &["Compass", label],
            NAV,
            &format!("after Enter on {label}"),
        );
        assert_eq!(rig.mode(), Touch::Tilt, "Enter on {label} changed the mode");
        rig.key(Key::ArrowLeft, NONE);
    }
    assert_eq!(
        rig.applied(),
        n0,
        "Enter on a view button emitted a SetField"
    );
    assert_eq!(rig.undo_depth(), d0);
    rejects("Enter on Shape leaving Cube", || {
        assert_eq!(views_seen[0], CompassView::Cube)
    });
}

/// The cell cursor, in the upper triangle.
fn cursor(rig: &Rig) -> (usize, usize) {
    let c = rig
        .app
        .manifold
        .compass
        .cursor
        .expect("no cell cursor in interaction mode");
    assert!(
        c.0 < c.1 && c.1 < 8,
        "the cursor {c:?} is off the upper triangle"
    );
    c
}

/// Where an arrow could take the cursor from `(i, j)`: whether it is at that direction's edge.
fn at_edge((i, j): (usize, usize), key: Key) -> bool {
    match key {
        Key::ArrowLeft => j == i + 1,
        Key::ArrowRight => j == 7,
        Key::ArrowUp => i == 0,
        Key::ArrowDown => i + 1 == j,
        _ => unreachable!(),
    }
}

#[test]
fn qa_mock_compass_view_keys_the_cell_cursor_space_and_esc() {
    let mut rig = mock();
    let mut p = random_plane(&mut Rng(0xc0c));
    p.z0 = [0.0; 8];
    rig.set_plane(&p);
    rig.onto_compass();
    // → to Plücker, Enter, then ← back to the compass.
    for _ in 0..4 {
        rig.key(Key::ArrowRight, NONE);
    }
    check_at(&rig.at(), &["Compass", "Plücker"], NAV, "on Plücker");
    rig.key(Key::Enter, NONE);
    assert_eq!(rig.view(), CompassView::Plucker);
    for _ in 0..4 {
        rig.key(Key::ArrowLeft, NONE);
    }
    assert_eq!(rig.app.view.focus.mode, NAV);
    assert_eq!(rig.app.view.focus.path.len(), 2, "on the compass");
    // Space in navigation mode steers nothing.
    let (n0, p0) = (rig.applied(), rig.snap().sim.plane);
    rig.key(Key::Space, NONE);
    assert_eq!(
        rig.applied(),
        n0,
        "Space in navigation mode emitted a SetField"
    );
    assert_eq!(rig.snap().sim.plane, p0, "Space in navigation mode steered");
    rig.key(Key::Enter, NONE);
    assert_eq!(rig.app.view.focus.mode, INTERACT, "Enter on the compass");
    let dom = my_dominant(&by_area(&p.q1, &p.q2));
    assert_eq!(cursor(&rig), dom, "the cursor starts on the dominant cell");
    // A bounded walk: every arrow keeps it on the upper triangle, stops it at an edge, moves it otherwise, and
    // edits nothing.
    let n1 = rig.applied();
    let keys = [
        Key::ArrowLeft,
        Key::ArrowRight,
        Key::ArrowUp,
        Key::ArrowDown,
    ];
    let mut rng = Rng(0x3a1c);
    let mut visited = vec![dom];
    let mut stopped = 0;
    for _ in 0..60 {
        let k = keys[((rng.next() + 1.0) * 2.0) as usize % 4];
        let from = cursor(&rig);
        rig.key(k, NONE);
        let to = cursor(&rig);
        if at_edge(from, k) {
            assert_eq!(to, from, "{k:?} at the edge from {from:?}");
            stopped += 1;
        } else {
            assert_ne!(to, from, "{k:?} from {from:?} did not move");
            match k {
                Key::ArrowLeft | Key::ArrowRight => assert_eq!(to.0, from.0, "{k:?} left the row"),
                _ => assert_eq!(to.1, from.1, "{k:?} left the column"),
            }
        }
        visited.push(to);
    }
    // Push into each edge from the corners.
    for (k, n) in [(Key::ArrowUp, 8), (Key::ArrowLeft, 8)] {
        for _ in 0..n {
            rig.key(k, NONE);
        }
    }
    assert_eq!(cursor(&rig), (0, 1), "↑ and ← to the corner");
    rig.key(Key::ArrowDown, NONE);
    assert_eq!(cursor(&rig), (0, 1), "↓ onto the diagonal");
    for _ in 0..8 {
        rig.key(Key::ArrowRight, NONE);
    }
    assert_eq!(cursor(&rig), (0, 7));
    for _ in 0..8 {
        rig.key(Key::ArrowDown, NONE);
    }
    assert_eq!(cursor(&rig), (6, 7), "↓ to the diagonal's edge");
    assert!(stopped > 0, "the walk never met an edge");
    assert_eq!(rig.applied(), n1, "the cursor's arrows emitted a SetField");
    // Space steers to it, one SetField, undone by one Ctrl+Z.
    rig.key(Key::ArrowUp, NONE);
    rig.key(Key::ArrowLeft, NONE);
    let target = cursor(&rig);
    assert_eq!(target, (5, 6));
    let (before, n2, d2) = (rig.snap(), rig.applied(), rig.undo_depth());
    rig.key(Key::Space, NONE);
    let after = rig.snap();
    assert_eq!(rig.applied() - n2, 1, "Space: SetFields");
    assert_eq!(rig.undo_depth(), d2 + 1);
    check_steered(&before, &after, target.0, target.1, "Space");
    rig.key(Key::Z, Modifiers::CTRL);
    assert_eq!(
        rig.snap().sim.plane,
        before.sim.plane,
        "one Ctrl+Z after Space"
    );
    // Controls: a cursor on the diagonal or the lower triangle.
    for bad in [(3, 3), (5, 2)] {
        let saved = rig.app.manifold.compass.cursor;
        rig.app.manifold.compass.cursor = Some(bad);
        rejects("a cursor off the upper triangle", || {
            let _ = cursor(&rig);
        });
        rig.app.manifold.compass.cursor = saved;
    }
    // Esc returns to navigation mode on the compass.
    rig.key(Key::Escape, NONE);
    assert_eq!(rig.app.view.focus.mode, NAV, "Esc");
    assert_eq!(rig.app.view.focus.path.len(), 2, "Esc left the compass");
    rejects("a Space in navigation mode that steers", || {
        assert_eq!(after.sim.plane, before.sim.plane)
    });
}

#[test]
fn qa_mock_compass_view_keys_in_cube_the_arrows_tilt_and_the_first_three_keep_their_keys() {
    // REQ-GUI-183 re-checked: compass, Tilt, Slice first; Enter on Slice and Tilt sets the mode; in the Cube view the
    // compass's arrows tilt, one SetField on the basis, z₀ kept.
    let mut rig = mock();
    rig.onto_compass();
    rig.keys_to(&[Key::ArrowRight, Key::ArrowRight, Key::Enter]);
    assert_eq!(rig.mode(), Touch::Slice, "Enter on Slice");
    rig.keys_to(&[Key::ArrowLeft, Key::Enter]);
    assert_eq!(rig.mode(), Touch::Tilt, "Enter on Tilt");
    rig.keys_to(&[Key::ArrowLeft]);
    check_at(
        &rig.at(),
        &["Compass", "compass"],
        NAV,
        "back on the compass",
    );
    assert_eq!(rig.view(), CompassView::Cube);
    rig.key(Key::Enter, NONE);
    assert_eq!(rig.app.view.focus.mode, INTERACT);
    assert!(
        rig.app.manifold.compass.cursor.is_none(),
        "a cell cursor in the Cube view"
    );
    let (before, n0) = (rig.snap(), rig.applied());
    rig.key(Key::ArrowRight, NONE);
    let after = rig.snap();
    assert_eq!(rig.applied() - n0, 1, "→ in the Cube view: SetFields");
    assert_ne!(
        after.sim.plane.q1, before.sim.plane.q1,
        "→ in the Cube view did not tilt"
    );
    assert_eq!(
        after.sim.plane.z0, before.sim.plane.z0,
        "→ in tilt mode moved z₀"
    );
    let t = View::of(&after.sim.plane).tau1;
    assert!(t != 0.0, "τ₁ unchanged");
    rejects("a Cube-view arrow that moves a cell cursor instead", || {
        assert_eq!(after.sim.plane, before.sim.plane)
    });
}

impl Rig {
    fn keys_to(&mut self, keys: &[Key]) {
        for k in keys {
            self.key(*k, NONE);
        }
    }
}
