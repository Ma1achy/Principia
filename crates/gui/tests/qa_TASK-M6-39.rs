//! QA tests for TASK-M6-39, written from the requirements, R-458 and R-465, not from the implementation:
//!
//! - REQ-GUI-206 (R-458 F1, F3): a drag on the compass's Cube view applies its edit every frame the pointer moves, at
//!   most once a frame, so the snapshot, the Manifold view's slider and the compass's readout follow it, each frame at
//!   the drag's target; the whole drag one undo entry, one Ctrl+Z restoring the value before it, and one console line,
//!   logged when it ends. The drag's target at each frame is computed here from the rates alone: tilt mode R-451 B3's
//!   (`τ₁` by the horizontal travel over the cube's square, `τ₂` by the vertical, up raising it, 180° across, each
//!   clamped to ±90°), slice mode R-465's (the travel along the drawn normal's screen direction `ŝ`, the cube's full
//!   width `2√3·S` the whole range `2·`half-range).
//! - REQ-GUI-207 (R-458 F2 to F5): a drag on a Slice centre slider, on `τ₁`, on Time's scrubber and the figure's pan
//!   applies its edit every frame, each drag one undo entry and one console line; the compass's mode switches on the
//!   drag's first frame, to Slice for a Slice centre slider and to Tilt for a Tilt slider, also at a slider's end, where
//!   no value changes, and on a click into a number box; the compass's drawing follows `z₀` and the basis every frame.
//! - REQ-GUI-217 (R-465): a slice-mode drag moves the slice along the decoder's normal by the drag projected onto `ŝ`,
//!   the unit vector along `P(n̂) − P(0)` (`P` R-451's isometric projection `P(x,y,z) = (CX + S·√3/2·(x − z),
//!   CY + S·(½(x + z) − y))`, `S = min(0.78·w/(2√3), 0.78·h/4)`), the component across `ŝ` doing nothing, so the target
//!   plane's point nearest the cube's centre moves on the screen along `+ŝ`, with the pointer; near the view axis
//!   (`|P(n̂) − P(0)| < 0.25·S`) the vertical travel, up moving the plane along the normal's side facing the viewer
//!   (`n̂·(1, 1, 1) > 0`), at the same rate. The poses are R-465 N4's three. The tilt-mode drag's rates, REQ-GUI-193's
//!   checks, are qa_TASK-M6-33's `qa_mock_compass_cube_tilt_mode_drags`, re-run unchanged.
//!
//! The decoder's plane is built here from R-451's construction, `u₁ = cos τ₁ e_a + sin τ₁ e_d`, `u₂ = cos τ₂ e_b +
//! sin τ₂ e_d`, `q₁ = σ(cos γ u₁ + sin γ u₂)`, `q₂ = σ(cos γ u₂ − sin γ u₁)`, its normal `q₁ × q₂`. Edits are counted
//! from the contract's log (each `SetField` logged as one `info` entry from `contract`, a drag's once at its end,
//! gui_state_contract §2) and from the mock's own count of the edits it applied. gui takes no dependency on validation
//! (R-187), so each check runs its control (R-176) through [`rejects`], on the trace a GUI with the fault would leave.
// The file name `qa_TASK-M6-39` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::accesskit::Role;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
use engine::contract::conformance;
use engine::contract::interface::EngineInterface;
use engine::contract::log::Source;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::Plane;
use engine::contract::snapshot::Snapshot;
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::compass;
use gui::explore::manifold_view::{Touch, GAMMA, SLIDERS, TAU1, TAU2, Z0_HALF_RANGE};
use gui::explore::time::SCRUBBER;
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};

type Z = [f64; 8];
type P3 = [f64; 3];

/// The default chart's frame (render_gui_spec §G2: the chart `z_α × z_β`, the depth axis `z_q3`): `e_a`, `e_b`, `e_d`.
const A: usize = 0;
const B: usize = 1;
const D: usize = 5;
const DEPTH_NAME: &str = "z_q3";
/// The frames each drag moves the pointer on (REQ-GUI-206, REQ-GUI-207: "ten frames").
const FRAMES: usize = 10;
/// f32 pointer positions over the cube's width: a move is read to 1e-5 of the half-range (as qa_TASK-M6-33).
const POINTER_TOL: f64 = 1e-5;
const SQRT3: f64 = 1.732_050_807_568_877_2;

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

// --- vectors and the decoder's plane, computed apart -------------------------------------------------------------------

fn dot3(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit3(a: P3) -> P3 {
    let n = dot3(a, a).sqrt();
    a.map(|x| x / n)
}

/// R-451's construction of `View::basis` in the cube's frame: `(q₁, q₂)` at `τ₁`, `τ₂`, `γ` in degrees, scale `σ`.
fn decoder_basis(t1: f64, t2: f64, g: f64, sigma: f64) -> (P3, P3) {
    let (t1, t2, g) = (t1.to_radians(), t2.to_radians(), g.to_radians());
    let u1 = [t1.cos(), 0.0, t1.sin()];
    let u2 = [0.0, t2.cos(), t2.sin()];
    let q1 = std::array::from_fn(|i| sigma * (g.cos() * u1[i] + g.sin() * u2[i]));
    let q2 = std::array::from_fn(|i| sigma * (g.cos() * u2[i] - g.sin() * u1[i]));
    (q1, q2)
}

/// The decoder's unit normal at `τ₁`, `τ₂`, `γ`, in the cube's frame.
fn normal(t1: f64, t2: f64, g: f64) -> P3 {
    let (q1, q2) = decoder_basis(t1, t2, g, 1.0);
    unit3(cross3(q1, q2))
}

/// A latent vector with components `r` on the frame `(e_a, e_b, e_d)` and `rest` elsewhere.
fn lift(r: P3, rest: &Z) -> Z {
    let mut z = *rest;
    z[A] = r[0];
    z[B] = r[1];
    z[D] = r[2];
    z
}

/// `z`'s components on the frame, in half-ranges: the point the cube draws.
fn on_frame(z: &Z) -> P3 {
    [z[A], z[B], z[D]].map(|x| x / Z0_HALF_RANGE)
}

// --- the isometric projection of R-451 -------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Iso {
    cx: f64,
    cy: f64,
    s: f64,
}

impl Iso {
    fn fit(square: Rect) -> Self {
        let (w, h) = (f64::from(square.width()), f64::from(square.height()));
        Self {
            cx: f64::from(square.center().x),
            cy: f64::from(square.center().y),
            s: (0.78 * w / (2.0 * SQRT3)).min(0.78 * h / 4.0),
        }
    }

    fn at(&self, [x, y, z]: P3) -> [f64; 2] {
        [
            self.cx + self.s * SQRT3 / 2.0 * (x - z),
            self.cy + self.s * (0.5 * (x + z) - y),
        ]
    }

    /// The cube's full screen width, `2√3·S`.
    fn width(&self) -> f64 {
        2.0 * SQRT3 * self.s
    }
}

/// `P(n) − P(0)` over `S`, `y` down.
fn on_screen(n: P3) -> [f64; 2] {
    [SQRT3 / 2.0 * (n[0] - n[2]), 0.5 * (n[0] + n[2]) - n[1]]
}

/// `ŝ`: the unit vector along `P(n̂) − P(0)`.
fn s_hat(n: P3) -> [f64; 2] {
    let v = on_screen(n);
    let len = v[0].hypot(v[1]);
    [v[0] / len, v[1] / len]
}

/// The screen image of the point of the plane through `z₀` with unit normal `n` nearest the cube's centre, `(n·p)·n`.
fn nearest_image(iso: &Iso, n: P3, z0: &Z) -> [f64; 2] {
    let c = dot3(n, on_frame(z0));
    iso.at(n.map(|x| c * x))
}

// --- the rig ---------------------------------------------------------------------------------------------------------

fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// An AccessKit node of a frame: its role, rect and numeric value.
#[derive(Clone, Debug)]
struct Node {
    role: Role,
    rect: Rect,
    value: Option<f64>,
}

fn nodes_of(out: &egui::FullOutput) -> Vec<Node> {
    let Some(update) = &out.platform_output.accesskit_update else {
        return Vec::new();
    };
    update
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let b = n.bounds()?;
            Some(Node {
                role: n.role(),
                rect: Rect::from_min_max(
                    egui::pos2(b.x0 as f32, b.y0 as f32),
                    egui::pos2(b.x1 as f32, b.y1 as f32),
                ),
                value: n.numeric_value(),
            })
        })
        .collect()
}

fn flatten_texts(shape: &egui::Shape, out: &mut Vec<(String, Rect)>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten_texts(s, out)),
        egui::Shape::Text(t) => out.push((t.galley.text().to_owned(), t.visual_bounding_rect())),
        _ => {}
    }
}

/// The texts a frame painted inside `rect`.
fn texts_in(out: &egui::FullOutput, rect: Rect) -> Vec<String> {
    let mut all = Vec::new();
    for clipped in &out.shapes {
        flatten_texts(&clipped.shape, &mut all);
    }
    all.into_iter()
        .filter(|(_, r)| rect.contains(r.center()))
        .map(|(t, _)| t)
        .collect()
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
    let p = rig.snap().sim.plane;
    for (q, k) in [(&p.q1, A), (&p.q2, B)] {
        assert!(
            (0..8).all(|i| i == k || q[i] == 0.0),
            "the default basis is not z_α × z_β: {p:?}"
        );
    }
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

    fn snap(&self) -> Snapshot {
        self.app.snapshot().clone()
    }

    fn depth(&self) -> u32 {
        self.app.snapshot().history.undo_depth
    }

    /// The mock's count of the edits it has applied.
    fn applied(&mut self) -> u64 {
        self.app.side().engine().edits_applied()
    }

    /// The contract's lines in the console from row `from`, the hover's (a no-history cursor) left out.
    fn lines_from(&self, from: usize) -> Vec<String> {
        self.app.console()[from..]
            .iter()
            .filter(|e| {
                e.source == Source::Contract && !e.message.starts_with("SetField Hover.at ")
            })
            .map(|e| e.message.clone())
            .collect()
    }

    /// Sets the plane through the contract, as a test's setup, and lets the compass come to rest in `mode`.
    fn set_plane(&mut self, plane: &Plane, mode: Touch) {
        for edit in [
            SimField::Z0(plane.z0),
            SimField::Basis {
                q1: plane.q1,
                q2: plane.q2,
            },
        ] {
            self.app.set_field(SetField {
                edit: Edit::Sim(edit),
                no_history: false,
            });
        }
        self.app.manifold.compass.mode = mode;
        self.idle(3);
        for _ in 0..400 {
            if !self.app.manifold.compass.moving {
                break;
            }
            self.idle(1);
        }
        assert!(
            !self.app.manifold.compass.moving,
            "the compass did not settle"
        );
    }

    fn square(&self) -> Rect {
        compass::square(layout().compass)
    }

    /// A pointer drag from `from` by `delta`, over `steps` frames, released, then the pointer taken off the compass.
    fn drag(&mut self, from: Pos2, delta: Vec2, steps: usize) {
        let _ = self.frame(vec![Event::PointerMoved(from)]);
        let _ = self.frame(vec![button(from, true)]);
        for i in 1..=steps {
            let _ = self.frame(vec![Event::PointerMoved(
                from + delta * (i as f32 / steps as f32),
            )]);
        }
        let _ = self.frame(vec![button(from + delta, false)]);
        self.idle(2);
        let _ = self.frame(vec![Event::PointerMoved(layout().figure.center())]);
        self.idle(1);
    }

    /// The rect of the slider placed as control `id`, as a frame draws it.
    fn slider_rect(&mut self, id: &str) -> Rect {
        let out = self.frame(Vec::new());
        let place = self
            .app
            .keyboard
            .place_of(id)
            .unwrap_or_else(|| panic!("`{id}` placed"))
            .rect;
        nodes_of(&out)
            .into_iter()
            .find(|n| n.role == Role::Slider && place.contains(n.rect.center()))
            .map(|n| n.rect)
            .unwrap_or_else(|| panic!("no slider drawn in `{id}`"))
    }

    /// The value of the slider placed as control `id` in the frame `out`.
    fn slider_value(&self, out: &egui::FullOutput, id: &str) -> f64 {
        let place = self.app.keyboard.place_of(id).expect("placed").rect;
        nodes_of(out)
            .into_iter()
            .find(|n| n.role == Role::Slider && place.contains(n.rect.center()))
            .and_then(|n| n.value)
            .unwrap_or_else(|| panic!("no slider value drawn in `{id}`"))
    }
}

/// The rig's default scale and the rest of its z₀.
fn base(rig: &Rig) -> (f64, Z) {
    let p = rig.snap().sim.plane;
    (p.q1[A].abs(), p.z0)
}

/// The plane at `τ₁`, `τ₂`, `γ` with z₀'s point `p` on the frame (in half-ranges), on the rig's scale and rest.
fn rig_plane(rig: &Rig, t1: f64, t2: f64, g: f64, p: P3) -> Plane {
    let (sigma, rest) = base(rig);
    let (q1, q2) = decoder_basis(t1, t2, g, sigma);
    Plane {
        z0: lift(p.map(|x| x * Z0_HALF_RANGE), &rest),
        q1: lift(q1, &[0.0; 8]),
        q2: lift(q2, &[0.0; 8]),
    }
}

/// τ₁ and τ₂ of a γ = 0 basis in the default frame, in degrees.
fn taus(plane: &Plane) -> (f64, f64) {
    (
        plane.q1[D].atan2(plane.q1[A]).to_degrees(),
        plane.q2[D].atan2(plane.q2[B]).to_degrees(),
    )
}

/// The number in a readout line `prefix <number><suffix>`.
fn read_number(lines: &[String], prefix: &str, suffix: &str) -> Option<f64> {
    lines.iter().find_map(|l| {
        let i = l.find(prefix)?;
        let rest = &l[i + prefix.len()..];
        let rest = rest.trim_start();
        let end = rest.find(suffix)?;
        rest[..end]
            .trim()
            .replace('−', "-")
            .replace('+', "")
            .parse()
            .ok()
    })
}

// === REQ-GUI-217: the slice drag follows the pointer ==================================================================

/// What a slice-mode drag by `delta` did at a pose: the target plane before and after (the contract's), the cube's
/// projection, and the pose's normal.
#[derive(Clone, Debug)]
struct Moved {
    before: Plane,
    after: Plane,
    iso: Iso,
    n: P3,
}

fn slice_drag(t1: f64, t2: f64, delta: impl Fn(&Iso) -> Vec2) -> Moved {
    let mut rig = mock();
    let plane = rig_plane(&rig, t1, t2, 0.0, [0.0; 3]);
    rig.set_plane(&plane, Touch::Slice);
    let sq = rig.square();
    let iso = Iso::fit(sq);
    let before = rig.snap().sim.plane;
    rig.drag(sq.center(), delta(&iso), 6);
    Moved {
        before,
        after: rig.snap().sim.plane,
        iso,
        n: normal(t1, t2, 0.0),
    }
}

/// The scalar move of `m` along its normal, in half-ranges, after checking the basis is unchanged, every component
/// finite and the move along the normal alone.
fn move_along_normal(m: &Moved) -> f64 {
    assert_eq!(
        (m.before.q1, m.before.q2),
        (m.after.q1, m.after.q2),
        "a slice drag changed the basis"
    );
    let d: Z = std::array::from_fn(|i| m.after.z0[i] - m.before.z0[i]);
    assert!(d.iter().all(|x| x.is_finite()), "a non-finite edit: {d:?}");
    let nl = lift(m.n, &[0.0; 8]);
    let along = d.iter().zip(&nl).map(|(x, y)| x * y).sum::<f64>();
    let off: f64 = (0..8)
        .map(|i| (d[i] - along * nl[i]).abs())
        .fold(0.0, f64::max);
    assert!(
        off <= POINTER_TOL * Z0_HALF_RANGE,
        "the move {d:?} is not along the normal {:?}",
        m.n
    );
    along / Z0_HALF_RANGE
}

/// The drag moved the slice by `want` half-ranges along the normal; where `want` is not zero, the image of the target
/// plane's point nearest the centre moved along `dir` on the screen, with the pointer.
fn check_slice_move(m: &Moved, want: f64, dir: [f64; 2], what: &str) {
    let got = move_along_normal(m);
    assert!(
        (got - want).abs() <= POINTER_TOL,
        "{what}: the slice moved {got} half-ranges along n̂, not {want}"
    );
    if want != 0.0 {
        let (a, b) = (
            nearest_image(&m.iso, m.n, &m.before.z0),
            nearest_image(&m.iso, m.n, &m.after.z0),
        );
        let v = [b[0] - a[0], b[1] - a[1]];
        let len = v[0].hypot(v[1]);
        let cos = (v[0] * dir[0] + v[1] * dir[1]) / (len * dir[0].hypot(dir[1]));
        assert!(
            len > 1.0 && cos > 1.0 - 1e-6,
            "{what}: the plane's image moved by {v:?}, not along {dir:?}"
        );
    }
}

/// `k` points along `v`.
fn along(v: [f64; 2], k: f64) -> Vec2 {
    egui::vec2((v[0] * k) as f32, (v[1] * k) as f32)
}

/// The move R-451 B3 as written would make, reading the drag's horizontal component over the cube's width.
fn horizontal_reading(m: &Moved, delta: Vec2) -> Moved {
    let k = f64::from(delta.x) / m.iso.width() * 2.0 * Z0_HALF_RANGE;
    let mut after = m.before.clone();
    let nl = lift(m.n, &[0.0; 8]);
    after.z0 = std::array::from_fn(|i| m.before.z0[i] + k * nl[i]);
    Moved { after, ..m.clone() }
}

/// The move a GUI reading the drag along `dir` (unit) would make.
fn reading_along(m: &Moved, delta: Vec2, dir: [f64; 2]) -> Moved {
    let travel = f64::from(delta.x) * dir[0] + f64::from(delta.y) * dir[1];
    let k = travel / m.iso.width() * 2.0 * Z0_HALF_RANGE;
    let mut after = m.before.clone();
    let nl = lift(m.n, &[0.0; 8]);
    after.z0 = std::array::from_fn(|i| m.before.z0[i] + k * nl[i]);
    Moved { after, ..m.clone() }
}

#[test]
fn qa_mock_compass_slice_follows_pointer_untilted() {
    // R-465 N4: untilted, n̂ = e_d and ŝ = (−√3/2, ½) on the screen, y down.
    let n = normal(0.0, 0.0, 0.0);
    assert!(
        (n[2] - 1.0).abs() < 1e-12,
        "untilted, the normal is e_d: {n:?}"
    );
    let sh = s_hat(n);
    assert!(
        (sh[0] + SQRT3 / 2.0).abs() < 1e-12 && (sh[1] - 0.5).abs() < 1e-12,
        "ŝ {sh:?}"
    );
    let across = [-sh[1], sh[0]];
    // Half the cube's width, √3·S, along ŝ: half the range along +e_d, the plane's image along +ŝ.
    let m = slice_drag(0.0, 0.0, |iso| along(sh, iso.width() / 2.0));
    check_slice_move(&m, 1.0, sh, "along ŝ");
    // Control: R-451 B3's horizontal reading moves the plane along −e_d for the same drag, against the pointer.
    let delta = along(sh, m.iso.width() / 2.0);
    rejects("the horizontal reading of a drag along ŝ", || {
        check_slice_move(&horizontal_reading(&m, delta), 1.0, sh, "along ŝ")
    });
    // The same length across ŝ: nothing.
    let m = slice_drag(0.0, 0.0, |iso| along(across, iso.width() / 2.0));
    check_slice_move(&m, 0.0, sh, "across ŝ");
    // Straight right, √3·S: along −e_d by (√3·S·ŝ_x)/(2√3·S)·2 = −√3/2 half-ranges, the image moving right and up.
    let m = slice_drag(0.0, 0.0, |iso| egui::vec2((iso.width() / 2.0) as f32, 0.0));
    check_slice_move(&m, -SQRT3 / 2.0, [1.0, -1.0 / SQRT3], "straight right");
    let (a, b) = (
        nearest_image(&m.iso, m.n, &m.before.z0),
        nearest_image(&m.iso, m.n, &m.after.z0),
    );
    assert!(
        b[0] > a[0] && b[1] < a[1],
        "straight right: the image moved {a:?} → {b:?}, not right and up"
    );
}

#[test]
fn qa_mock_compass_slice_follows_pointer_tilted() {
    // R-465 N4: τ₁ = 30°, n̂ ∝ (−½, 0, √3/2), ŝ along (−1.183, 0.183).
    let n = normal(30.0, 0.0, 0.0);
    assert!(
        (n[0] + 0.5).abs() < 1e-12 && n[1].abs() < 1e-12 && (n[2] - SQRT3 / 2.0).abs() < 1e-12,
        "n̂ {n:?}"
    );
    let v = on_screen(n);
    assert!(
        (v[0] + 1.183).abs() < 5e-4 && (v[1] - 0.183).abs() < 5e-4,
        "P(n̂) − P(0) is {v:?}·S"
    );
    let sh = s_hat(n);
    let across = [-sh[1], sh[0]];
    let m = slice_drag(30.0, 0.0, |iso| along(sh, iso.width() / 2.0));
    check_slice_move(&m, 1.0, sh, "tilted, along ŝ");
    let delta = along(sh, m.iso.width() / 2.0);
    rejects("the horizontal reading, tilted", || {
        check_slice_move(&horizontal_reading(&m, delta), 1.0, sh, "tilted, along ŝ")
    });
    let m = slice_drag(30.0, 0.0, |iso| along(across, iso.width() / 2.0));
    check_slice_move(&m, 0.0, sh, "tilted, across ŝ");
    // Control: a GUI reading the drag along the untilted P(e_d) moves the plane for this drag across the tilted ŝ.
    let untilted = s_hat([0.0, 0.0, 1.0]);
    let delta = along(across, m.iso.width() / 2.0);
    rejects("the untilted ŝ in place of the tilted", || {
        check_slice_move(
            &reading_along(&m, delta, untilted),
            0.0,
            sh,
            "tilted, across ŝ",
        )
    });
}

#[test]
fn qa_mock_compass_slice_follows_pointer_at_the_view_axis() {
    // R-465 N3: τ₁ = τ₂ = −45°, γ = 0: n̂ = (1, 1, 1)/√3, drawn as a point; the drag reads its vertical travel, up moving
    // the plane along the side facing the viewer, n̂ itself here (n̂·(1, 1, 1) > 0).
    let n = normal(-45.0, -45.0, 0.0);
    assert!(n.iter().all(|x| (x - 1.0 / SQRT3).abs() < 1e-12), "n̂ {n:?}");
    let v = on_screen(n);
    assert!(
        v[0].hypot(v[1]) < 0.25,
        "the normal's image is {v:?}·S, not under 0.25·S"
    );
    let up = slice_drag(-45.0, -45.0, |iso| {
        egui::vec2(0.0, -(iso.width() / 2.0) as f32)
    });
    let check_up = |m: &Moved| {
        let got = move_along_normal(m);
        assert!(
            (got - 1.0).abs() <= POINTER_TOL,
            "an upward drag of √3·S moved {got} half-ranges, not +1"
        );
    };
    check_up(&up);
    // Controls: a GUI that sends no edit, and one that sends a non-finite one.
    rejects("no edit at the view axis", || {
        check_up(&Moved {
            after: up.before.clone(),
            ..up.clone()
        })
    });
    rejects("a non-finite edit at the view axis", || {
        let mut after = up.after.clone();
        after.z0[D] = f64::NAN;
        check_up(&Moved {
            after,
            ..up.clone()
        })
    });
    let side = slice_drag(-45.0, -45.0, |iso| {
        egui::vec2((iso.width() / 2.0) as f32, 0.0)
    });
    let got = move_along_normal(&side);
    assert!(
        got.abs() <= POINTER_TOL,
        "a horizontal drag at the view axis moved {got} half-ranges"
    );
}

// === REQ-GUI-206: the live compass drag ===============================================================================

/// One frame of a compass drag: the plane the snapshot holds and the drag's target then, the slider's value and its
/// target, the readout's number and its target, the undo depth, the edits the frame applied.
#[derive(Clone, Debug)]
struct Seen {
    plane: Plane,
    target: Plane,
    slider: f64,
    want_slider: f64,
    readout: Option<f64>,
    want_readout: f64,
    depth: u32,
    edits: u64,
}

#[derive(Clone, Debug)]
struct Trace {
    start: Plane,
    frames: Vec<Seen>,
    depth0: u32,
    end_depth: u32,
    lines: Vec<String>,
    undone: Plane,
}

/// The plane a slice-mode drag of `delta` from the start sets, untilted (n̂ = e_d), by R-465's rate.
fn slice_target(start: &Plane, iso: &Iso, delta: Vec2) -> Plane {
    let sh = s_hat([0.0, 0.0, 1.0]);
    let travel = f64::from(delta.x) * sh[0] + f64::from(delta.y) * sh[1];
    let mut p = start.clone();
    p.z0[D] += travel / iso.width() * 2.0 * Z0_HALF_RANGE;
    p
}

/// The plane a tilt-mode drag of `delta` from the untilted start sets, by R-451 B3's rates over the square, γ = 0.
fn tilt_target(start: &Plane, sq: Rect, delta: Vec2) -> Plane {
    let sigma = start.q1[A].hypot(start.q1[D]);
    let t1 = (f64::from(delta.x) / f64::from(sq.width()) * 180.0).clamp(-90.0, 90.0);
    let t2 = (-f64::from(delta.y) / f64::from(sq.height()) * 180.0).clamp(-90.0, 90.0);
    let (q1, q2) = decoder_basis(t1, t2, 0.0, sigma);
    Plane {
        z0: start.z0,
        q1: lift(q1, &[0.0; 8]),
        q2: lift(q2, &[0.0; 8]),
    }
}

fn max_diff(a: &Z, b: &Z) -> f64 {
    (0..8).map(|i| (a[i] - b[i]).abs()).fold(0.0, f64::max)
}

fn plane_diff(a: &Plane, b: &Plane) -> f64 {
    max_diff(&a.z0, &b.z0)
        .max(max_diff(&a.q1, &b.q1))
        .max(max_diff(&a.q2, &b.q2))
}

/// A compass drag in `mode` from the cube's centre, `step` a frame for [`FRAMES`] frames, then one frame with two
/// pointer events (half a step, then a whole), the release, and one Ctrl+Z.
fn compass_drag(mode: Touch, step: Vec2) -> Trace {
    let mut rig = mock();
    let start = rig_plane(&rig, 0.0, 0.0, 0.0, [0.0; 3]);
    rig.set_plane(&start, mode);
    let start = rig.snap().sim.plane;
    let sq = rig.square();
    let iso = Iso::fit(sq);
    let compass_rect = layout().compass;
    let from = sq.center();
    let (depth0, rows) = (rig.depth(), rig.app.console().len());
    let _ = rig.frame(vec![Event::PointerMoved(from)]);
    let _ = rig.frame(vec![button(from, true)]);
    let mut frames = Vec::new();
    let mut seen = |rig: &mut Rig, events: Vec<Event>, delta: Vec2| {
        let n0 = rig.applied();
        let out = rig.frame(events);
        let edits = rig.applied() - n0;
        let (target, slider_id) = match mode {
            Touch::Slice => (slice_target(&start, &iso, delta), SLIDERS[D]),
            Touch::Tilt => (tilt_target(&start, sq, delta), TAU1),
        };
        let texts = texts_in(&out, compass_rect);
        let (want_slider, want_readout, readout) = match mode {
            Touch::Slice => (
                target.z0[D],
                target.z0[D] / Z0_HALF_RANGE,
                read_number(&texts, &format!("{DEPTH_NAME} ="), "of range"),
            ),
            Touch::Tilt => {
                let t1 = taus(&target).0;
                (
                    t1,
                    t1,
                    read_number(&texts, &format!("τ₁ → {DEPTH_NAME}"), "°"),
                )
            }
        };
        frames.push(Seen {
            plane: rig.snap().sim.plane,
            target,
            slider: rig.slider_value(&out, slider_id),
            want_slider,
            readout,
            want_readout,
            depth: rig.depth(),
            edits,
        });
    };
    for k in 1..=FRAMES {
        let delta = step * k as f32;
        seen(&mut rig, vec![Event::PointerMoved(from + delta)], delta);
    }
    let (half, whole) = (step * (FRAMES as f32 + 0.5), step * (FRAMES as f32 + 1.0));
    seen(
        &mut rig,
        vec![
            Event::PointerMoved(from + half),
            Event::PointerMoved(from + whole),
        ],
        whole,
    );
    let _ = rig.frame(vec![button(from + whole, false)]);
    rig.idle(2);
    let end_depth = rig.depth();
    let lines = rig.lines_from(rows);
    for pressed in [true, false] {
        let _ = rig.frame(vec![Event::Key {
            key: Key::Z,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        }]);
    }
    rig.idle(2);
    Trace {
        start,
        frames,
        depth0,
        end_depth,
        lines,
        undone: rig.snap().sim.plane,
    }
}

/// Every frame: one edit; the snapshot at the drag's target; the slider and the readout at it, each changed from the
/// frame before; the drag one entry from its first frame; one console line; one Ctrl+Z restoring the start.
fn check_live(t: &Trace, readout_tol: f64) {
    let mut prev: Option<&Seen> = None;
    for (k, s) in t.frames.iter().enumerate() {
        let k = k + 1;
        assert_eq!(s.edits, 1, "frame {k}: {} edits, not one", s.edits);
        assert!(
            plane_diff(&s.plane, &s.target) <= POINTER_TOL,
            "frame {k}: the snapshot {:?} is not the drag's target {:?}",
            s.plane,
            s.target
        );
        // The slider as the frame the pointer moved drew it, beside the readout and the figure of that frame.
        assert!(
            (s.slider - s.want_slider).abs() <= 1e-4 * s.want_slider.abs().max(1.0),
            "frame {k}: the slider reads {}, not the target's {} (the frame before's target: {:?})",
            s.slider,
            s.want_slider,
            prev.map(|p| p.want_slider)
        );
        let r = s
            .readout
            .unwrap_or_else(|| panic!("frame {k}: no readout line"));
        assert!(
            (r - s.want_readout).abs() <= readout_tol,
            "frame {k}: the readout reads {r}, not the target's {}",
            s.want_readout
        );
        assert_eq!(
            s.depth,
            t.depth0 + 1,
            "frame {k}: the drag is not one entry"
        );
        if let Some(p) = prev {
            assert!(
                plane_diff(&s.plane, &p.plane) > 1e-6,
                "frame {k}: the snapshot did not change"
            );
            assert!(s.slider != p.slider, "frame {k}: the slider did not change");
        }
        prev = Some(s);
    }
    assert_eq!(t.end_depth, t.depth0 + 1, "the drag is not one undo entry");
    assert_eq!(
        t.lines.len(),
        1,
        "the console lines of one drag: {:?}",
        t.lines
    );
    assert!(
        plane_diff(&t.undone, &t.start) <= 1e-12,
        "one Ctrl+Z left {:?}, not the plane before the drag {:?}",
        t.undone,
        t.start
    );
}

/// The trace of a GUI sending the drag's edit only on its release.
fn only_on_release(t: &Trace) -> Trace {
    let mut c = t.clone();
    for s in &mut c.frames {
        s.plane = t.start.clone();
        // Where the drag began, untilted through the centre, both z_q3 and τ₁ read 0.
        s.slider = 0.0;
        s.readout = Some(0.0);
        s.depth = t.depth0;
        s.edits = 0;
    }
    c
}

/// The trace of a GUI pushing an undo entry a frame.
fn entry_per_frame(t: &Trace) -> Trace {
    let mut c = t.clone();
    for (k, s) in c.frames.iter_mut().enumerate() {
        s.depth = t.depth0 + 1 + k as u32;
    }
    c.end_depth = t.depth0 + c.frames.len() as u32;
    c.lines = vec![t.lines[0].clone(); c.frames.len()];
    c
}

#[test]
fn qa_mock_live_compass_drag_slice_mode() {
    // Along the untilted ŝ, 5 points a frame: R-465's slice drag, z₀ along e_d.
    let sh = s_hat([0.0, 0.0, 1.0]);
    let t = compass_drag(Touch::Slice, along(sh, 5.0));
    // The readout's fraction is written to two decimals.
    check_live(&t, 0.005 + 1e-9);
    rejects("a drag that sends its edit only on release", || {
        check_live(&only_on_release(&t), 0.005 + 1e-9)
    });
    rejects("a drag that pushes an undo entry per frame", || {
        check_live(&entry_per_frame(&t), 0.005 + 1e-9)
    });
}

#[test]
fn qa_mock_live_compass_drag_tilt_mode() {
    // Right and up: τ₁ and τ₂ both move, well inside the clamp.
    let t = compass_drag(Touch::Tilt, egui::vec2(5.0, -2.0));
    // τ₁'s angle is written to one decimal.
    check_live(&t, 0.05 + 1e-9);
    rejects("a drag that sends its edit only on release", || {
        check_live(&only_on_release(&t), 0.05 + 1e-9)
    });
    rejects("a drag that pushes an undo entry per frame", || {
        check_live(&entry_per_frame(&t), 0.05 + 1e-9)
    });
}

#[test]
fn qa_mock_conformance_the_mock_keeps_history_keys() {
    // The mock passes the one suite, its key cases with it (R-390, R-458 F1).
    conformance::run(MockEngine::frozen)
        .unwrap_or_else(|e| panic!("the mock fails the suite: {e}"));
    // Control: the mock with its keys ignored (each keyed edit its own entry, as the interface's default) fails.
    struct IgnoresKeys(MockEngine);
    impl EngineInterface for IgnoresKeys {
        fn set_field(&mut self, edit: SetField) {
            self.0.set_field(edit);
        }
        fn snapshot(&mut self) -> Snapshot {
            self.0.snapshot()
        }
        fn undo(&mut self) {
            self.0.undo();
        }
        fn redo(&mut self) {
            self.0.redo();
        }
    }
    let err = conformance::run(|| IgnoresKeys(MockEngine::frozen()))
        .expect_err("a mock ignoring keys passed");
    assert!(err.contains("key"), "the failure names no key case: {err}");
}

// === REQ-GUI-207: the live slider drags and the compass's switch ======================================================

/// One frame of a slider drag: the snapshot's plane, the mock's (the figure's chart), the compass's drawn plane and
/// its mode, the undo depth.
#[derive(Clone, Debug)]
struct SliderSeen {
    plane: Plane,
    figure: Plane,
    cut: Option<compass::Cut>,
    mode: Touch,
    depth: u32,
}

#[derive(Clone, Debug)]
struct SliderTrace {
    /// The compass's mode after the press on the thumb, before any move.
    pressed: Touch,
    frames: Vec<SliderSeen>,
    depth0: u32,
    end_depth: u32,
    lines: Vec<String>,
    edits: u64,
}

/// A drag on the slider `id`, pressed at `at` (a fraction of the rail, 0 its left end, 1 its right) and moved right by
/// `step` a frame for [`FRAMES`] frames; the compass first in `mode`, the plane at `(τ₁, τ₂)`.
fn slider_drag(id: &str, at: f32, step: f32, mode: Touch, (t1, t2): (f64, f64)) -> SliderTrace {
    let mut rig = mock();
    let plane = rig_plane(&rig, t1, t2, 0.0, [0.0; 3]);
    rig.set_plane(&plane, mode);
    let rail = rig.slider_rect(id);
    let from = egui::pos2(rail.left() + rail.width() * at, rail.center().y);
    let (depth0, rows, n0) = (rig.depth(), rig.app.console().len(), rig.applied());
    let _ = rig.frame(vec![Event::PointerMoved(from)]);
    let _ = rig.frame(vec![button(from, true)]);
    let pressed = rig.app.manifold.compass.mode;
    let mut frames = Vec::new();
    for k in 1..=FRAMES {
        let _ = rig.frame(vec![Event::PointerMoved(
            from + egui::vec2(step * k as f32, 0.0),
        )]);
        let snap = rig.snap();
        let drawing = compass::current(layout().compass, &snap.sim, &rig.app.manifold.compass);
        frames.push(SliderSeen {
            plane: snap.sim.plane.clone(),
            figure: rig.app.side().engine().plane().clone(),
            cut: drawing.slice.map(|s| s.cut),
            mode: rig.app.manifold.compass.mode,
            depth: rig.depth(),
        });
    }
    let _ = rig.frame(vec![button(
        from + egui::vec2(step * FRAMES as f32, 0.0),
        false,
    )]);
    rig.idle(2);
    SliderTrace {
        pressed,
        frames,
        depth0,
        end_depth: rig.depth(),
        lines: rig.lines_from(rows),
        edits: rig.applied() - n0,
    }
}

/// The compass in `mode` from the press on; each frame the snapshot's plane moved by `moved` (strictly, the same way),
/// the figure's chart the snapshot's, and the drawing changed by `drawn`; one entry from the first frame; one line.
fn check_slider(
    t: &SliderTrace,
    mode: Touch,
    moved: fn(&Plane) -> f64,
    drawn: fn(&compass::Cut) -> Vec<f64>,
) {
    assert_eq!(
        t.pressed, mode,
        "the compass's mode on the drag's first frame"
    );
    let mut prev: Option<&SliderSeen> = None;
    for (k, s) in t.frames.iter().enumerate() {
        let k = k + 1;
        assert_eq!(s.mode, mode, "frame {k}: the compass's mode");
        assert_eq!(
            s.figure, s.plane,
            "frame {k}: the figure's chart is not the snapshot's"
        );
        assert_eq!(
            s.depth,
            t.depth0 + 1,
            "frame {k}: the drag is not one entry"
        );
        let cut = s
            .cut
            .as_ref()
            .unwrap_or_else(|| panic!("frame {k}: no plane drawn"));
        if let Some(p) = prev {
            assert!(
                moved(&s.plane) > moved(&p.plane),
                "frame {k}: the value did not move with the drag"
            );
            assert_ne!(
                drawn(cut),
                drawn(p.cut.as_ref().expect("drawn")),
                "frame {k}: the compass's drawing did not move"
            );
        }
        prev = Some(s);
    }
    assert_eq!(t.end_depth, t.depth0 + 1, "the drag is not one undo entry");
    assert_eq!(
        t.lines.len(),
        1,
        "the console lines of one drag: {:?}",
        t.lines
    );
}

fn slider_entry_per_frame(t: &SliderTrace) -> SliderTrace {
    let mut c = t.clone();
    for (k, s) in c.frames.iter_mut().enumerate() {
        s.depth = t.depth0 + 1 + k as u32;
    }
    c.end_depth = t.depth0 + FRAMES as u32;
    c.lines = vec![t.lines[0].clone(); FRAMES];
    c
}

#[test]
fn qa_mock_live_slider_drag_slice_centre_of_the_normal() {
    // From the rail's middle (z_q3 = 0), right: z_q3 rises every frame, the compass in Slice from the press, its drawn
    // offset (n̂ = e_d, the offset z_q3 in half-ranges) following.
    let t = slider_drag(SLIDERS[D], 0.5, 6.0, Touch::Tilt, (0.0, 0.0));
    let z: fn(&Plane) -> f64 = |p| p.z0[D];
    let offset: fn(&compass::Cut) -> Vec<f64> = |c| vec![c.offset];
    check_slider(&t, Touch::Slice, z, offset);
    assert!(
        t.frames[0].plane.z0[D] > 0.0,
        "the first frame did not move z_q3"
    );
    rejects("a switch only on a changed value", || {
        check_slider(
            &SliderTrace {
                pressed: Touch::Tilt,
                ..t.clone()
            },
            Touch::Slice,
            z,
            offset,
        )
    });
    rejects("a slider that pushes an undo entry per frame", || {
        check_slider(&slider_entry_per_frame(&t), Touch::Slice, z, offset)
    });
}

#[test]
fn qa_mock_live_slider_drag_tau1() {
    // From τ₁ = 0, right: τ₁ rises every frame, the compass in Tilt from the press, its drawn normal turning.
    let t = slider_drag(TAU1, 0.5, 6.0, Touch::Slice, (0.0, 0.0));
    let tau1: fn(&Plane) -> f64 = |p| taus(p).0;
    let normal: fn(&compass::Cut) -> Vec<f64> = |c| c.normal.to_vec();
    check_slider(&t, Touch::Tilt, tau1, normal);
    rejects("a switch only on a changed value", || {
        check_slider(
            &SliderTrace {
                pressed: Touch::Slice,
                ..t.clone()
            },
            Touch::Tilt,
            tau1,
            normal,
        )
    });
    rejects("a slider that pushes an undo entry per frame", || {
        check_slider(&slider_entry_per_frame(&t), Touch::Tilt, tau1, normal)
    });
}

#[test]
fn qa_mock_live_slider_drag_tau2_pressed_at_its_end_switches_with_no_value_change() {
    // τ₂ at 90°, the compass in Slice: a press on the rail's right end, the thumb, dragged on past it. No value changes,
    // and the compass is in Tilt from the press.
    let t = slider_drag(TAU2, 0.995, 4.0, Touch::Slice, (0.0, 90.0));
    let check = |t: &SliderTrace| {
        assert_eq!(
            t.pressed,
            Touch::Tilt,
            "a press on τ₂'s thumb at its end left the compass in Slice"
        );
        assert!(
            t.frames.iter().all(|s| s.mode == Touch::Tilt),
            "held past the end, the mode went back"
        );
    };
    check(&t);
    assert_eq!(
        (t.end_depth, t.edits),
        (t.depth0, 0),
        "a value changed past the end"
    );
    assert!(
        t.frames
            .iter()
            .all(|s| (taus(&s.plane).1 - 90.0).abs() < 1e-9),
        "τ₂ left 90°"
    );
    rejects("a switch only on a changed value", || {
        check(&SliderTrace {
            pressed: Touch::Slice,
            ..t.clone()
        })
    });
}

#[test]
fn qa_mock_live_slider_drag_a_click_into_gammas_box_switches_to_tilt() {
    let mut rig = mock();
    let plane = rig_plane(&rig, 0.0, 0.0, 0.0, [0.0; 3]);
    rig.set_plane(&plane, Touch::Slice);
    let out = rig.frame(Vec::new());
    let place = rig.app.keyboard.place_of(GAMMA).expect("γ placed").rect;
    let field = nodes_of(&out)
        .into_iter()
        .find(|n| n.role != Role::Slider && n.value.is_some() && place.contains(n.rect.center()))
        .map(|n| n.rect)
        .expect("γ's number box");
    let before = rig.snap().sim.plane;
    let at = field.center();
    let _ = rig.frame(vec![Event::PointerMoved(at)]);
    let _ = rig.frame(vec![button(at, true)]);
    let _ = rig.frame(vec![button(at, false)]);
    rig.idle(1);
    let mode = rig.app.manifold.compass.mode;
    let check = |mode: Touch| {
        assert_eq!(
            mode,
            Touch::Tilt,
            "a click into γ's number box left the compass in Slice"
        )
    };
    check(mode);
    assert_eq!(
        rig.snap().sim.plane,
        before,
        "a click into the box changed the plane"
    );
    rejects("a switch only on a changed value", || check(Touch::Slice));
}

/// The scrubber's drag: the playhead after each frame, the depth before and after, its lines.
fn scrub() -> (Vec<f64>, u32, u32, Vec<String>, f64) {
    let mut rig = mock();
    let rail = rig.slider_rect(SCRUBBER);
    let t_max = rig.snap().sim.horizon.t;
    let (depth0, rows) = (rig.depth(), rig.app.console().len());
    let from = egui::pos2(rail.left() + rail.width() * 0.1, rail.center().y);
    let _ = rig.frame(vec![Event::PointerMoved(from)]);
    let _ = rig.frame(vec![button(from, true)]);
    let mut times = Vec::new();
    for k in 1..=FRAMES {
        let _ = rig.frame(vec![Event::PointerMoved(
            from + egui::vec2(8.0 * k as f32, 0.0),
        )]);
        times.push(rig.snap().render.playhead.t);
    }
    let _ = rig.frame(vec![button(
        from + egui::vec2(8.0 * FRAMES as f32, 0.0),
        false,
    )]);
    rig.idle(2);
    (times, depth0, rig.depth(), rig.lines_from(rows), t_max)
}

fn check_one_entry(what: &str, depth0: u32, depth: u32, lines: &[String]) {
    assert_eq!(depth, depth0 + 1, "{what}: not one undo entry");
    assert_eq!(
        lines.len(),
        1,
        "{what}: the console lines of one drag: {lines:?}"
    );
}

#[test]
fn qa_mock_live_slider_drag_the_scrubber_follows_and_is_one_entry() {
    let (times, depth0, depth, lines, t_max) = scrub();
    let check = |times: &[f64]| {
        assert!(
            times.windows(2).all(|w| w[1] > w[0]) && times[0] > 0.0 && times[FRAMES - 1] <= t_max,
            "the playhead does not follow the scrub every frame: {times:?}"
        );
    };
    check(&times);
    check_one_entry("the scrubber", depth0, depth, &lines);
    rejects("a scrubber that applies only on release", || {
        check(&[times[0]; FRAMES])
    });
    rejects("a scrubber pushing an entry a frame", || {
        check_one_entry(
            "the scrubber",
            depth0,
            depth0 + FRAMES as u32,
            &vec![lines[0].clone(); FRAMES],
        )
    });
}

#[test]
fn qa_mock_live_slider_drag_the_figures_pan_follows_and_is_one_entry() {
    let mut rig = mock();
    let from = layout().figure.center();
    let (depth0, rows) = (rig.depth(), rig.app.console().len());
    let _ = rig.frame(vec![Event::PointerMoved(from)]);
    let _ = rig.frame(vec![button(from, true)]);
    let mut seen = Vec::new();
    for k in 1..=FRAMES {
        let _ = rig.frame(vec![Event::PointerMoved(
            from + egui::vec2(8.0 * k as f32, 0.0),
        )]);
        seen.push((rig.snap().sim.plane.z0, rig.depth()));
    }
    let _ = rig.frame(vec![button(
        from + egui::vec2(8.0 * FRAMES as f32, 0.0),
        false,
    )]);
    rig.idle(2);
    let check = |seen: &[(Z, u32)]| {
        assert!(
            seen.windows(2).all(|w| w[0].0 != w[1].0),
            "the pan does not move z₀ every frame"
        );
        assert!(
            seen.iter().all(|s| s.1 == depth0 + 1),
            "the pan is not one entry from its first frame"
        );
    };
    check(&seen);
    check_one_entry("the pan", depth0, rig.depth(), &rig.lines_from(rows));
    rejects("a pan applied only on release", || {
        check(&vec![(seen[0].0, depth0); FRAMES])
    });
}

#[test]
fn qa_mock_live_slider_drags_each_one_entry_and_one_line() {
    // The Slice centre slider and τ₁, each its own drag: one entry and one line each (the scrubber's and the pan's are
    // their own tests).
    for (id, mode) in [(SLIDERS[D], Touch::Tilt), (TAU1, Touch::Slice)] {
        let t = slider_drag(id, 0.5, 6.0, mode, (0.0, 0.0));
        check_one_entry(id, t.depth0, t.end_depth, &t.lines);
        // At most one edit a frame (the press and the ten moves), and one each frame the value moved.
        assert!(
            (FRAMES as u64..=FRAMES as u64 + 1).contains(&t.edits),
            "{id}: {} edits over a drag of {FRAMES} moves",
            t.edits
        );
    }
}
