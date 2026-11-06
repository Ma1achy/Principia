//! REQ-GUI-218, REQ-GUI-221, REQ-GUI-222 and REQ-GUI-223 (R-466): each of Slice centre, Tilt and Navigate has a reset
//! returning that section, and only it, to the active chart preset's own values, read in the preset's coordinates; each
//! is one undoable command, dimmed at home, and drives the compass live, switching no mode. Scripted on the mock, each
//! check also shown failing on what a GUI with the fault would leave (R-176, through `rejects`; gui's tests have no
//! `negative_control!`, `support`'s note).

use eframe::egui::{self, accesskit::Role, Event, Key, Modifiers, PointerButton, Pos2};
use engine::contract::sim_config::{Latent, Lock, Plane, LATENT_DIM};

use super::compass_support::{drawing, settle};
use super::mock_manifold_view::{in_rect, panel_geometry, type_at};
use super::support::{
    click_scope, focus, frame_texts, headless, keys_on, mock_app, press, rejects,
};
use crate::app::App;
use crate::explore::compass::readout;
use crate::explore::manifold_view::navigate::magnification;
use crate::explore::manifold_view::reset::{
    self, at, command, dimmed_because, enabled, home_enabled, Section, ALREADY_HOME,
    HOME_TOLERANCE, NO_PRESET, SECTIONS,
};
use crate::explore::manifold_view::{
    self as mv, axis, basis_edit, lock_edit, row, z0_edit, Touch, View, PRESETS, PRESET_HOMES,
    TAU1, TILT_Y_ENTRIES,
};
use crate::explore::FIGURE;
use crate::headless::Headless;
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
/// `z_μ1`'s index.
pub(super) const MU1: usize = 6;
/// The script's two presets: the test preset, and the second, of other home values.
pub(super) const SCRIPT_PRESETS: [usize; 2] = [0, 1];

/// One app on the mock, and its headless context.
pub(super) type Rig = (App<MockSide>, Headless);

/// The app at preset `p`'s home view.
pub(super) fn at_preset(p: usize) -> Rig {
    let mut rig = (mock_app(), headless());
    let (a, b) = PRESETS[p];
    let home = PRESET_HOMES[p];
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    rig.0.set_field(z0_edit(home.base));
    rig.0
        .set_field(basis_edit((axis(a, home.window), axis(b, home.window))));
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    settle(&mut rig.1, &mut rig.0);
    rig
}

/// Arrows on control `id`, in interaction mode from navigation: `n` presses of `key` with `modifiers`, then Esc.
pub(super) fn step(rig: &mut Rig, id: &str, key: Key, modifiers: Modifiers, n: usize) {
    let (app, h) = (&mut rig.0, &mut rig.1);
    app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(id);
    keys_on(h, app);
    press(h, app, Key::Enter, NONE);
    for _ in 0..n {
        press(h, app, key, modifiers);
    }
    press(h, app, Key::Escape, NONE);
}

/// Types `value` into the number box of slider row `i` of Slice centre.
fn type_slice_box(rig: &mut Rig, i: usize, value: f64) {
    let (_, _, rh, [_, _, centre, _]) = panel_geometry(&rig.1);
    let nodes = super::support::nodes(&mut rig.1, &mut rig.0);
    let at = in_rect(&nodes, row(centre, rh, 1 + i), Role::SpinButton)[0]
        .rect
        .center();
    type_at(&mut rig.1, &mut rig.0, at, &format!("{value}"));
}

/// Moves Slice centre: a slider, and the number box of `z_μ1`.
pub(super) fn move_slice(rig: &mut Rig) {
    let View { a, b, .. } = View::read(&rig.0.snapshot().sim);
    let off = (0..LATENT_DIM)
        .find(|i| *i != a && *i != b)
        .expect("six axes off the plane");
    step(rig, mv::SLIDERS[off], Key::ArrowUp, SHIFT, 2);
    type_slice_box(rig, MU1, 0.625);
}

/// Moves Tilt: `Y` to `z_μ1`, and `τ₁`, `τ₂` and `γ` off 0.
pub(super) fn move_tilt(rig: &mut Rig) {
    let (app, h) = (&mut rig.0, &mut rig.1);
    app.view.focus.path = app
        .keyboard
        .tree(app.view.mode)
        .path_to(TILT_Y_ENTRIES[MU1]);
    keys_on(h, app);
    press(h, app, Key::Enter, NONE);
    step(rig, TAU1, Key::ArrowUp, SHIFT, 3);
    step(rig, mv::TAU2, Key::ArrowDown, SHIFT, 2);
    step(rig, mv::GAMMA, Key::ArrowUp, SHIFT, 1);
}

/// Moves Navigate: a pan and a zoom.
pub(super) fn move_navigate(rig: &mut Rig) {
    step(rig, FIGURE, Key::ArrowRight, NONE, 3);
    step(rig, mv::ZOOM, Key::ArrowUp, NONE, 3);
}

/// Moves all three sections, a mass tilted into the plane.
pub(super) fn move_all(rig: &mut Rig) {
    move_slice(rig);
    move_tilt(rig);
    move_navigate(rig);
}

/// The three sections' values, read in preset `p`'s coordinates (R-466 K2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Values {
    /// `z₀`'s six coordinates off the preset's axes, the two on them zeroed.
    pub slice: Latent,
    /// `τ₁`, `τ₂` and `γ`.
    pub tilt: [f64; 3],
    /// `z₀`'s two coordinates on the axes, and `log₂ σ`.
    pub navigate: [f64; 3],
}

impl Values {
    /// Of a plane's `z₀` on preset `p`, read as `view`.
    fn of(p: usize, z0: &Latent, view: &View) -> Self {
        let (a, b) = PRESETS[p];
        let mut slice = *z0;
        (slice[a], slice[b]) = (0.0, 0.0);
        Self {
            slice,
            tilt: [view.tau1, view.tau2, view.gamma],
            navigate: [z0[a], z0[b], view.scale.log2()],
        }
    }

    /// `app`'s.
    pub fn read(app: &App<MockSide>, p: usize) -> Self {
        let sim = &app.snapshot().sim;
        let view = View::read(sim);
        assert_eq!(view.preset, Some(p), "the preset is still the one read");
        Self::of(p, &sim.plane.z0, &view)
    }

    /// Preset `p`'s home values.
    pub fn home(p: usize) -> Self {
        let home = PRESET_HOMES[p];
        let (a, b) = PRESETS[p];
        let view = View {
            preset: Some(p),
            a,
            b,
            d: 0,
            x: 0,
            y: 0,
            scale: home.window,
            tau1: 0.0,
            tau2: 0.0,
            gamma: 0.0,
        };
        Self::of(p, &home.base, &view)
    }

    /// `section`'s values.
    fn of_section(&self, section: Section) -> Vec<f64> {
        match section {
            Section::SliceCentre => self.slice.to_vec(),
            Section::Tilt => self.tilt.to_vec(),
            Section::Navigate => self.navigate.to_vec(),
        }
    }
}

/// Asserts `a` and `b` agree within 10⁻¹²: `what`.
fn close(a: &[f64], b: &[f64], what: &str) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() <= 1e-12, "{what}: {a:?} against {b:?}");
    }
}

/// Asserts `after` is what `section`'s reset leaves from `before` on preset `p`: its section at home, the other two as
/// they were.
fn check_reset(section: Section, before: &Values, after: &Values, home: &Values) {
    close(
        &after.of_section(section),
        &home.of_section(section),
        "the section reads the preset's values",
    );
    for other in SECTIONS.into_iter().filter(|s| *s != section) {
        close(
            &after.of_section(other),
            &before.of_section(other),
            "another section reads as before",
        );
    }
}

/// Asserts `values` are preset `p`'s home view.
pub(super) fn check_home(values: &Values, p: usize) {
    let home = Values::home(p);
    for section in SECTIONS {
        close(
            &values.of_section(section),
            &home.of_section(section),
            "the home view",
        );
    }
}

#[test]
fn mock_manifold_resets_each_returns_its_section_alone() {
    for p in SCRIPT_PRESETS {
        for section in SECTIONS {
            let mut rig = at_preset(p);
            move_all(&mut rig);
            let before = Values::read(&rig.0, p);
            let sim = rig.0.snapshot().sim.clone();
            let home = Values::home(p);
            for s in SECTIONS {
                assert_ne!(
                    before.of_section(s),
                    home.of_section(s),
                    "preset {p}: {s:?} moved"
                );
            }
            click_scope(&mut rig.1, &mut rig.0, section.scope());
            let after = Values::read(&rig.0, p);
            check_reset(section, &before, &after, &home);
            let left = &rig.0.snapshot().sim;
            assert_eq!(left.slice, sim.slice, "the targets and H, V, D kept");

            // Controls, on what a GUI with the fault would leave.
            rejects("a reset that does nothing", || {
                check_reset(section, &before, &before, &home)
            });
            let zero = Values {
                slice: [0.0; 8],
                tilt: [0.0; 3],
                navigate: [0.0; 3],
            };
            if section != Section::Tilt {
                rejects("a reset to literal zero", || {
                    check_reset(section, &before, &zero, &home)
                });
            }
            let other = SECTIONS.into_iter().find(|s| *s != section).unwrap();
            let mut too_far = after;
            match other {
                Section::SliceCentre => too_far.slice = home.slice,
                Section::Tilt => too_far.tilt = home.tilt,
                Section::Navigate => too_far.navigate = home.navigate,
            }
            rejects("a reset that resets another section", || {
                check_reset(section, &before, &too_far, &home)
            });
            let first = Values::home(0);
            if p != 0 && section != Section::Tilt {
                rejects("a GUI hardcoding the first preset's values", || {
                    let mut fixed = after;
                    match section {
                        Section::SliceCentre => fixed.slice = first.slice,
                        _ => fixed.navigate = first.navigate,
                    }
                    check_reset(section, &before, &fixed, &home)
                });
            }
        }
    }
}

/// `z₀ + P⊥(b − z₀)`: K2's first reading of a Slice centre reset, across the tilted plane.
fn across_tilted_plane(z0: &Latent, q1: &Latent, q2: &Latent, b: &Latent) -> Latent {
    let d: Latent = std::array::from_fn(|i| b[i] - z0[i]);
    let (h1, h2) = (q1.map(|x| x / mv::norm(q1)), q2);
    let e2 = mv::add(h2, -mv::dot(h2, &h1), &h1);
    let e2 = e2.map(|x| x / mv::norm(&e2));
    let in_plane = mv::add(&h1.map(|x| x * mv::dot(&d, &h1)), mv::dot(&d, &e2), &e2);
    std::array::from_fn(|i| z0[i] + d[i] - in_plane[i])
}

#[test]
fn mock_manifold_resets_slice_centre_is_not_across_the_tilted_plane() {
    let mut rig = at_preset(0);
    move_all(&mut rig);
    let sim = rig.0.snapshot().sim.clone();
    let (q1, q2) = (sim.plane.q1, sim.plane.q2);
    let before = Values::read(&rig.0, 0);
    click_scope(&mut rig.1, &mut rig.0, Section::SliceCentre.scope());
    let mu = MU1;
    let base = PRESET_HOMES[0].base;
    assert_eq!(
        rig.0.snapshot().sim.plane.z0[mu],
        base[mu],
        "the mass tilted into the plane is at the base point's"
    );
    // Control: across the tilted plane, the mass tilted into it stays off the base point.
    let view = View::read(&sim);
    let wrong = across_tilted_plane(&sim.plane.z0, &q1, &q2, &base);
    rejects("a Slice centre reset across the tilted plane", || {
        check_reset(
            Section::SliceCentre,
            &before,
            &Values::of(0, &wrong, &view),
            &Values::home(0),
        )
    });
}

#[test]
fn mock_manifold_resets_in_any_order_compose_to_home() {
    use Section::{Navigate, SliceCentre, Tilt};
    let orders = [
        [SliceCentre, Tilt, Navigate],
        [SliceCentre, Navigate, Tilt],
        [Tilt, SliceCentre, Navigate],
        [Tilt, Navigate, SliceCentre],
        [Navigate, SliceCentre, Tilt],
        [Navigate, Tilt, SliceCentre],
    ];
    for p in SCRIPT_PRESETS {
        for order in orders {
            let mut rig = at_preset(p);
            move_all(&mut rig);
            for section in order {
                click_scope(&mut rig.1, &mut rig.0, section.scope());
            }
            check_home(&Values::read(&rig.0, p), p);
            let plane = &rig.0.snapshot().sim.plane;
            let (a, b) = PRESETS[p];
            let window = PRESET_HOMES[p].window;
            assert_eq!(plane.q1, axis(a, window), "untilted, bit for bit");
            assert_eq!(plane.q2, axis(b, window), "untilted, bit for bit");
            assert_eq!(plane.z0, PRESET_HOMES[p].base, "bit for bit");
        }
    }
    // Control: two of the three leave the third off home.
    let mut rig = at_preset(0);
    move_all(&mut rig);
    click_scope(&mut rig.1, &mut rig.0, SliceCentre.scope());
    click_scope(&mut rig.1, &mut rig.0, Tilt.scope());
    rejects("two of the three resets", || {
        check_home(&Values::read(&rig.0, 0), 0)
    });
}

#[test]
fn mock_manifold_resets_read_the_preset_never_literal_zero() {
    for p in SCRIPT_PRESETS {
        let (a, b) = PRESETS[p];
        let home = PRESET_HOMES[p];
        let (u, v) = home.centre(a, b);
        assert!(u != 0.0 || v != 0.0, "preset {p}: a centre off (0, 0)");
        let masses = [home.base[MU1], home.base[MU1 + 1]];
        assert!(masses.iter().all(|m| *m != 0.0), "preset {p}: masses");
    }
    assert_ne!(
        PRESET_HOMES[0], PRESET_HOMES[1],
        "a second preset of other home values"
    );
    assert_eq!(
        PRESET_HOMES[0].base,
        crate::mock::MOCK_Z0,
        "the mock opens at the test preset's home"
    );
}

/// The zoom row reads against the window, the depth stays absolute, and a drag or a typed zoom lands against it.
#[test]
fn mock_manifold_resets_zoom_row_reads_the_window() {
    let rig = at_preset(1);
    let view = View::read(&rig.0.snapshot().sim);
    assert_eq!(view.window(), 0.5);
    assert_eq!(view.zoom_row(), 0.0, "× 1.00 at the window");
    assert_eq!(view.zoom(), 1.0, "the absolute zoom is −log₂ σ");
    let in_two = view.with_zoom_row(2.0);
    assert_eq!(in_two.scale, 0.125);
    assert_eq!(in_two.zoom_row(), 2.0);
    let free = View {
        preset: None,
        ..view
    };
    assert_eq!(free.window(), 1.0, "no preset, no window");
    assert_eq!(free.zoom_row(), free.zoom());
    let mut rig = at_preset(1);
    let texts = frame_texts(&mut rig.1, &mut rig.0);
    assert!(texts.iter().any(|t| t == "× 1.00"), "the row at the window");
    // Control: read absolutely, the row would say × 2.00 at this window.
    rejects("a zoom row reading −log₂ σ", || {
        assert_eq!(magnification(view.zoom()), "× 1.00")
    });
}

#[test]
fn mock_manifold_resets_at_tolerance_boundary() {
    assert!(at(HOME_TOLERANCE, 0.0), "at 10⁻¹² is home");
    assert!(at(-HOME_TOLERANCE, 0.0), "either side");
    assert!(!at(2.0 * HOME_TOLERANCE, 0.0), "past it is not");
    assert!(!at(f64::NAN, 0.0), "no value is no home");
}

#[test]
fn mock_manifold_resets_scopes_keep_the_order() {
    let rig = at_preset(0);
    let tree = rig.0.keyboard.tree(rig.0.view.mode);
    for (scope, section) in [
        (mv::NAVIGATE, Section::Navigate),
        (mv::SLICE_CENTRE, Section::SliceCentre),
        (mv::TILT, Section::Tilt),
    ] {
        let children = tree.children(Some(scope));
        assert_eq!(children.last(), Some(&section.scope()), "last of {scope}");
        assert_eq!(Section::of(section.scope()), Some(section));
    }
    assert_eq!(Section::of(mv::TAU1), None);
}

#[test]
fn mock_manifold_resets_enter_acts_as_the_click() {
    for section in SECTIONS {
        let mut rig = at_preset(0);
        move_all(&mut rig);
        let before = Values::read(&rig.0, 0);
        let (app, h) = (&mut rig.0, &mut rig.1);
        app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(section.scope());
        keys_on(h, app);
        let at_reset = focus(app).last().copied().map(str::to_owned);
        press(h, app, Key::Enter, NONE);
        check_reset(section, &before, &Values::read(&rig.0, 0), &Values::home(0));
        assert_eq!(
            focus(&rig.0).last().copied().map(str::to_owned),
            at_reset,
            "the focus stays"
        );
    }
}

/// Locked, the edits go through the lock's rule (K5): a Tilt reset and a Navigate reset's zoom keep the anchor's chart
/// point, moving `z₀`, the lock and its anchor staying.
#[test]
fn mock_manifold_resets_locked_turn_about_the_pin() {
    for section in [Section::Tilt, Section::Navigate] {
        let mut rig = at_preset(0);
        move_tilt(&mut rig);
        move_navigate(&mut rig);
        let sim = rig.0.snapshot().sim.clone();
        let anchor = mv::point(&sim.plane, 0.75, 0.25);
        let lock = Lock {
            locked: true,
            z_locked: anchor,
        };
        rig.0.set_field(lock_edit(lock.clone()));
        let _ = rig.1.frame(&mut rig.0, Vec::new());
        let before = rig.0.snapshot().sim.clone();
        // Navigate pans first, moving where the pin falls, and then zooms about it.
        let panned = if section == Section::Navigate {
            let mut z0 = before.plane.z0;
            (z0[0], z0[1]) = PRESET_HOMES[0].centre(0, 1);
            Plane {
                z0,
                ..before.plane.clone()
            }
        } else {
            before.plane.clone()
        };
        let at_pin = mv::chart_coords(&panned, &anchor).expect("a plane");
        click_scope(&mut rig.1, &mut rig.0, section.scope());
        let after = rig.0.snapshot().sim.clone();
        assert_eq!(after.lock, lock, "the lock and its anchor stay");
        let (s, t) = mv::chart_coords(&after.plane, &anchor).expect("a plane");
        assert!(
            (s - at_pin.0).abs() < 1e-9 && (t - at_pin.1).abs() < 1e-9,
            "{section:?} keeps the pin: ({s}, {t})"
        );
        // Control: free of the pin the anchor's chart point moves.
        rejects("a reset that ignores the pin", || {
            let free = Plane {
                z0: panned.z0,
                ..after.plane.clone()
            };
            let (s, t) = mv::chart_coords(&free, &anchor).expect("a plane");
            assert!((s - at_pin.0).abs() < 1e-9 && (t - at_pin.1).abs() < 1e-9);
        });
    }
}

/// One Ctrl+Z.
fn undo(rig: &mut Rig) {
    press(&mut rig.1, &mut rig.0, Key::Z, Modifiers::COMMAND);
    let _ = rig.1.frame(&mut rig.0, Vec::new());
}

/// Asserts one command raised the depth by exactly 1 and one undo restored `before` bit for bit, one redo `after`.
fn check_one_entry(rig: &mut Rig, depth: u32, before: &Plane) {
    let done = rig.0.snapshot().history.undo_depth;
    assert_eq!(done, depth + 1, "one entry");
    let after = rig.0.snapshot().sim.plane.clone();
    undo(rig);
    assert_eq!(&rig.0.snapshot().sim.plane, before, "one undo restores all");
    rig.0.redo();
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    assert_eq!(rig.0.snapshot().sim.plane, after, "one redo sets it again");
}

#[test]
fn mock_reset_undo_each_is_one_entry() {
    for section in SECTIONS {
        let mut rig = at_preset(0);
        move_all(&mut rig);
        let depth = rig.0.snapshot().history.undo_depth;
        let before = rig.0.snapshot().sim.plane.clone();
        click_scope(&mut rig.1, &mut rig.0, section.scope());
        check_one_entry(&mut rig, depth, &before);
        // Control: a command sent as two entries raises the depth by 2, so the check fails on it.
        let mut two = at_preset(0);
        move_all(&mut two);
        let depth = two.0.snapshot().history.undo_depth;
        let before = two.0.snapshot().sim.plane.clone();
        for edit in command(&two.0.snapshot().sim.clone(), section) {
            two.0.set_field(edit.clone());
            two.0.set_field(edit);
        }
        let _ = two.1.frame(&mut two.0, Vec::new());
        rejects("a command of two entries", || {
            check_one_entry(&mut two, depth, &before)
        });
    }
}

#[test]
fn mock_reset_undo_at_home_adds_no_entry() {
    let mut rig = at_preset(0);
    let (depth, rows) = (rig.0.snapshot().history.undo_depth, rig.0.console().len());
    for section in SECTIONS {
        click_scope(&mut rig.1, &mut rig.0, section.scope());
        let (app, h) = (&mut rig.0, &mut rig.1);
        app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(section.scope());
        keys_on(h, app);
        press(h, app, Key::Enter, NONE);
    }
    press(&mut rig.1, &mut rig.0, Key::Home, NONE);
    assert_eq!(rig.0.snapshot().history.undo_depth, depth, "nothing sent");
    assert_eq!(rig.0.console().len(), rows, "nothing logged");
}

/// A command is under a fresh key even with the pointer held: Home pressed through a drag leaves its own entry (K10).
#[test]
fn mock_reset_undo_home_during_a_held_drag_is_its_own_entry() {
    let mut rig = at_preset(0);
    let (app, h) = (&mut rig.0, &mut rig.1);
    let rail = app.keyboard.place_of(TAU1).expect("placed").rect.center();
    let button = |pos: Pos2, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    let _ = h.frame(app, vec![Event::PointerMoved(rail)]);
    let _ = h.frame(app, vec![button(rail, true)]);
    let _ = h.frame(app, vec![Event::PointerMoved(rail + egui::vec2(30.0, 0.0))]);
    let held = app.snapshot().history.undo_depth;
    assert!(
        enabled(&app.snapshot().sim, Section::Tilt),
        "the drag moved"
    );
    let [down, up] = crate::capture::key(Key::Home, NONE);
    let _ = h.frame(app, down);
    assert_eq!(app.snapshot().history.undo_depth, held + 1, "its own entry");
    let _ = h.frame(app, up);
    let _ = h.frame(app, vec![button(rail, false)]);
}

/// Whether the widget of scope `id`, in the frame of `output`, is drawn disabled.
fn disabled(output: &egui::FullOutput, rect: egui::Rect) -> bool {
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("accesskit on")
        .nodes
        .iter()
        .find(|(_, n)| {
            n.role() == Role::Button
                && n.bounds().is_some_and(|b| {
                    let c = egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
                    rect.contains(c)
                })
        })
        .map(|(_, n)| n.is_disabled())
        .expect("a button there")
}

/// The dimming each control is drawn with: the three resets', in [`SECTIONS`]' order, and the Home button's.
fn drawn_dimmed(rig: &mut Rig) -> ([bool; 3], bool) {
    let output = rig.1.frame(&mut rig.0, Vec::new());
    let of = |section: Section| {
        let place = rig.0.keyboard.place_of(section.scope()).expect("placed");
        disabled(&output, place.rect)
    };
    let resets = SECTIONS.map(of);
    let layout = crate::layout::Layout::new(rig.1.screen(), crate::capture::PIXELS_PER_POINT);
    (resets, disabled(&output, reset::home_rect(layout.figure)))
}

/// Asserts the controls' dimming: `off` the sections off home.
fn check_dimming(drawn: ([bool; 3], bool), off: &[Section]) {
    let (resets, home) = drawn;
    for (i, section) in SECTIONS.into_iter().enumerate() {
        assert_eq!(resets[i], !off.contains(&section), "{section:?}'s reset");
    }
    assert_eq!(home, off.is_empty(), "Home");
}

#[test]
fn mock_reset_dimming_at_home_and_after_a_move() {
    let mut rig = at_preset(0);
    check_dimming(drawn_dimmed(&mut rig), &[]);
    // Controls: a reset enabled at home, a reset disabled after its section moved.
    rejects("a reset enabled at home", || {
        check_dimming(drawn_dimmed(&mut at_preset(0)), &[Section::Tilt])
    });
    for (section, by) in [
        (Section::SliceCentre, move_slice as fn(&mut Rig)),
        (Section::Tilt, move_tilt),
        (Section::Navigate, move_navigate),
    ] {
        let mut rig = at_preset(0);
        by(&mut rig);
        check_dimming(drawn_dimmed(&mut rig), &[section]);
        assert!(home_enabled(&rig.0.snapshot().sim));
        rejects("a reset disabled after its section moved", || {
            check_dimming(drawn_dimmed(&mut rig), &[])
        });
        click_scope(&mut rig.1, &mut rig.0, section.scope());
        // The pointer on the button, its dimming read with no hover of it.
        let _ = rig.1.frame(&mut rig.0, Vec::new());
        check_dimming(drawn_dimmed(&mut rig), &[]);
    }
}

#[test]
fn mock_reset_dimming_at_the_tolerance() {
    let (a, b) = PRESETS[0];
    let window = PRESET_HOMES[0].window;
    let base = PRESET_HOMES[0].base;
    let mut rig = at_preset(0);
    let run = |rig: &mut Rig, z0: Latent, scale: f64| {
        rig.0.set_field(z0_edit(z0));
        rig.0
            .set_field(basis_edit((axis(a, scale), axis(b, scale))));
        let _ = rig.1.frame(&mut rig.0, Vec::new());
        SECTIONS.map(|s| enabled(&rig.0.snapshot().sim, s))
    };
    // z_q0's base point is 0, so the difference is exactly the offset.
    let (q0, past) = (2, 2.0 * HOME_TOLERANCE);
    let mut z0 = base;
    z0[q0] = HOME_TOLERANCE;
    assert_eq!(run(&mut rig, z0, window), [false; 3], "at 10⁻¹²");
    z0[q0] = past;
    assert_eq!(run(&mut rig, z0, window), [true, false, false], "past it");
    let mut z0 = base;
    z0[a] += past;
    assert_eq!(run(&mut rig, z0, window), [false, false, true], "z₀ on e_a");
    let mut z0 = base;
    z0[b] += past;
    assert_eq!(run(&mut rig, z0, window), [false, false, true], "z₀ on e_b");
    let off = window * (past * 8.0).exp2();
    assert_eq!(run(&mut rig, base, off), [false, false, true], "the zoom");
    let _ = run(&mut rig, base, window);
    // Each angle alone holds Tilt off home.
    for (tau1, tau2, gamma) in [(1e-9, 0.0, 0.0), (0.0, 1e-9, 0.0), (0.0, 0.0, 1e-9)] {
        let view = View {
            tau1,
            tau2,
            gamma,
            ..View::read(&rig.0.snapshot().sim)
        };
        rig.0.set_field(z0_edit(base));
        rig.0.set_field(basis_edit(view.basis()));
        let _ = rig.1.frame(&mut rig.0, Vec::new());
        let on = SECTIONS.map(|s| enabled(&rig.0.snapshot().sim, s));
        assert_eq!(on, [false, true, false], "{tau1} {tau2} {gamma}");
        let _ = run(&mut rig, base, window);
    }
}

#[test]
fn mock_reset_dimming_with_no_preset() {
    let mut rig = at_preset(0);
    let mut q1 = axis(0, 1.0);
    q1[1] = 0.5;
    q1[2] = 0.25;
    q1[3] = 0.125;
    q1[4] = 0.0625;
    rig.0.set_field(basis_edit((q1, axis(7, 1.0))));
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    let sim = rig.0.snapshot().sim.clone();
    assert_eq!(View::read(&sim).preset, None, "no preset holds it");
    assert_eq!(dimmed_because(&sim), NO_PRESET);
    check_dimming(drawn_dimmed(&mut rig), &[]);
    for section in SECTIONS {
        assert!(
            command(&sim, section).is_empty(),
            "{section:?} does nothing"
        );
    }
    assert!(reset::home(&sim).is_empty(), "Home does nothing");
    let depth = rig.0.snapshot().history.undo_depth;
    press(&mut rig.1, &mut rig.0, Key::Home, NONE);
    assert_eq!(rig.0.snapshot().history.undo_depth, depth);
    let at = at_preset(0);
    assert_eq!(dimmed_because(&at.0.snapshot().sim), ALREADY_HOME);
}

/// The compass's target plane and readout in the frame after `run`, and the mode it left.
fn check_live(mode: Touch, run: impl Fn(&mut Rig)) {
    let mut rig = at_preset(0);
    move_all(&mut rig);
    rig.0.manifold.compass.mode = mode;
    settle(&mut rig.1, &mut rig.0);
    run(&mut rig);
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    assert_eq!(rig.0.manifold.compass.mode, mode, "the mode is kept");
    let sim = rig.0.snapshot().sim.clone();
    let target = View::read(&sim);
    assert!(rig.0.manifold.compass.moving, "the cube eases to the plane");
    let drawn = drawing(&rig.1, &rig.0);
    assert_eq!(
        drawn.readout,
        readout(&sim.plane.z0, &target, mode),
        "the readout reads the new state"
    );
    settle(&mut rig.1, &mut rig.0);
    let rest = rig.0.manifold.compass.shown.expect("drawn");
    let (shown_q1, _) = rest.view.basis();
    assert_eq!(
        shown_q1,
        target.basis().0,
        "the cube rests on the new plane"
    );
}

#[test]
fn mock_reset_compass_live_in_both_modes() {
    for mode in [Touch::Tilt, Touch::Slice] {
        for section in SECTIONS {
            check_live(mode, |rig| {
                click_scope(&mut rig.1, &mut rig.0, section.scope());
            });
        }
        check_live(mode, |rig| {
            press(&mut rig.1, &mut rig.0, Key::Home, NONE);
        });
    }
    // Controls: a reset that switches the mode, and one that leaves the plane.
    rejects("a reset switching the mode", || {
        check_live(Touch::Tilt, |rig| {
            click_scope(&mut rig.1, &mut rig.0, Section::Tilt.scope());
            rig.0.manifold.compass.mode = Touch::Slice;
        })
    });
    rejects("a compass keeping the old plane", || {
        check_live(Touch::Tilt, |_| {})
    });
}
