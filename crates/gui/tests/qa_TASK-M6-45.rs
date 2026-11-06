//! QA tests for TASK-M6-45, written from R-466 and its seven requirements, not from the implementation:
//!
//! - REQ-GUI-224 (the definition, §G7's table): the mock's presets carry the table's base point, centre and window.
//! - REQ-GUI-218 (K2, "at every tilt"): over every preset, every pair of targets its Tilt offers, and tilts, rotations
//!   and scales across their spans, ends included, each reset sets its section's values and only them, read in the
//!   preset's coordinates; the three in each of the six orders reach Home's view. The basis is built here from R-457
//!   E1's formula, not through the GUI's own, and the fixtures' `z₀` from nothing the code reads.
//! - REQ-GUI-222 (K11): dimming reads the preset's coordinates, never the tilted plane's: tilted, with `z₀` at the base
//!   point, Slice centre's and Navigate's resets stay dimmed while Tilt's and Home are live.
//! - REQ-GUI-219 (K1, K7): the Home key wherever the focus is, in places the task's tests leave out (Time's scrubber, a
//!   window's control, an open menu, the Stain page, the console's header and its log while navigating), Shift and Alt
//!   leaving it Home, the focus staying; after a mouse click, with the ring hidden; in a Slice centre number box being
//!   typed in, the caret, nothing reset; the Home button's tooltips, live and dimmed; and a window over the button:
//!   R-466 K7 makes it the one control over the figure, not a control over the windows, so a click on a window that
//!   lies over the corner is the window's.
//! - REQ-GUI-221 (K5, K10): locked as well as free, every way to a command is one entry, one undo restores `z₀` and the
//!   basis bit for bit, one redo sets them again.
//! - REQ-GUI-218 and K6 (the keyboard): Enter into a section, the arrows reach its reset last, Enter on it is its click,
//!   and a reset that dims keeps its place.
//! - REQ-GUI-224's zoom row, by the Chart's own preset edit: choosing a preset keeps `σ` and the row reads against the
//!   new window; the depth readout stays absolute.
//!
//! Each check runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-45` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::f64::consts::{FRAC_PI_2, PI};

use eframe::egui::{accesskit::Role, pos2, vec2, Event, Key, Modifiers, PointerButton, Pos2};
use eframe::egui::{Rect, Shape};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Plane, SimConfig, SliceFrame, LATENT_DIM};
use engine::contract::view_ui::{KeyMode, Mode, Window};
use gui::app::App;
use gui::capture::{self, PIXELS_PER_POINT, SIZE};
use gui::explore::manifold_view::chart::preset_edit;
use gui::explore::manifold_view::reset::{
    self, command, enabled, home, home_enabled, home_rect, Section, ALREADY_HOME, HOME_TOOLTIP,
    NO_PRESET, SECTIONS,
};
use gui::explore::manifold_view::{
    self as mv, basis_edit, lock_edit, slice_axis, target_choices, z0_edit, View, PRESETS,
    PRESET_HOMES,
};
use gui::explore::{top_bar, FIGURE};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::MockSide;
use gui::windows::{console, entry, run};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const NONE: Modifiers = Modifiers::NONE;
/// R-466 K11, K13: the tolerance of "at the preset's values".
const TOL: f64 = 1e-12;
/// A key no command has: it turns keyboard navigation on and does nothing else.
const UNBOUND: Key = Key::F12;

/// A row of §G7's table: the preset's axes, base point, centre and window.
type Row = ((usize, usize), Latent, (f64, f64), f64);

/// §G7's table, transcribed: each preset's axes, base point, centre and window.
const G7: [Row; 6] = {
    let main = [0.180, 0.410, 0.0, -0.227, 0.0, 0.312, 0.333, 0.333];
    [
        ((0, 1), main, (0.180, 0.410), 1.0),
        (
            (2, 3),
            [0.250, 0.500, 0.125, -0.250, 0.0625, 0.375, 0.250, -0.125],
            (0.125, -0.250),
            0.5,
        ),
        ((4, 5), main, (0.0, 0.312), 1.0),
        ((6, 7), main, (0.333, 0.333), 1.0),
        ((0, 6), main, (0.180, 0.333), 1.0),
        ((5, 6), main, (0.312, 0.333), 1.0),
    ]
};

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn e(i: usize, s: f64) -> Latent {
    let mut v = [0.0; LATENT_DIM];
    v[i] = s;
    v
}

/// R-457 E1's basis, written out here: `u₁ = cos τ₁ e_a + sin τ₁ e_X`, `u₂ = cos τ₂ e_b + sin τ₂ e_Y`, `q₁ = σ(cos γ u₁ +
/// sin γ u₂)`, `q₂ = σ(cos γ u₂ − sin γ u₁)`.
fn e1_basis(
    (a, b): (usize, usize),
    (x, y): (usize, usize),
    sigma: f64,
    (t1, t2, g): (f64, f64, f64),
) -> (Latent, Latent) {
    let (mut u1, mut u2) = ([0.0; LATENT_DIM], [0.0; LATENT_DIM]);
    u1[a] = t1.cos();
    u1[x] += t1.sin();
    u2[b] = t2.cos();
    u2[y] += t2.sin();
    let (mut q1, mut q2) = ([0.0; LATENT_DIM], [0.0; LATENT_DIM]);
    for i in 0..LATENT_DIM {
        q1[i] = sigma * (g.cos() * u1[i] + g.sin() * u2[i]);
        q2[i] = sigma * (g.cos() * u2[i] - g.sin() * u1[i]);
    }
    (q1, q2)
}

/// `edits` applied to `sim`: a reset and Home may edit only `z₀` and the basis, so any other field panics here, which
/// also holds the targets, H, V and D still.
fn apply(sim: &mut SimConfig, edits: &[SetField]) {
    for edit in edits {
        match &edit.edit {
            Edit::Sim(SimField::Z0(z0)) => sim.plane.z0 = *z0,
            Edit::Sim(SimField::Basis { q1, q2 }) => {
                sim.plane.q1 = *q1;
                sim.plane.q2 = *q2;
            }
            other => panic!("a reset edited {other:?}"),
        }
    }
}

fn near(a: &Latent, b: &Latent) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= TOL)
}

fn base_sim() -> SimConfig {
    let (mut app, mut h) = rig();
    let _ = h.frame(&mut app, Vec::new());
    app.snapshot().sim.clone()
}

/// `base` at preset `p`, the pose given, targets `(x, y)`, `z₀` off the base point in all eight coordinates.
fn pose(
    base: &SimConfig,
    p: usize,
    (x, y): (usize, usize),
    sigma: f64,
    angles: (f64, f64, f64),
) -> SimConfig {
    let mut sim = base.clone();
    let (a, b) = PRESETS[p];
    let (q1, q2) = e1_basis((a, b), (x, y), sigma, angles);
    sim.plane = Plane {
        z0: std::array::from_fn(|i| PRESET_HOMES[p].base[i] + 0.0625 * (i as f64 + 1.0)),
        q1,
        q2,
    };
    sim.slice.frame = SliceFrame {
        normal: u32::try_from(slice_axis(a, b)).expect("an index"),
        tilt_x: u32::try_from(x).expect("an index"),
        tilt_y: u32::try_from(y).expect("an index"),
    };
    sim
}

/// Whether `after` is what `section`'s reset of `sim` must leave (K2).
fn section_ok(
    sim: &SimConfig,
    after: &SimConfig,
    p: usize,
    (x, y): (usize, usize),
    sigma: f64,
    angles: (f64, f64, f64),
    section: Section,
) -> bool {
    let (a, b) = PRESETS[p];
    let home_at = PRESET_HOMES[p];
    let (z, was) = (&after.plane.z0, &sim.plane.z0);
    match section {
        Section::SliceCentre => {
            (0..LATENT_DIM).all(|i| {
                let want = if i == a || i == b {
                    was[i]
                } else {
                    home_at.base[i]
                };
                z[i].to_bits() == want.to_bits()
            }) && after.plane.q1 == sim.plane.q1
                && after.plane.q2 == sim.plane.q2
        }
        Section::Tilt => {
            z == was && near(&after.plane.q1, &e(a, sigma)) && near(&after.plane.q2, &e(b, sigma))
        }
        Section::Navigate => {
            let (q1, q2) = e1_basis((a, b), (x, y), home_at.window, angles);
            (0..LATENT_DIM).all(|i| {
                let want = if i == a || i == b {
                    home_at.base[i]
                } else {
                    was[i]
                };
                (z[i] - want).abs() <= TOL
            }) && near(&after.plane.q1, &q1)
                && near(&after.plane.q2, &q2)
        }
    }
}

/// One section's reset from `sim`, checked against K2.
fn check_section(
    sim: &SimConfig,
    p: usize,
    targets: (usize, usize),
    sigma: f64,
    angles: (f64, f64, f64),
    section: Section,
) -> Result<(), String> {
    let edits = command(sim, section);
    if section == Section::Tilt && angles == (0.0, 0.0, 0.0) {
        // At home already: dimmed, and it sends nothing (K10).
        return if edits.is_empty() {
            Ok(())
        } else {
            Err("Tilt at home sent edits".to_owned())
        };
    }
    if edits.is_empty() {
        return Err(format!("{section:?}: a reset that does nothing"));
    }
    let mut after = sim.clone();
    apply(&mut after, &edits);
    if !section_ok(sim, &after, p, targets, sigma, angles, section) {
        return Err(format!(
            "{section:?} moved what it must not: {:?}",
            after.plane
        ));
    }
    if enabled(&after, section) {
        return Err(format!("{section:?} still live after its reset"));
    }
    Ok(())
}

/// The sweep: `Err` names the first pose that fails.
fn sweep(
    base: &SimConfig,
    run: impl Fn(&SimConfig, usize, (usize, usize), f64, (f64, f64, f64)) -> Result<(), String>,
) -> Result<usize, String> {
    let taus = [-FRAC_PI_2, -1.1, -0.2, 0.0, 0.3, 1.3, FRAC_PI_2];
    let gammas = [-PI, -2.0, -0.4, 0.0, 0.7, 2.5, PI];
    let mut n = 0;
    for p in 0..PRESETS.len() {
        let (a, b) = PRESETS[p];
        let choices = target_choices(a, b);
        for &x in &choices {
            for &y in &choices {
                for &sigma in &[PRESET_HOMES[p].window / 8.0, PRESET_HOMES[p].window * 3.0] {
                    for &t1 in &taus {
                        for &t2 in &taus {
                            // Both tilted fully into one target: the two vectors coincide, no plane.
                            if x == y && t1.abs() == FRAC_PI_2 && t2.abs() == FRAC_PI_2 {
                                continue;
                            }
                            for &g in &gammas {
                                let angles = (t1, t2, g);
                                let sim = pose(base, p, (x, y), sigma, angles);
                                run(&sim, p, (x, y), sigma, angles).map_err(|why| {
                                    format!(
                                        "preset {p}, X {x}, Y {y}, σ {sigma}, {angles:?}: {why}"
                                    )
                                })?;
                                n += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(n)
}

fn rig() -> (App<MockSide>, Headless) {
    (
        App::new(MockSide::new(MockEngine::frozen(), None), FORMAT),
        Headless::new(SIZE, PIXELS_PER_POINT),
    )
}

struct Rig {
    app: App<MockSide>,
    h: Headless,
}

impl Rig {
    fn new(p: usize) -> Self {
        let (mut app, mut h) = rig();
        let _ = h.frame(&mut app, Vec::new());
        let (a, b) = PRESETS[p];
        let w = PRESET_HOMES[p].window;
        app.set_field(z0_edit(PRESET_HOMES[p].base));
        app.set_field(basis_edit((e(a, w), e(b, w))));
        let mut rig = Self { app, h };
        rig.idle(3);
        rig
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            let _ = self.h.frame(&mut self.app, Vec::new());
        }
    }

    fn sim(&self) -> &SimConfig {
        &self.app.snapshot().sim
    }

    fn depth(&self) -> u32 {
        self.app.snapshot().history.undo_depth
    }

    /// All three sections off home: `z₀` in all eight coordinates, the tilts on, the scale doubled. Directly through
    /// the contract, each edit its own entry.
    fn moved(&mut self) {
        let p = View::read(self.sim()).preset.expect("a preset");
        let (a, b) = PRESETS[p];
        let frame = self.sim().slice.frame;
        let (x, y) = (frame.tilt_x as usize, frame.tilt_y as usize);
        let w = PRESET_HOMES[p].window;
        let z0: Latent =
            std::array::from_fn(|i| PRESET_HOMES[p].base[i] + 0.03125 * (i as f64 + 1.0));
        self.app.set_field(z0_edit(z0));
        self.app.set_field(basis_edit(e1_basis(
            (a, b),
            (x, y),
            2.0 * w,
            (0.3, -0.2, 0.4),
        )));
        self.idle(3);
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        for events in capture::key(key, modifiers) {
            let _ = self.h.frame(&mut self.app, events);
        }
        self.idle(1);
    }

    /// The focus at `id`, keyboard navigation on.
    fn focus(&mut self, id: &str) {
        self.app.view.focus.path = self.app.keyboard.tree(self.app.view.mode).path_to(id);
        self.key(UNBOUND, NONE);
    }

    fn click(&mut self, at: Pos2) {
        for events in capture::click(at) {
            let _ = self.h.frame(&mut self.app, events);
        }
        self.idle(1);
    }

    fn click_scope(&mut self, id: &str) {
        let at = self
            .app
            .keyboard
            .place_of(id)
            .unwrap_or_else(|| panic!("{id} placed"))
            .rect
            .center();
        self.click(at);
    }

    fn canvas(&self) -> Rect {
        Layout::with_splits(self.h.screen(), PIXELS_PER_POINT, &self.app.splits).figure
    }

    fn button(&self) -> Rect {
        home_rect(self.canvas())
    }

    fn open(&mut self, window: Window) {
        self.idle(1);
        self.app.open_window(&self.h.ctx, window, None);
        self.idle(4);
    }

    fn drawn(&self, window: Window) -> Rect {
        self.app
            .keyboard
            .place_of(entry(window).scope)
            .unwrap_or_else(|| panic!("{window:?} drawn"))
            .rect
    }

    /// A pointer drag from `from` to `to`, released, then frames to settle.
    fn drag(&mut self, from: Pos2, to: Pos2) {
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: NONE,
        };
        let mut frames = vec![vec![Event::PointerMoved(from)], vec![button(from, true)]];
        for k in 1..=4 {
            frames.push(vec![Event::PointerMoved(
                from + (to - from) * (k as f32 / 4.0),
            )]);
        }
        frames.push(vec![button(to, false)]);
        for events in frames {
            let _ = self.h.frame(&mut self.app, events);
        }
        self.idle(4);
    }

    /// Asserts the preset's home view, within R-466's tolerance.
    fn at_home(&self, p: usize, what: &str) {
        let (a, b) = PRESETS[p];
        let w = PRESET_HOMES[p].window;
        let plane = &self.sim().plane;
        assert!(
            near(&plane.z0, &PRESET_HOMES[p].base),
            "{what}: z₀ {plane:?}"
        );
        assert!(near(&plane.q1, &e(a, w)), "{what}: q₁ {plane:?}");
        assert!(near(&plane.q2, &e(b, w)), "{what}: q₂ {plane:?}");
    }

    /// The tooltips showing after the pointer rests at `at`.
    fn tooltip(&mut self, at: Pos2) -> String {
        let _ = self.h.frame(&mut self.app, vec![Event::PointerMoved(at)]);
        let mut texts = String::new();
        for _ in 0..80 {
            let out = self.h.frame(&mut self.app, Vec::new());
            for clipped in &out.shapes {
                flatten(&clipped.shape, &mut texts);
            }
        }
        texts
    }
}

fn flatten(shape: &Shape, out: &mut String) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
        Shape::Text(text) => {
            out.push_str(text.galley.text());
            out.push('\n');
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-224: the table.

fn check_table(table: &[Row; 6]) {
    for (p, &(axes, base, centre, window)) in table.iter().enumerate() {
        assert_eq!(PRESETS[p], axes, "preset {p}'s axes");
        assert_eq!(PRESET_HOMES[p].base, base, "preset {p}'s base point");
        assert_eq!(
            PRESET_HOMES[p].centre(axes.0, axes.1),
            centre,
            "preset {p}'s centre"
        );
        assert_eq!(PRESET_HOMES[p].window, window, "preset {p}'s window");
    }
}

#[test]
fn qa_presets_carry_g7_table() {
    check_table(&G7);
    assert_ne!(PRESET_HOMES[0].base, PRESET_HOMES[1].base, "other values");
    let (mu1, mu2) = (PRESET_HOMES[0].base[6], PRESET_HOMES[0].base[7]);
    assert!(
        mu1 != 0.0 && mu2 != 0.0,
        "non-zero masses in the test preset"
    );
    // Control: a table a digit off.
    let mut wrong = G7;
    wrong[1].3 = 1.0;
    rejects("a window the table does not hold", || check_table(&wrong));
}

#[test]
fn qa_mock_opens_at_home() {
    let (mut app, mut h) = rig();
    let _ = h.frame(&mut app, Vec::new());
    assert!(!home_enabled(&app.snapshot().sim), "the mock opens at home");
    // Control: moved, it is not.
    let mut z0 = app.snapshot().sim.plane.z0;
    z0[2] += 0.25;
    app.set_field(z0_edit(z0));
    let _ = h.frame(&mut app, Vec::new());
    assert!(home_enabled(&app.snapshot().sim));
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-218: at every tilt.

#[test]
fn qa_each_reset_alone_over_every_preset_target_and_tilt() {
    let base = base_sim();
    let n = sweep(&base, |sim, p, targets, sigma, angles| {
        let view = View::read(sim);
        if view.preset != Some(p) {
            return Err(format!("the preset read {:?}", view.preset));
        }
        for section in SECTIONS {
            check_section(sim, p, targets, sigma, angles, section)?;
        }
        Ok(())
    })
    .unwrap_or_else(|why| panic!("{why}"));
    assert!(n > 100_000, "the sweep ran {n} poses");

    // Controls, each what a GUI with the fault would leave, from one tilted pose with `z_μ1` as Y.
    let (p, targets, sigma, angles) = (0, (5, 6), 2.0, (0.3, 0.4, 0.2));
    let sim = pose(&base, p, targets, sigma, angles);
    let good =
        |after: &SimConfig, section| section_ok(&sim, after, p, targets, sigma, angles, section);
    // A reset that does nothing.
    for section in SECTIONS {
        rejects("a reset that does nothing", || assert!(good(&sim, section)));
    }
    // K2's first reading: `z₀ ← z₀ + P⊥(b − z₀)`, across the tilted plane.
    let mut across = sim.clone();
    let (q1, q2) = (sim.plane.q1, sim.plane.q2);
    let dot = |u: &Latent, v: &Latent| u.iter().zip(v).map(|(a, b)| a * b).sum::<f64>();
    let d: Latent = std::array::from_fn(|i| PRESET_HOMES[p].base[i] - sim.plane.z0[i]);
    let (g11, g12, g22) = (dot(&q1, &q1), dot(&q1, &q2), dot(&q2, &q2));
    let (d1, d2) = (dot(&d, &q1), dot(&d, &q2));
    let det = g11 * g22 - g12 * g12;
    let (s, t) = ((g22 * d1 - g12 * d2) / det, (g11 * d2 - g12 * d1) / det);
    for i in 0..LATENT_DIM {
        across.plane.z0[i] += d[i] - s * q1[i] - t * q2[i];
    }
    rejects("a Slice centre reset across the tilted plane", || {
        assert!(good(&across, Section::SliceCentre))
    });
    // A reset to literal zero.
    let mut zeroed = sim.clone();
    for i in (0..LATENT_DIM).filter(|i| ![0, 1].contains(i)) {
        zeroed.plane.z0[i] = 0.0;
    }
    rejects("a Slice centre reset to literal zero", || {
        assert!(good(&zeroed, Section::SliceCentre))
    });
    // A reset that also resets another section.
    let mut both = sim.clone();
    apply(&mut both, &command(&sim, Section::Tilt));
    apply(&mut both, &command(&sim, Section::SliceCentre));
    rejects("a Tilt reset that also resets Slice centre", || {
        assert!(good(&both, Section::Tilt))
    });
}

#[test]
fn qa_three_resets_in_all_orders_are_home_over_every_preset_target_and_tilt() {
    const ORDERS: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let base = base_sim();
    let n = sweep(&base, |sim, p, _, _, _| {
        let (a, b) = PRESETS[p];
        let w = PRESET_HOMES[p].window;
        let at_home = |s: &SimConfig| {
            near(&s.plane.z0, &PRESET_HOMES[p].base)
                && near(&s.plane.q1, &e(a, w))
                && near(&s.plane.q2, &e(b, w))
        };
        for order in ORDERS {
            let mut s = sim.clone();
            for i in order {
                let edits = command(&s, SECTIONS[i]);
                apply(&mut s, &edits);
            }
            if !at_home(&s) {
                return Err(format!("{order:?} left {:?}", s.plane));
            }
            if home_enabled(&s) {
                return Err(format!("{order:?}: Home live at home"));
            }
        }
        let mut s = sim.clone();
        apply(&mut s, &home(sim));
        if !at_home(&s) {
            return Err(format!("Home left {:?}", s.plane));
        }
        Ok(())
    })
    .unwrap_or_else(|why| panic!("{why}"));
    assert!(n > 100_000, "the sweep ran {n} poses");
    // Control: two of the three do not reach it.
    let sim = pose(&base, 1, (0, 6), 0.25, (0.3, 0.2, 0.1));
    let mut s = sim.clone();
    for section in [Section::Tilt, Section::Navigate] {
        let edits = command(&s, section);
        apply(&mut s, &edits);
    }
    rejects("two resets taken for Home", || {
        assert!(near(&s.plane.z0, &PRESET_HOMES[1].base))
    });
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-222: the preset's coordinates, never the row's figures.

#[test]
fn qa_dimming_tilted_reads_the_preset_coordinates() {
    let base = base_sim();
    let w = PRESET_HOMES[1].window;
    let mut sim = pose(&base, 1, (0, 6), w, (0.4, -0.3, 0.2));
    sim.plane.z0 = PRESET_HOMES[1].base;
    assert!(
        !enabled(&sim, Section::SliceCentre),
        "z₀ is at the base point"
    );
    assert!(
        !enabled(&sim, Section::Navigate),
        "z₀ at the centre, σ at the window"
    );
    assert!(enabled(&sim, Section::Tilt));
    assert!(home_enabled(&sim));
    // Control: the same pose in the plane's coordinates has `z₀` moved off the plane's centre by nothing the check saw;
    // a coordinate on an axis moved lights Navigate alone, one off them Slice centre alone.
    let mut moved = sim.clone();
    moved.plane.z0[2] += 1e-9;
    assert!(enabled(&moved, Section::Navigate));
    assert!(!enabled(&moved, Section::SliceCentre));
    let mut moved = sim.clone();
    moved.plane.z0[6] += 1e-9;
    assert!(enabled(&moved, Section::SliceCentre));
    assert!(!enabled(&moved, Section::Navigate));
    // A window of 0.5: the zoom compared as log₂(σ / σ_h), on both sides of K11's tolerance.
    let scaled = |octaves: f64| {
        let mut s = base.clone();
        let (a, b) = PRESETS[1];
        let sigma = w * octaves.exp2();
        s.plane = Plane {
            z0: PRESET_HOMES[1].base,
            q1: e(a, sigma),
            q2: e(b, sigma),
        };
        s
    };
    assert!(!enabled(&scaled(5e-13), Section::Navigate));
    assert!(enabled(&scaled(5e-12), Section::Navigate));
    rejects("σ = 1 taken for the window of a preset of 0.5", || {
        assert!(!enabled(&scaled(1.0), Section::Navigate))
    });
}

#[test]
fn qa_no_preset_every_command_is_empty() {
    let mut sim = base_sim();
    // A basis out of every preset's frame: all eight coordinates in both vectors.
    sim.plane.q1 = std::array::from_fn(|i| 0.1 + 0.01 * i as f64);
    sim.plane.q2 = std::array::from_fn(|i| 0.3 - 0.02 * i as f64);
    assert_eq!(View::read(&sim).preset, None, "no preset holds it");
    for section in SECTIONS {
        assert!(!enabled(&sim, section));
        assert!(command(&sim, section).is_empty());
    }
    assert!(!home_enabled(&sim));
    assert!(home(&sim).is_empty());
    assert_eq!(reset::dimmed_because(&sim), NO_PRESET);
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-219: the Home key, wherever the focus is.

/// The Home key from `id`, `enter` going in first, on a moved rig: the home view, the focus and the mode unchanged.
fn home_from(setup: impl Fn(&mut Rig), id: &str, enter: bool, modifiers: Modifiers) {
    let mut rig = Rig::new(0);
    setup(&mut rig);
    rig.moved();
    rig.focus(id);
    if enter {
        rig.key(Key::Enter, NONE);
        assert_eq!(rig.app.view.focus.mode, KeyMode::Interaction, "{id}");
    }
    let (path, mode) = (rig.app.view.focus.path.clone(), rig.app.view.focus.mode);
    let depth = rig.depth();
    rig.key(Key::Home, modifiers);
    rig.at_home(0, id);
    assert_eq!(rig.depth(), depth + 1, "{id}: one entry");
    assert_eq!(rig.app.view.focus.path, path, "{id}: the focus stays");
    assert_eq!(rig.app.view.focus.mode, mode, "{id}: the mode stays");
}

#[test]
fn qa_home_key_on_elements_of_every_kind() {
    let plain = |_: &mut Rig| {};
    let run_open = |rig: &mut Rig| rig.open(Window::Run);
    let console_open = |rig: &mut Rig| rig.open(Window::Console);
    home_from(plain, gui::explore::time::SCRUBBER, true, NONE);
    home_from(plain, mv::ZOOM, true, NONE);
    home_from(plain, mv::TILT_X, false, NONE);
    home_from(run_open, run::T_HORIZON, false, NONE);
    home_from(run_open, run::T_HORIZON, true, NONE);
    home_from(console_open, console::HEADER, false, NONE);
    home_from(console_open, console::LOG, false, NONE);
    // Shift and Alt leave it Home.
    home_from(plain, FIGURE, false, Modifiers::SHIFT);
    home_from(plain, mv::SLIDERS[2], true, Modifiers::ALT);
    // Control: End and Page Up are no Home.
    let mut rig = Rig::new(0);
    rig.moved();
    let before = rig.sim().plane.clone();
    rig.focus(FIGURE);
    rig.key(Key::End, NONE);
    rig.key(Key::PageUp, NONE);
    assert_eq!(rig.sim().plane, before);
    rejects("a Home key that did nothing", || {
        rig.key(Key::PageDown, NONE);
        rig.at_home(0, "Page Down")
    });
}

#[test]
fn qa_home_key_with_a_menu_open_and_on_the_stain_page() {
    // View's menu open from the keyboard.
    let mut rig = Rig::new(0);
    rig.moved();
    rig.focus("view");
    rig.key(Key::Enter, NONE);
    rig.key(Key::Home, NONE);
    rig.at_home(0, "View's menu open");
    // On the Stain page, navigating and interacting.
    for enter in [false, true] {
        let mut rig = Rig::new(1);
        rig.moved();
        rig.app.view.mode = Mode::Stain;
        rig.idle(3);
        rig.focus(gui::stain::CANVAS);
        if enter {
            rig.key(Key::Enter, NONE);
        }
        rig.key(Key::Home, NONE);
        rig.at_home(1, "the Stain page");
    }
}

#[test]
fn qa_home_key_after_a_mouse_click_acts_and_shows_the_ring() {
    let mut rig = Rig::new(0);
    rig.moved();
    let beside = rig.button().center() - vec2(90.0, -40.0);
    rig.click(beside);
    assert!(!rig.app.keyboard.navigating(), "the click hid the ring");
    rig.key(Key::Home, NONE);
    rig.at_home(0, "after a click");
    assert!(rig.app.keyboard.navigating(), "the key shows the ring (K1)");
}

#[test]
fn qa_home_key_in_a_number_box_moves_the_caret() {
    let mut rig = Rig::new(0);
    rig.moved();
    let row = rig
        .app
        .keyboard
        .place_of(mv::SLIDERS[2])
        .expect("placed")
        .rect;
    let out = rig.h.frame(&mut rig.app, Vec::new());
    let update = out.platform_output.accesskit_update.expect("a tree");
    let at = update
        .nodes
        .iter()
        .filter(|(_, n)| n.role() == Role::SpinButton)
        .filter_map(|(_, n)| n.bounds())
        .map(|b| pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32))
        .find(|c| (c.y - row.center().y).abs() < row.height() && c.x > row.min.x)
        .expect("slider 2's number box");
    rig.click(at);
    let select_all = Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    };
    let _ = rig.h.frame(&mut rig.app, vec![select_all]);
    let _ = rig
        .h
        .frame(&mut rig.app, vec![Event::Text(".25".to_owned())]);
    let plane = rig.sim().plane.clone();
    rig.key(Key::Home, NONE);
    assert_eq!(rig.sim().plane.q1, plane.q1, "nothing reset");
    let _ = rig.h.frame(&mut rig.app, vec![Event::Text("-".to_owned())]);
    rig.key(Key::Enter, NONE);
    rig.idle(2);
    // Control: the caret at the start reads −0.25; a Home that left the caret reads ".25-", no number at all.
    assert_eq!(rig.sim().plane.z0[2], -0.25, "the caret went to the start");
    assert_ne!(rig.sim().plane.z0[2], PRESET_HOMES[0].base[2]);
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-219, K7: the button.

#[test]
fn qa_home_button_tooltips() {
    let mut rig = Rig::new(0);
    let at = rig.button().center();
    let texts = rig.tooltip(at);
    assert!(texts.contains(ALREADY_HOME), "dimmed at home: {texts}");
    // No preset: the tooltip says so.
    let q1: Latent = std::array::from_fn(|i| 0.1 + 0.01 * i as f64);
    let q2: Latent = std::array::from_fn(|i| 0.3 - 0.02 * i as f64);
    let mut rig = Rig::new(0);
    rig.app.set_field(basis_edit((q1, q2)));
    rig.idle(3);
    let texts = rig.tooltip(at);
    assert!(texts.contains(NO_PRESET), "no preset: {texts}");
    // Live.
    let mut rig = Rig::new(0);
    rig.moved();
    let texts = rig.tooltip(at);
    assert!(texts.contains(HOME_TOOLTIP), "live: {texts}");
    rejects("a live button read as dimmed", || {
        assert!(texts.contains(ALREADY_HOME))
    });
}

/// A window lying over the button's corner, which is not the figure: the click is the window's.
#[test]
fn qa_home_button_does_not_take_a_click_on_a_window_over_it() {
    let mut rig = Rig::new(0);
    rig.moved();
    rig.open(Window::Run);
    let corner = rig.button().center();
    let r = rig.drawn(Window::Run);
    let from = pos2(r.min.x + 80.0, r.min.y + 14.0);
    rig.drag(from, from + (corner - r.center()));
    let r = rig.drawn(Window::Run);
    assert!(r.expand(-4.0).contains(corner), "{r:?} covers {corner:?}");
    let (plane, depth) = (rig.sim().plane.clone(), rig.depth());
    rig.click(corner);
    assert_eq!(rig.sim().plane, plane, "the click was the window's");
    assert_eq!(rig.depth(), depth, "no command");
    // Control: with the window closed, the same click is Home.
    rig.app.close_window(&rig.h.ctx, Window::Run);
    rig.idle(3);
    rig.click(corner);
    rig.at_home(0, "the button uncovered");
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-221: one entry, every way, locked and free.

type Way = (&'static str, fn(&mut Rig));

fn ways() -> Vec<Way> {
    vec![
        ("Slice centre's reset", |rig| {
            rig.click_scope(Section::SliceCentre.scope())
        }),
        ("Tilt's reset", |rig| rig.click_scope(Section::Tilt.scope())),
        ("Navigate's reset", |rig| {
            rig.click_scope(Section::Navigate.scope())
        }),
        ("the Home button", |rig| {
            let at = rig.button().center();
            rig.click(at);
        }),
        ("the Home key", |rig| rig.key(Key::Home, NONE)),
        ("View › Home", |rig| {
            rig.focus(top_bar::HOME.0);
            rig.key(Key::Enter, NONE);
        }),
    ]
}

fn one_entry(locked: bool) {
    for (what, way) in ways() {
        let mut rig = Rig::new(1);
        rig.moved();
        if locked {
            // The pin off the centre, so the lock's rule has work to do.
            let plane = rig.sim().plane.clone();
            let z_locked: Latent =
                std::array::from_fn(|i| plane.z0[i] + 0.25 * plane.q1[i] - 0.5 * plane.q2[i]);
            rig.app.set_field(lock_edit(Lock {
                locked: true,
                z_locked,
            }));
            rig.idle(3);
        }
        let (before, lock, depth) = (rig.sim().plane.clone(), rig.sim().lock.clone(), rig.depth());
        way(&mut rig);
        let after = rig.sim().plane.clone();
        assert_ne!(after, before, "{what} acted");
        assert_eq!(rig.depth(), depth + 1, "{what}: one entry");
        assert_eq!(rig.sim().lock, lock, "{what}: the lock stays");
        rig.app.undo();
        rig.idle(3);
        assert_eq!(rig.depth(), depth, "{what}: one undo");
        assert_eq!(rig.sim().plane, before, "{what}: restored bit for bit");
        rig.app.redo();
        rig.idle(3);
        assert_eq!(rig.sim().plane, after, "{what}: redo sets it again");
    }
}

#[test]
fn qa_each_way_is_one_entry_free() {
    one_entry(false);
}

#[test]
fn qa_each_way_is_one_entry_locked() {
    one_entry(true);
}

#[test]
fn qa_one_entry_control_two_entries_fail() {
    let mut rig = Rig::new(1);
    rig.moved();
    let depth = rig.depth();
    // A Home sent as two entries, by hand.
    let sim = rig.sim().clone();
    for edit in home(&sim) {
        rig.app.set_field(edit);
    }
    rig.idle(3);
    rejects("a command of two entries", || {
        assert_eq!(rig.depth(), depth + 1)
    });
}

// ---------------------------------------------------------------------------------------------------------------------
// K6: the keyboard reaches each reset, last.

#[test]
fn qa_each_reset_is_reached_last_and_enter_is_its_click() {
    for section in SECTIONS {
        let id = section.scope();
        let mut rig = Rig::new(0);
        rig.moved();
        let parent = match section {
            Section::SliceCentre => mv::SLICE_CENTRE,
            Section::Tilt => mv::TILT,
            Section::Navigate => mv::NAVIGATE,
        };
        rig.focus(parent);
        rig.key(Key::Enter, NONE);
        let first = rig.app.view.focus.path.last().map(String::as_str);
        assert_ne!(
            first,
            Some(id),
            "Enter lands on the section's first element"
        );
        let mut steps = 0;
        while rig.app.view.focus.path.last().map(String::as_str) != Some(id) {
            steps += 1;
            assert!(steps < 20, "{section:?}: the arrows never reach its reset");
            rig.key(Key::ArrowDown, NONE);
        }
        rig.key(Key::ArrowDown, NONE);
        assert_eq!(
            rig.app.view.focus.path.last().map(String::as_str),
            Some(id),
            "{section:?}: nothing after the reset"
        );
        rig.key(Key::Enter, NONE);
        assert!(!enabled(rig.sim(), section), "{section:?}: Enter acted");
        // Dimmed, it keeps its place, and Enter on it is nothing.
        let (plane, depth) = (rig.sim().plane.clone(), rig.depth());
        assert_eq!(rig.app.view.focus.path.last().map(String::as_str), Some(id));
        rig.key(Key::Enter, NONE);
        assert_eq!(rig.sim().plane, plane);
        assert_eq!(rig.depth(), depth);
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-224: the zoom row against the window, the preset chosen on the Chart.

#[test]
fn qa_choosing_a_preset_keeps_sigma_and_home_reads_its_window() {
    let mut rig = Rig::new(0);
    let sim = rig.sim().clone();
    let view = View::read(&sim);
    rig.app.set_field(preset_edit(&sim, view, 1));
    rig.idle(3);
    let view = View::read(rig.sim());
    assert_eq!(view.preset, Some(1));
    assert_eq!(view.scale, 1.0, "§G2: choosing a preset keeps σ");
    assert_eq!(view.zoom_row(), -1.0, "log₂(σ_h / σ) = log₂ 0.5");
    assert_eq!(view.zoom(), 0.0, "the depth stays absolute");
    assert!(
        enabled(rig.sim(), Section::Navigate),
        "off this preset's window"
    );
    rig.key(Key::Home, NONE);
    rig.at_home(1, "the second preset");
    let view = View::read(rig.sim());
    assert_eq!(view.zoom_row(), 0.0, "× 1.00 at the window");
    assert_eq!(view.zoom(), 1.0, "the depth reads −log₂ 0.5");
    // Control: the first preset's window is no home of this one.
    rejects("σ = 1 as the second preset's home", || {
        assert_eq!(view.scale, 1.0)
    });
}
