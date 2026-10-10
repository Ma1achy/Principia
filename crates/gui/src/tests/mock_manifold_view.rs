//! `mock_manifold_view` (REQ-GUI-171; render_gui_spec §G2, §G4; chart_decoder_contract Parts 3–4): every control of
//! the Manifold view emits a `SetField` on `z₀` or the basis, by its keys, its drag or its typed value; the preset
//! switches the basis; after a lock a slider reads the anchor plus its offset and moving it is an excursion, a
//! `SetField` on `z₀` with the anchor kept; and the precision warning follows the mock's events (R-54). Each check also
//! runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, Event, Key, Modifiers, PointerButton};
use engine::contract::interface::EngineInterface;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Plane};
use engine::contract::snapshot::{Precision, Snapshot};
use engine::contract::view_ui::Mode;

use super::support::{focus, frame_texts, headless, mock_app, press, rejects};
use crate::app::App;
use crate::explore::manifold_view::centre::{offset_text, span};
use crate::explore::manifold_view::depth::{
    depth_text, digits_left, superscript, warning, NO_WARNING, QUAD_LEVEL,
};
use crate::explore::manifold_view::navigate::{
    figure_coords, magnification, pan, wheel_octaves, zoom_by,
};
use crate::explore::manifold_view::slice_tilt::{Angles, ROTATION_RANGE, TILT_RANGE};
use crate::explore::manifold_view::{
    self as mv, activate, add, adjust, axis, basis_edit, chart_coords, dominant, in_plane,
    lock_edit, point, slice_axis, z0_edit, Out, Scratch, Touch, View, CENTRE_Z0, GAMMA, NAV_Z0,
    PRESETS, SLICE_AXIS, SLIDERS, TAU1, ZOOM,
};
use crate::explore::{COMPASS, FIGURE, MANIFOLD_VIEW};
use crate::headless::Headless;
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, Arrows};
use crate::mock::{MockEngine, MOCK_Q1, MOCK_Q2, MOCK_Z0};
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;

fn mock_snapshot() -> Snapshot {
    MockEngine::frozen().snapshot()
}

fn close(a: &Latent, b: &Latent, tol: f64) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
}

/// The `z₀` or basis a `SetField` sets; panics on any other edit.
fn sim_field(edit: &SetField) -> &SimField {
    match &edit.edit {
        Edit::Sim(field @ (SimField::Z0(_) | SimField::Basis { .. })) => field,
        other => panic!("not a SetField on z₀ or the basis: {other:?}"),
    }
}

/// Asserts that `out` holds one undoable `SetField`, on `z₀` or the basis, that moves `plane`.
fn one_setfield_moving(out: &Out, plane: &Plane, what: &str) {
    assert_eq!(
        out.edits.len(),
        1,
        "{what}: one SetField, got {:?}",
        out.edits
    );
    let edit = &out.edits[0];
    assert!(!edit.no_history, "{what}: undoable (R-69)");
    match sim_field(edit) {
        SimField::Z0(z0) => assert_ne!(z0, &plane.z0, "{what}: z₀ moved"),
        SimField::Basis { q1, q2 } => {
            assert_ne!((q1, q2), (&plane.q1, &plane.q2), "{what}: the basis moved")
        }
        SimField::Lock(_) => unreachable!("sim_field passes z₀ and the basis only"),
    }
}

/// Every value the Manifold view's sections, the figure and the compass hold, by id, with its step kind.
fn values(app: &App<MockSide>) -> Vec<Adjust> {
    let tree = app.keyboard.tree(Mode::Explore);
    let mut found = Vec::new();
    let mut stack: Vec<&str> = tree.children(Some(MANIFOLD_VIEW));
    stack.extend([FIGURE, COMPASS]);
    while let Some(id) = stack.pop() {
        let scope = tree.get(id).expect("a registered scope");
        if let Arrows::Adjust(kind) = scope.arrows {
            found.push(Adjust {
                scope: scope.id,
                kind,
                times: 1.0,
                direction: Direction::Up,
                shift: false,
            });
        }
        stack.extend(tree.children(Some(id)));
    }
    found
}

#[test]
fn mock_manifold_view_every_value_emits_one_setfield_on_z0_or_the_basis() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let snapshot = mock_snapshot();
    let values = values(&app);
    // Chart's sixteen components, Navigate's u, v, zoom and eight z₀, Centre z₀'s eight, Slice & tilt's four, the
    // figure and the compass.
    assert_eq!(
        values.len(),
        16 + 11 + 8 + 4 + 2,
        "{:?}",
        values.iter().map(|v| v.scope).collect::<Vec<_>>()
    );
    for value in &values {
        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            for times in [1.0, -1.0] {
                let adjust_ = Adjust {
                    direction,
                    times,
                    ..*value
                };
                let mut out = Out::default();
                adjust(&adjust_, &snapshot, &mut Scratch::default(), &mut out);
                one_setfield_moving(&out, &snapshot.sim.plane, value.scope);
            }
        }
    }
    // Control: a value of another screen's emits nothing here.
    let mut out = Out::default();
    let other = Adjust {
        scope: "time_scrub",
        ..values[0]
    };
    adjust(&other, &snapshot, &mut Scratch::default(), &mut out);
    rejects("another screen's value", || {
        one_setfield_moving(&out, &snapshot.sim.plane, "time_scrub")
    });
    // Control: a lock edit is not on z₀ or the basis.
    let lock = lock_edit(Lock {
        locked: true,
        z_locked: MOCK_Z0,
    });
    rejects("a lock edit", || {
        let _ = sim_field(&lock);
    });
}

#[test]
fn mock_manifold_view_values_step_as_their_kind_says() {
    let snapshot = mock_snapshot();
    let plane = &snapshot.sim.plane;
    let run = |scope, kind, times, direction, shift| {
        let mut out = Out::default();
        let mut scratch = Scratch::default();
        let a = Adjust {
            scope,
            kind,
            times,
            direction,
            shift,
        };
        adjust(&a, &snapshot, &mut scratch, &mut out);
        (out.edits.first().map(|e| sim_field(e).clone()), scratch)
    };
    use crate::keyboard::scopes::StepKind::{Angle, Bounded, Pan, Tilt, ZoomLog2};
    // A z₀ value: a hundredth of its range, 2, on its own axis, in Navigate and in Centre z₀ alike.
    for ids in [NAV_Z0, SLIDERS] {
        for (i, id) in ids.iter().enumerate() {
            let (edit, scratch) = run(id, Bounded, 1.0, Direction::Up, false);
            let mut want = plane.z0;
            want[i] += 0.02;
            assert_eq!(edit, Some(SimField::Z0(want)), "{id}");
            assert_eq!(scratch.compass.mode, Touch::Slice, "{id} shows slicing");
        }
    }
    // A basis component: a hundredth of its range, 2.
    let (edit, _) = run(mv::Q2_COMPONENTS[3], Bounded, -1.0, Direction::Down, false);
    let mut q2 = plane.q2;
    q2[3] -= 0.02;
    assert_eq!(edit, Some(SimField::Basis { q1: plane.q1, q2 }));
    let (edit, _) = run(mv::Q1_COMPONENTS[0], Bounded, 1.0, Direction::Up, false);
    let mut q1 = plane.q1;
    q1[0] += 0.02;
    assert_eq!(edit, Some(SimField::Basis { q1, q2: plane.q2 }));
    // u and v: a twentieth of the view, along q₁ and q₂.
    let (edit, _) = run(mv::CENTRE_U, Pan, 1.0, Direction::Up, false);
    assert_eq!(edit, Some(SimField::Z0(add(&plane.z0, 0.1, &plane.q1))));
    let (edit, _) = run(mv::CENTRE_V, Pan, -1.0, Direction::Up, false);
    assert_eq!(edit, Some(SimField::Z0(add(&plane.z0, -0.1, &plane.q2))));
    // The zoom: a quarter octave.
    let (edit, _) = run(ZOOM, ZoomLog2, 4.0, Direction::Up, false);
    let Some(SimField::Basis { q1, q2 }) = edit else {
        panic!("a basis edit")
    };
    assert!(
        close(&q1, &MOCK_Q1.map(|v| v / 2.0), 1e-12)
            && close(&q2, &MOCK_Q2.map(|v| v / 2.0), 1e-12)
    );
    // The slice step: z₀ along the hidden slice direction.
    let (edit, _) = run(mv::SLICE_STEP, Bounded, 1.0, Direction::Up, false);
    let mut want = plane.z0;
    want[SLICE_AXIS] += 0.02;
    assert_eq!(edit, Some(SimField::Z0(want)));
    // An angle: a degree, shown as tilting.
    let (edit, scratch) = run(TAU1, Angle, 1.0, Direction::Up, false);
    let Some(SimField::Basis { q1, q2 }) = edit else {
        panic!("a basis edit")
    };
    let view = View::of(&Plane {
        z0: plane.z0,
        q1,
        q2,
    });
    assert!((view.tau1.to_degrees() - 1.0).abs() < 1e-9, "{view:?}");
    assert_eq!(scratch.compass.mode, Touch::Tilt);
    let (edit, _) = run(GAMMA, Angle, -1.0, Direction::Up, false);
    let Some(SimField::Basis { q1, q2 }) = edit else {
        panic!("a basis edit")
    };
    let view = View::of(&Plane {
        z0: plane.z0,
        q1,
        q2,
    });
    assert!((view.gamma.to_degrees() + 1.0).abs() < 1e-9, "{view:?}");
    // The figure: ← → along q₁, ↑ ↓ along q₂.
    let (edit, _) = run(FIGURE, Pan, 1.0, Direction::Right, false);
    assert_eq!(edit, Some(SimField::Z0(add(&plane.z0, 0.1, &plane.q1))));
    let (edit, _) = run(FIGURE, Pan, -1.0, Direction::Left, false);
    assert_eq!(edit, Some(SimField::Z0(add(&plane.z0, -0.1, &plane.q1))));
    let (edit, _) = run(FIGURE, Pan, 1.0, Direction::Up, false);
    assert_eq!(edit, Some(SimField::Z0(add(&plane.z0, 0.1, &plane.q2))));
    let (edit, _) = run(FIGURE, Pan, -1.0, Direction::Down, false);
    assert_eq!(edit, Some(SimField::Z0(add(&plane.z0, -0.1, &plane.q2))));
    // The compass: arrows tilt, Shift+arrows orbit the cube and edit nothing.
    let (edit, scratch) = run(COMPASS, Tilt, 1.0, Direction::Right, false);
    assert!(matches!(edit, Some(SimField::Basis { .. })), "{edit:?}");
    assert_eq!(scratch.compass.mode, Touch::Tilt);
    let (edit, scratch) = run(COMPASS, Tilt, 10.0, Direction::Right, true);
    assert_eq!(edit, None, "Shift orbits");
    assert_ne!(
        scratch.compass,
        Scratch::default().compass,
        "the cube turned"
    );
    rejects("a pan along the wrong axis", || {
        let (edit, _) = run(FIGURE, Pan, 1.0, Direction::Right, false);
        assert_eq!(edit, Some(SimField::Z0(add(&plane.z0, 0.1, &plane.q2))));
    });
}

#[test]
fn mock_manifold_view_the_preset_cycles_the_basis() {
    let snapshot = mock_snapshot();
    let mut out = Out::default();
    activate(mv::PRESET, &snapshot, &mut Scratch::default(), &mut out);
    // The mock's chart is the first preset, z_α × z_β; the next is z_q0 × z_q1, at the same scale.
    let (a, b) = PRESETS[1];
    assert_eq!(out.edits, vec![basis_edit((axis(a, 1.0), axis(b, 1.0)))]);
    // From the last preset, back to the first.
    let mut last = snapshot.clone();
    let (a, b) = PRESETS[PRESETS.len() - 1];
    (last.sim.plane.q1, last.sim.plane.q2) = (axis(a, 0.5), axis(b, 0.5));
    let mut out = Out::default();
    activate(mv::PRESET, &last, &mut Scratch::default(), &mut out);
    let (a, b) = PRESETS[0];
    assert_eq!(out.edits, vec![basis_edit((axis(a, 0.5), axis(b, 0.5)))]);
    // An edited basis, in no preset's frame: the first preset.
    let mut edited = snapshot.clone();
    edited.sim.plane.q1 = [0.6, 0.0, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0];
    edited.sim.plane.q2 = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    assert_eq!(View::of(&edited.sim.plane).preset, None);
    let mut out = Out::default();
    activate(mv::PRESET, &edited, &mut Scratch::default(), &mut out);
    let (a, b) = PRESETS[0];
    let scale = View::of(&edited.sim.plane).scale;
    assert_eq!(
        out.edits,
        vec![basis_edit((axis(a, scale), axis(b, scale)))],
        "at its scale"
    );
    // Enter on a value, or on a control of another screen's, activates nothing here.
    for id in [ZOOM, "time_play", mv::BUILDER] {
        let mut out = Out::default();
        activate(id, &snapshot, &mut Scratch::default(), &mut out);
        assert_eq!(out, Out::default(), "{id}");
    }
    rejects("the preset left as it was", || {
        let mut out = Out::default();
        activate(mv::PRESET, &snapshot, &mut Scratch::default(), &mut out);
        assert_eq!(out.edits, vec![basis_edit((MOCK_Q1, MOCK_Q2))]);
    });
}

// --- the app, by keys, drags and typing --------------------------------------------------------------------------------

fn keys(h: &mut Headless, app: &mut App<MockSide>, keys: &[Key]) {
    for key in keys {
        press(h, app, *key, NONE);
    }
}

/// Tab, Tab, Enter, ↓, ↓, Enter: Centre z₀'s first slider, adjusting.
fn to_first_slider(h: &mut Headless, app: &mut App<MockSide>) {
    keys(
        h,
        app,
        &[
            Key::Tab,
            Key::Tab,
            Key::Enter,
            Key::ArrowDown,
            Key::ArrowDown,
            Key::Enter,
        ],
    );
    assert_eq!(focus(app), [MANIFOLD_VIEW, CENTRE_Z0, SLIDERS[0]]);
}

fn place(app: &App<MockSide>, id: &str) -> egui::Rect {
    app.keyboard
        .place_of(id)
        .unwrap_or_else(|| panic!("{id} placed"))
        .rect
}

/// A primary drag from `from` by `delta`, one frame a step.
fn drag(h: &mut Headless, app: &mut App<MockSide>, from: egui::Pos2, delta: egui::Vec2) {
    let to = from + delta;
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    for events in [
        vec![Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![Event::PointerMoved(from + delta / 2.0)],
        vec![Event::PointerMoved(to)],
        vec![button(to, false)],
        Vec::new(),
    ] {
        let _ = h.frame(app, events);
    }
}

#[test]
fn mock_manifold_view_the_arrows_on_a_slider_emit_a_setfield_on_z0() {
    let mut app = mock_app();
    let mut h = headless();
    to_first_slider(&mut h, &mut app);
    let before = app.snapshot().clone();
    press(&mut h, &mut app, Key::ArrowUp, NONE);
    let _ = h.frame(&mut app, Vec::new());
    let after = app.snapshot().clone();
    let mut want = before.sim.plane.z0;
    want[0] += 0.02;
    assert!(
        close(&after.sim.plane.z0, &want, 1e-12),
        "{:?}",
        after.sim.plane.z0
    );
    assert_eq!(
        after.history.undo_depth,
        before.history.undo_depth + 1,
        "one SetField"
    );
    rejects("the slider before the key", || {
        assert!(close(&before.sim.plane.z0, &want, 1e-12))
    });
}

#[test]
fn mock_manifold_view_a_drag_on_a_slider_emits_setfields_on_z0_or_the_basis() {
    for (id, on_z0) in [
        (SLIDERS[0], true),
        (SLIDERS[7], true),
        (mv::SLICE_STEP, true),
        (TAU1, false),
        (GAMMA, false),
        (ZOOM, false),
    ] {
        let mut app = mock_app();
        let mut h = headless();
        let _ = h.frame(&mut app, Vec::new());
        let before = app.snapshot().clone();
        let rect = place(&app, id);
        // On the slider's rail, a third of the way along, dragged right.
        let from = egui::pos2(rect.min.x + rect.width() * 0.2, rect.center().y);
        drag(&mut h, &mut app, from, egui::vec2(30.0, 0.0));
        let after = app.snapshot().clone();
        assert!(
            after.history.undo_depth > before.history.undo_depth,
            "{id}: SetFields"
        );
        let (b, a) = (&before.sim.plane, &after.sim.plane);
        if on_z0 {
            assert_ne!(a.z0, b.z0, "{id} moves z₀");
            assert_eq!((a.q1, a.q2), (b.q1, b.q2), "{id} leaves the basis");
        } else {
            assert_ne!((a.q1, a.q2), (b.q1, b.q2), "{id} moves the basis");
            assert_eq!(a.z0, b.z0, "{id} leaves z₀");
        }
        assert_eq!(after.sim.lock, before.sim.lock, "{id} leaves the lock");
    }
    // Control: a drag on the panel's title moves nothing.
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let before = app.snapshot().history.undo_depth;
    let title = place(&app, MANIFOLD_VIEW).min + egui::vec2(20.0, 14.0);
    drag(&mut h, &mut app, title, egui::vec2(30.0, 0.0));
    let after = app.snapshot().history.undo_depth;
    rejects("a drag on the title", || assert!(after > before));
}

#[test]
fn mock_manifold_view_a_typed_value_emits_a_setfield_on_z0() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let field = place(&app, NAV_Z0[1]).center();
    for events in crate::capture::click(field) {
        let _ = h.frame(&mut app, events);
    }
    let select_all = Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    };
    for events in [
        vec![select_all],
        vec![Event::Text("0.5".to_owned())],
        crate::capture::key(Key::Enter, NONE)[0].clone(),
        crate::capture::key(Key::Enter, NONE)[1].clone(),
        Vec::new(),
        Vec::new(),
    ] {
        let _ = h.frame(&mut app, events);
    }
    let z0 = app.snapshot().sim.plane.z0;
    assert!((z0[1] - 0.5).abs() < 1e-12, "z_β typed: {z0:?}");
    let mut rest = z0;
    rest[1] = MOCK_Z0[1];
    assert_eq!(rest, MOCK_Z0, "the other values kept");
    rejects("the value before typing", || {
        assert!((MOCK_Z0[1] - 0.5).abs() < 1e-12)
    });
}

// --- the lock: re-based sliders and excursions --------------------------------------------------------------------------

#[test]
fn mock_manifold_view_locked_sliders_read_anchor_plus_offset_and_move_as_an_excursion() {
    let mut app = mock_app();
    let mut h = headless();
    let free = frame_texts(&mut h, &mut app);
    let anchor = MOCK_Z0;
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: anchor,
    }));
    let texts = frame_texts(&mut h, &mut app);
    let zero = offset_text(0.0, 0.0);
    assert_eq!(zero, "+ 0.000");
    // Centre z₀'s eight sliders and the slice step read anchor + 0.
    let offsets = texts.iter().filter(|t| **t == zero).count();
    assert_eq!(offsets, 9, "{texts:?}");
    rejects("the sliders while free", || {
        assert_eq!(free.iter().filter(|t| **t == zero).count(), 9)
    });
    to_first_slider(&mut h, &mut app);
    let depth = app.snapshot().history.undo_depth;
    press(&mut h, &mut app, Key::ArrowUp, NONE);
    let texts = frame_texts(&mut h, &mut app);
    let snapshot = app.snapshot().clone();
    assert!(
        (snapshot.sim.plane.z0[0] - (anchor[0] + 0.02)).abs() < 1e-12,
        "z₀ moved off the anchor"
    );
    assert_eq!(
        snapshot.sim.lock,
        Lock {
            locked: true,
            z_locked: anchor
        },
        "the anchor kept: an excursion"
    );
    assert_eq!(snapshot.history.undo_depth, depth + 1, "one SetField");
    let moved = offset_text(snapshot.sim.plane.z0[0], anchor[0]);
    assert_eq!(moved, "+ 0.020");
    assert!(
        texts.contains(&moved),
        "the slider reads anchor + 0.020: {texts:?}"
    );
    rejects("the slider read against zero", || {
        assert!(texts.contains(&offset_text(snapshot.sim.plane.z0[0], 0.0)));
    });
}

#[test]
fn mock_manifold_view_slider_spans_and_offsets() {
    assert_eq!(span(None), (-1.0, 1.0), "free: [−1, 1]");
    assert_eq!(span(Some(0.25)), (-0.75, 1.25), "locked: the anchor ± 1");
    assert_eq!(offset_text(0.3, 0.25), "+ 0.050");
    assert_eq!(offset_text(0.2, 0.25), "− 0.050", "a true minus");
    assert_eq!(offset_text(0.25, 0.25), "+ 0.000", "no offset reads plus");
    rejects("a span centred on zero while locked", || {
        assert_eq!(span(Some(0.25)), (-1.0, 1.0))
    });
}

// --- the precision warning: the events, not a depth (R-54) ----------------------------------------------------------

/// The mock's app with its basis set to zoom `octaves`, and the texts of its next frame.
fn at_zoom(octaves: f64) -> (Snapshot, Vec<String>) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let k = (-octaves).exp2();
    app.set_field(basis_edit((MOCK_Q1.map(|v| v * k), MOCK_Q2.map(|v| v * k))));
    let texts = frame_texts(&mut h, &mut app);
    (app.snapshot().clone(), texts)
}

#[test]
fn mock_manifold_view_the_precision_warning_follows_the_events() {
    let (calm, texts) = at_zoom(19.5);
    assert_eq!(
        calm.precision,
        Precision {
            decode_switchover: false,
            at_f32_floor: false
        }
    );
    assert!(
        texts.iter().any(|t| t == NO_WARNING),
        "no event, no warning"
    );
    let (switched, texts) = at_zoom(20.5);
    assert_eq!(
        switched.precision,
        Precision {
            decode_switchover: true,
            at_f32_floor: false
        }
    );
    let want = format!("f64 · {} digits left · linearise decode", digits_left(20.5));
    assert!(texts.contains(&want), "{want:?} in {texts:?}");
    let (floor, texts) = at_zoom(23.5);
    assert_eq!(
        floor.precision,
        Precision {
            decode_switchover: true,
            at_f32_floor: true
        }
    );
    let want = format!(
        "f64 · {} digits left · linearise decode · at the f32 floor",
        digits_left(23.5)
    );
    assert!(texts.contains(&want), "{want:?} in {texts:?}");
    rejects("the warning without the event", || {
        let (_, texts) = at_zoom(19.5);
        assert!(!texts.iter().any(|t| t == NO_WARNING));
    });
}

#[test]
fn mock_manifold_view_depth_and_warning_texts() {
    assert_eq!(superscript(0), "⁰");
    assert_eq!(superscript(-12), "⁻¹²");
    assert_eq!(superscript(305), "³⁰⁵");
    assert_eq!(depth_text(0.0), format!("2⁰ · quad level {QUAD_LEVEL}"));
    assert_eq!(depth_text(3.99), "2⁻³ · quad level —");
    assert_eq!(depth_text(4.0), "2⁻⁴ · quad level —");
    assert_eq!(depth_text(-1.5), "2² · quad level —");
    // 53 log₁₀ 2 ≈ 15.95 digits.
    assert_eq!(digits_left(0.0), 15);
    assert_eq!(digits_left(-5.0), 15, "zoomed out, no digits are spent");
    assert_eq!(digits_left(20.0), 9);
    assert_eq!(digits_left(53.0), 0);
    assert_eq!(digits_left(80.0), 0, "never below zero");
    let none = Precision {
        decode_switchover: false,
        at_f32_floor: false,
    };
    assert_eq!(
        warning(&none, 30.0),
        None,
        "no event, no warning, however deep"
    );
    let floor = Precision {
        decode_switchover: false,
        at_f32_floor: true,
    };
    assert_eq!(warning(&floor, 0.0).as_deref(), Some("at the f32 floor"));
    rejects("a warning from the depth alone", || {
        assert!(warning(&none, 30.0).is_some())
    });
}

// --- the chart's maths ------------------------------------------------------------------------------------------------

#[test]
fn mock_manifold_view_a_view_round_trips_its_basis() {
    for (p, &(a, b)) in PRESETS.iter().enumerate() {
        for (scale, tau1, tau2, gamma) in [
            (1.0, 0.0, 0.0, 0.0),
            (0.25, 0.3, -0.2, 0.7),
            (2.0f64.powi(-30), -1.2, 0.9, -2.8),
            (3.0, 0.0, 0.5, 1.4),
            (0.5, 0.4, 0.0, -1.0),
        ] {
            let view = View {
                preset: Some(p),
                a,
                b,
                d: slice_axis(a, b),
                scale,
                tau1,
                tau2,
                gamma,
            };
            let (q1, q2) = view.basis();
            let back = View::of(&Plane {
                z0: MOCK_Z0,
                q1,
                q2,
            });
            assert_eq!(
                (back.preset, back.a, back.b, back.d),
                (Some(p), a, b, view.d),
                "{view:?}"
            );
            for (x, y, what) in [
                (back.scale / scale, 1.0, "scale"),
                (back.tau1, tau1, "τ₁"),
                (back.tau2, tau2, "τ₂"),
                (back.gamma, gamma, "γ"),
            ] {
                assert!((x - y).abs() < 1e-9, "{what}: {x} for {y}, {view:?}");
            }
            assert!((back.zoom() + scale.log2()).abs() < 1e-9);
        }
    }
    let view = View::of(&Plane {
        z0: MOCK_Z0,
        q1: MOCK_Q1,
        q2: MOCK_Q2,
    });
    assert_eq!(view.name(), "z_α × z_β");
    assert_eq!(view.with_zoom(2.0).scale, 0.25);
    rejects("a view that lost its rotation", || {
        let view = View {
            preset: Some(0),
            a: 0,
            b: 1,
            d: SLICE_AXIS,
            scale: 1.0,
            tau1: 0.0,
            tau2: 0.0,
            gamma: 0.5,
        };
        let (q1, q2) = view.basis();
        assert_eq!(
            View::of(&Plane {
                z0: MOCK_Z0,
                q1,
                q2
            })
            .gamma,
            0.0
        );
    });
}

#[test]
fn mock_manifold_view_the_frame_of_an_edited_basis() {
    // Leaning on z_q1 and z_μ2, outside every preset: read in its dominant axes' frame.
    let q1 = [0.1, 0.0, 0.0, -0.9, 0.0, 0.0, 0.2, 0.0];
    let q2 = [0.0, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8];
    let view = View::of(&Plane {
        z0: MOCK_Z0,
        q1,
        q2,
    });
    assert_eq!(
        (view.preset, view.a, view.b, view.d),
        (None, 3, 7, SLICE_AXIS)
    );
    // q₂ leaning on q₁'s axis most: its next axis names it.
    let q2 = [0.0, 0.0, 0.0, 0.95, 0.0, 0.0, 0.0, 0.5];
    let view = View::of(&Plane {
        z0: MOCK_Z0,
        q1,
        q2,
    });
    assert_eq!((view.a, view.b), (3, 7));
    assert_eq!(slice_axis(0, 1), SLICE_AXIS);
    assert_eq!(
        slice_axis(SLICE_AXIS, 1),
        0,
        "the slice axis shown: the first axis neither shows"
    );
    assert_eq!(slice_axis(0, SLICE_AXIS), 1);
    assert_eq!(slice_axis(1, SLICE_AXIS), 0);
    assert_eq!(
        dominant(&[0.0, -0.5, 0.5, 0.2, 0.0, 0.0, 0.0, 0.0]),
        1,
        "the first of equals"
    );
    assert_eq!(dominant(&[0.0; 8]), 0);
    assert_eq!(dominant(&axis(7, -0.1)), 7, "by size, not sign");
    rejects("the first preset's frame", || {
        assert_eq!(view.preset, Some(0))
    });
}

#[test]
fn mock_manifold_view_chart_points_and_coordinates() {
    let plane = Plane {
        z0: MOCK_Z0,
        q1: [0.5, 0.0, 0.0, 0.0, 0.0, 0.2, 0.0, 0.0],
        q2: [0.1, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
    for (s, t) in [(0.0, 0.0), (1.0, 1.0), (0.35, 0.6), (0.5, 0.5), (-0.2, 1.4)] {
        let z = point(&plane, s, t);
        let (s2, t2) = chart_coords(&plane, &z).expect("a sound basis");
        assert!(
            (s - s2).abs() < 1e-12 && (t - t2).abs() < 1e-12,
            "({s}, {t}) came back ({s2}, {t2})"
        );
    }
    assert!(
        close(&point(&plane, 0.5, 0.5), &plane.z0, 1e-15),
        "the centre is z₀"
    );
    assert!(
        close(
            &point(&plane, 1.0, 0.5),
            &add(&plane.z0, 1.0, &plane.q1),
            1e-15
        ),
        "s = 1 is z₀ + q₁"
    );
    assert!(
        close(
            &point(&plane, 0.5, 0.0),
            &add(&plane.z0, -1.0, &plane.q2),
            1e-15
        ),
        "t = 0 is z₀ − q₂"
    );
    // In-plane coordinates along the unit directions.
    let square = Plane {
        z0: [0.0; 8],
        q1: axis(0, 2.0),
        q2: axis(1, 0.5),
    };
    assert_eq!(
        in_plane(&square, &[0.3, -0.4, 9.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        Some((0.3, -0.4))
    );
    let skew = Plane {
        z0: [0.0; 8],
        q1: axis(0, 1.0),
        q2: [0.6, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
    let (u, v) = in_plane(&skew, &add(&axis(0, 0.5), 2.0, &skew.q2)).expect("a sound basis");
    assert!(
        (u - 0.5).abs() < 1e-12 && (v - 2.0).abs() < 1e-12,
        "({u}, {v})"
    );
    // Degenerate bases have none.
    let flat = Plane {
        z0: [0.0; 8],
        q1: axis(0, 1.0),
        q2: axis(0, 2.0),
    };
    assert_eq!(in_plane(&flat, &MOCK_Z0), None);
    assert_eq!(chart_coords(&flat, &MOCK_Z0), None);
    let zero = Plane {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: axis(1, 1.0),
    };
    assert_eq!(in_plane(&zero, &MOCK_Z0), None);
    assert_eq!(chart_coords(&zero, &MOCK_Z0), None);
    let zero2 = Plane {
        z0: [0.0; 8],
        q1: axis(1, 1.0),
        q2: [0.0; 8],
    };
    assert_eq!(in_plane(&zero2, &MOCK_Z0), None);
    // A deep but sound basis keeps its coordinates.
    let deep = Plane {
        z0: [0.0; 8],
        q1: axis(0, 1e-15),
        q2: axis(1, 1e-15),
    };
    let (s, t) = chart_coords(&deep, &point(&deep, 0.25, 0.75)).expect("deep is not degenerate");
    assert!(
        (s - 0.25).abs() < 1e-12 && (t - 0.75).abs() < 1e-12,
        "({s}, {t})"
    );
    assert!(in_plane(&deep, &MOCK_Z0).is_some());
    rejects("the point at t = 1 for t = 0", || {
        assert!(close(
            &point(&plane, 0.5, 1.0),
            &add(&plane.z0, -1.0, &plane.q2),
            1e-15
        ));
    });
}

#[test]
fn mock_manifold_view_navigation_edits() {
    let plane = Plane {
        z0: MOCK_Z0,
        q1: MOCK_Q1,
        q2: MOCK_Q2,
    };
    // A drag right by a tenth of the figure moves the view left by a tenth: z₀ − 0.2 q₁; down, up: z₀ + 0.2 q₂.
    let edit = pan(&plane, egui::vec2(10.0, 0.0), egui::vec2(100.0, 50.0));
    assert_eq!(edit, z0_edit(add(&plane.z0, -0.2, &plane.q1)));
    let edit = pan(&plane, egui::vec2(0.0, 5.0), egui::vec2(100.0, 50.0));
    assert_eq!(edit, z0_edit(add(&plane.z0, 0.2, &plane.q2)));
    // An octave in halves the basis; out doubles it.
    assert_eq!(
        zoom_by(&plane, 1.0),
        basis_edit((MOCK_Q1.map(|v| v / 2.0), MOCK_Q2.map(|v| v / 2.0)))
    );
    assert_eq!(
        zoom_by(&plane, -1.0),
        basis_edit((MOCK_Q1.map(|v| v * 2.0), MOCK_Q2.map(|v| v * 2.0)))
    );
    assert_eq!(magnification(0.0), "× 1.00");
    assert_eq!(magnification(3.0), "× 8.00");
    // The figure's coordinates: t Y-up.
    let figure = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(200.0, 100.0));
    assert_eq!(
        figure_coords(figure, egui::pos2(100.0, 150.0)),
        (0.0, 0.0),
        "bottom left"
    );
    assert_eq!(
        figure_coords(figure, egui::pos2(300.0, 50.0)),
        (1.0, 1.0),
        "top right"
    );
    assert_eq!(
        figure_coords(figure, egui::pos2(150.0, 125.0)),
        (0.25, 0.25)
    );
    // The wheel: a notch is an eighth of an octave, a point a sixty-fourth, a page one.
    let wheel = |unit, y: f32| Event::MouseWheel {
        unit,
        delta: egui::vec2(3.0, y),
        phase: egui::TouchPhase::Move,
        modifiers: NONE,
    };
    assert_eq!(
        wheel_octaves(&[wheel(egui::MouseWheelUnit::Line, 2.0)]),
        0.25
    );
    assert_eq!(
        wheel_octaves(&[wheel(egui::MouseWheelUnit::Point, 32.0)]),
        0.5
    );
    assert_eq!(
        wheel_octaves(&[wheel(egui::MouseWheelUnit::Page, -1.0)]),
        -1.0
    );
    let events = [
        wheel(egui::MouseWheelUnit::Line, 8.0),
        Event::PointerMoved(egui::pos2(1.0, 1.0)),
        wheel(egui::MouseWheelUnit::Line, 8.0),
    ];
    assert_eq!(wheel_octaves(&events), 2.0, "summed, other events ignored");
    rejects("a pan against the drag", || {
        assert_eq!(
            pan(&plane, egui::vec2(10.0, 0.0), egui::vec2(100.0, 50.0)),
            z0_edit(add(&plane.z0, 0.2, &plane.q1))
        );
    });
}

#[test]
fn mock_manifold_view_angles_turn_within_their_spans() {
    let mut angles = Angles {
        tau1: 89.5,
        tau2: -89.5,
        gamma: 179.0,
    };
    angles.turn(TAU1, 1.0);
    angles.turn(mv::TAU2, -1.0);
    angles.turn(GAMMA, 2.0);
    assert_eq!(
        angles,
        Angles {
            tau1: TILT_RANGE.1,
            tau2: TILT_RANGE.0,
            gamma: ROTATION_RANGE.1
        }
    );
    angles.turn(GAMMA, -400.0);
    assert_eq!(angles.gamma, ROTATION_RANGE.0);
    angles.turn(TAU1, -10.0);
    assert_eq!(angles.tau1, 80.0, "τ₁ only");
    assert_eq!(
        (angles.tau2, angles.gamma),
        (TILT_RANGE.0, ROTATION_RANGE.0)
    );
    assert_eq!(
        Angles {
            tau1: 23.44,
            tau2: 0.0,
            gamma: 30.3
        }
        .readout(),
        "τ₁ 23.4° · τ₂ 0.0° · γ 30.3°"
    );
    let view = View {
        preset: Some(0),
        a: 0,
        b: 1,
        d: SLICE_AXIS,
        scale: 1.0,
        tau1: 0.0,
        tau2: 0.0,
        gamma: 0.0,
    };
    let turned = Angles {
        tau1: 90.0,
        tau2: -45.0,
        gamma: 180.0,
    }
    .apply(view);
    assert_eq!(
        (turned.tau1, turned.tau2, turned.gamma),
        (
            std::f64::consts::FRAC_PI_2,
            -std::f64::consts::FRAC_PI_4,
            std::f64::consts::PI
        )
    );
    rejects("a tilt past its span", || {
        let mut a = Angles {
            tau1: 89.5,
            tau2: 0.0,
            gamma: 0.0,
        };
        a.turn(TAU1, 1.0);
        assert_eq!(a.tau1, 90.5);
    });
}

// --- the panel's layout: rows, fields and their places ---------------------------------------------------------------

use super::support::{nodes, shapes, Node};

fn click_at(h: &mut Headless, app: &mut App<MockSide>, at: egui::Pos2) {
    for events in crate::capture::click(at) {
        let _ = h.frame(app, events);
    }
}
use eframe::egui::accesskit::Role;

/// The panel, its body and its row height, at the capture's size.
fn panel_geometry(h: &Headless) -> (egui::Rect, egui::Rect, f32, [egui::Rect; 4]) {
    let panel =
        crate::layout::Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT).manifold_view;
    (
        panel,
        panel.shrink(mv::MARGIN),
        mv::row_height(panel),
        mv::sections(panel),
    )
}

fn in_rect(nodes: &[Node], rect: egui::Rect, role: Role) -> Vec<Node> {
    let mut found: Vec<Node> = nodes
        .iter()
        .filter(|n| n.role == role && rect.contains(n.rect.center()))
        .cloned()
        .collect();
    found.sort_by(|a, b| {
        (a.rect.min.y, a.rect.min.x)
            .partial_cmp(&(b.rect.min.y, b.rect.min.x))
            .unwrap()
    });
    found
}

/// Asserts that every slider row in `section` (rows `rows`) holds its slider then its field, side by side, filling the
/// body to its right edge, each within its row; `offset` adds the locked offset after the field.
fn check_slider_rows(
    all: &[Node],
    section: egui::Rect,
    h: f32,
    rows: std::ops::Range<usize>,
    body: egui::Rect,
    offset: bool,
) {
    for i in rows {
        let line = mv::row(section, h, i);
        let sliders = in_rect(all, line, Role::Slider);
        let fields = in_rect(all, line, Role::SpinButton);
        assert_eq!(
            (sliders.len(), fields.len()),
            (1, 1),
            "row {i} at {line:?}: one slider, one field"
        );
        let (s, f) = (&sliders[0].rect, &fields[0].rect);
        assert!(
            s.max.x <= f.min.x && f.min.x - s.max.x <= 16.0,
            "row {i}: the field follows the slider: {s:?} {f:?}"
        );
        assert!(
            s.width() > 120.0,
            "row {i}: the slider takes the row: {s:?}"
        );
        let end = if offset {
            f.max.x + 8.0 + crate::explore::manifold_view::centre::OFFSET_W
        } else {
            f.max.x
        };
        assert!(
            (end - body.max.x).abs() <= 1.0,
            "row {i}: the row ends at the body's edge: {f:?}, {body:?}"
        );
        for r in [s, f] {
            assert!(
                r.min.y >= line.min.y - 0.5 && r.max.y <= line.max.y + 0.5,
                "row {i}: {r:?} in {line:?}"
            );
            assert!(
                r.height() >= h - 6.0,
                "row {i}: {r:?} fills its row's height {h}"
            );
        }
    }
}

#[test]
fn mock_manifold_view_slider_rows_fill_the_panel() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let (_, body, rh, [_, _, centre, slice]) = panel_geometry(&h);
    let all = nodes(&mut h, &mut app);
    check_slider_rows(&all, centre, rh, 1..9, body, false);
    check_slider_rows(&all, slice, rh, 1..5, body, false);
    // Locked, the offset follows each field, inside the body.
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: MOCK_Z0,
    }));
    let _ = h.frame(&mut app, Vec::new());
    let locked = nodes(&mut h, &mut app);
    check_slider_rows(&locked, centre, rh, 1..9, body, true);
    check_slider_rows(&locked, slice, rh, 1..2, body, true);
    check_slider_rows(&locked, slice, rh, 2..5, body, false);
    let offsets: Vec<&Node> = locked.iter().filter(|n| n.text == "+ 0.000").collect();
    assert_eq!(offsets.len(), 9);
    for o in offsets {
        assert!(
            o.rect.max.x <= body.max.x + 0.5,
            "the offset inside the body: {:?}",
            o.rect
        );
    }
    rejects("the Chart section, which has no sliders", || {
        let (_, _, rh, [chart, ..]) = panel_geometry(&h);
        check_slider_rows(&all, chart, rh, 1..2, body, false);
    });
}

#[test]
fn mock_manifold_view_every_control_in_its_row_inside_the_body() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let (panel, body, rh, [chart, navigate, ..]) = panel_geometry(&h);
    let all = nodes(&mut h, &mut app);
    let inside_panel: Vec<&Node> = all
        .iter()
        .filter(|n| panel.contains(n.rect.center()))
        .collect();
    for n in &inside_panel {
        assert!(
            body.expand(1.0).contains_rect(n.rect),
            "{} at {:?} outside the body {body:?}",
            n.text,
            n.rect
        );
    }
    // Navigate: u and v in row 1, the zoom in row 2, z₀ in rows 3 and 4, four to a row, in order.
    let fields = |i| in_rect(&all, mv::row(navigate, rh, i), Role::SpinButton);
    assert_eq!(fields(1).len(), 2, "u and v");
    let zoom = in_rect(&all, mv::row(navigate, rh, 2), Role::Slider);
    assert_eq!(zoom.len(), 1, "the zoom");
    assert!(
        zoom[0].rect.width() > 150.0,
        "the zoom slider takes its row: {:?}",
        zoom[0].rect
    );
    let z0: Vec<f64> = fields(3)
        .iter()
        .chain(&fields(4))
        .filter_map(|n| n.value)
        .collect();
    assert_eq!(z0, MOCK_Z0, "z₀ in order, four to a row");
    for n in fields(1).iter().chain(&fields(3)).chain(&fields(4)) {
        assert!(
            (n.rect.height() - (rh - 2.0)).abs() < 0.5,
            "{:?} fills its row of {rh}, less 2",
            n.rect
        );
    }
    // Chart: the preset across row 1, the basis rows' edit buttons at the right, the builder in row 4.
    let preset = all
        .iter()
        .find(|n| n.text == "z_α × z_β")
        .expect("the preset");
    assert!(
        preset.rect.width() > 150.0 && preset.rect.max.x <= body.max.x + 0.5,
        "{:?}",
        preset.rect
    );
    for i in [2, 3] {
        let line = mv::row(chart, rh, i);
        let edit = all
            .iter()
            .find(|n| n.text == "edit…" && line.contains(n.rect.center()))
            .expect("an edit button");
        assert!(
            edit.rect.max.x <= body.max.x + 0.5 && edit.rect.max.x > body.max.x - 30.0,
            "{:?}",
            edit.rect
        );
    }
    let texts: Vec<&str> = all.iter().map(|n| n.text.as_str()).collect();
    for text in [
        "Chart",
        "Navigate",
        "Centre z₀",
        "Slice & tilt",
        mv::navigate::NOTE,
        "1.00·z_α",
        "1.00·z_β",
        mv::chart::KIND,
    ] {
        assert!(texts.contains(&text), "no {text:?}");
    }
    assert!(mv::row(chart, rh, 4).contains(
        all.iter()
            .find(|n| n.text == "Chart builder…")
            .unwrap()
            .rect
            .center()
    ));
    assert_eq!(
        mv::row(navigate, 10.0, 2).min.y,
        navigate.min.y + 20.0,
        "row i starts i rows down"
    );
    rejects("a field outside the body", || {
        let mut far = inside_panel[0].clone();
        far.rect = far.rect.translate(egui::vec2(body.width(), 0.0));
        assert!(body.expand(1.0).contains_rect(far.rect));
    });
}

#[test]
fn mock_manifold_view_the_zoom_slider_reaches_out_past_zero() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    app.set_field(basis_edit((
        MOCK_Q1.map(|v| v * 4.0),
        MOCK_Q2.map(|v| v * 4.0),
    )));
    let _ = h.frame(&mut app, Vec::new());
    let (_, _, rh, [_, navigate, ..]) = panel_geometry(&h);
    let all = nodes(&mut h, &mut app);
    let zoom = in_rect(&all, mv::row(navigate, rh, 2), Role::Slider);
    assert_eq!(zoom[0].value, Some(-2.0), "two octaves out reads −2");
    assert!(all.iter().any(|n| n.text == "× 0.25"));
    rejects("a zoom clamped at zero", || {
        assert_eq!(zoom[0].value, Some(0.0))
    });
}

#[test]
fn mock_manifold_view_the_anchor_marks_each_rails_centre() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: MOCK_Z0,
    }));
    let _ = h.frame(&mut app, Vec::new());
    let (_, _, rh, [_, _, centre, _]) = panel_geometry(&h);
    let all = nodes(&mut h, &mut app);
    let ticks: Vec<[egui::Pos2; 2]> = shapes(&mut h, &mut app)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == crate::explore::lock::GOLD =>
            {
                Some(points)
            }
            _ => None,
        })
        .collect();
    for i in 1..9 {
        let rail = in_rect(&all, mv::row(centre, rh, i), Role::Slider)[0].rect;
        let tick = ticks
            .iter()
            .find(|[a, _]| rail.contains(*a))
            .unwrap_or_else(|| panic!("no anchor mark on row {i}'s rail {rail:?}"));
        assert!(
            (tick[0].x - rail.center().x).abs() < 0.5 && tick[0].x == tick[1].x,
            "at the centre: {tick:?}"
        );
        assert_eq!(
            (tick[0].y, tick[1].y),
            (rail.min.y + 2.0, rail.max.y - 2.0),
            "across the rail: {tick:?}"
        );
    }
    rejects("a free panel's rails", || {
        let mut free = mock_app();
        let mut h = headless();
        let _ = h.frame(&mut free, Vec::new());
        let gold = shapes(&mut h, &mut free).into_iter().any(|s| {
            matches!(s, egui::Shape::LineSegment { stroke, .. } if stroke.color == crate::explore::lock::GOLD)
        });
        assert!(gold);
    });
}

/// Clicks the field at `at`, selects its text and types `text`, then Enter.
fn type_at(h: &mut Headless, app: &mut App<MockSide>, at: egui::Pos2, text: &str) {
    for events in crate::capture::click(at) {
        let _ = h.frame(app, events);
    }
    let select_all = Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    };
    let [enter_down, enter_up] = crate::capture::key(Key::Enter, NONE);
    for events in [
        vec![select_all],
        vec![Event::Text(text.to_owned())],
        enter_down,
        enter_up,
        Vec::new(),
        Vec::new(),
    ] {
        let _ = h.frame(app, events);
    }
}

#[test]
fn mock_manifold_view_a_typed_angle_sets_that_angle() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    // From τ₁ = 10°, typing 25 sets 25°.
    let view = View {
        tau1: 10f64.to_radians(),
        ..View::of(&Plane {
            z0: MOCK_Z0,
            q1: MOCK_Q1,
            q2: MOCK_Q2,
        })
    };
    app.set_field(basis_edit(view.basis()));
    let _ = h.frame(&mut app, Vec::new());
    let (_, _, rh, [_, _, _, slice]) = panel_geometry(&h);
    let all = nodes(&mut h, &mut app);
    for (i, want) in [(2, 10.0), (3, 0.0), (4, 0.0)] {
        let field = &in_rect(&all, mv::row(slice, rh, i), Role::SpinButton)[0];
        assert!(
            (field.value.unwrap() - want).abs() < 1e-6,
            "row {i} reads {:?}",
            field.value
        );
    }
    let field = in_rect(&all, mv::row(slice, rh, 2), Role::SpinButton)[0]
        .rect
        .center();
    type_at(&mut h, &mut app, field, "25");
    let plane = app.snapshot().sim.plane.clone();
    let tau1 = View::of(&plane).tau1.to_degrees();
    assert!((tau1 - 25.0).abs() < 1e-9, "τ₁ typed to 25: {tau1}");
    // γ typed, in its own row.
    let all = nodes(&mut h, &mut app);
    let field = in_rect(&all, mv::row(slice, rh, 4), Role::SpinButton)[0]
        .rect
        .center();
    type_at(&mut h, &mut app, field, "-30");
    let gamma = View::of(&app.snapshot().sim.plane).gamma.to_degrees();
    assert!((gamma + 30.0).abs() < 1e-9, "γ typed to −30: {gamma}");
    rejects("τ₁ turned by the typed value", || {
        assert!((tau1 - 35.0).abs() < 1e-9)
    });
}

#[test]
fn mock_manifold_view_dragging_u_moves_the_centre_along_q1() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    // One octave in: the view spans half as much, and u steps half as fast.
    app.set_field(basis_edit((
        MOCK_Q1.map(|v| v / 2.0),
        MOCK_Q2.map(|v| v / 2.0),
    )));
    let _ = h.frame(&mut app, Vec::new());
    let u = place(&app, mv::CENTRE_U).center();
    drag(&mut h, &mut app, u, egui::vec2(40.0, 0.0));
    let z0 = app.snapshot().sim.plane.z0;
    let moved = z0[0] - MOCK_Z0[0];
    // 40 points at σ / 200 a point: 0.1 along the unit direction of q₁.
    assert!((moved - 0.1).abs() < 0.02, "u moved z_α by {moved}");
    assert_eq!(z0[1], MOCK_Z0[1], "v kept");
    rejects("a step four times smaller", || {
        assert!((moved - 0.025).abs() < 0.02)
    });
}

#[test]
fn mock_manifold_view_plus_zooms_only_the_focused_figure() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    // The top bar focused: + is not the figure's.
    press(&mut h, &mut app, Key::Tab, NONE);
    press(&mut h, &mut app, Key::Plus, NONE);
    let _ = h.frame(&mut app, Vec::new());
    let q1 = app.snapshot().sim.plane.q1;
    assert_eq!(q1, MOCK_Q1, "no zoom without the figure's focus");
    keys(&mut h, &mut app, &[Key::Tab, Key::Tab]);
    press(&mut h, &mut app, Key::Plus, NONE);
    let _ = h.frame(&mut app, Vec::new());
    let zoomed = app.snapshot().sim.plane.q1;
    assert_ne!(zoomed, MOCK_Q1, "the focused figure zooms");
    rejects("the unfocused +", || assert_ne!(q1, MOCK_Q1));
}

#[test]
fn mock_manifold_view_the_chart_rows() {
    use crate::explore::manifold_view::chart::basis_text;
    assert_eq!(basis_text(&MOCK_Q1), "1.00·z_α");
    assert_eq!(basis_text(&[0.0; 8]), "0");
    assert_eq!(
        basis_text(&[-0.5, 0.0, 0.25, 0.0, 0.0, -0.125, 0.0, 0.0]),
        "−0.50·z_α + 0.25·z_q0 − 0.12·z_q3"
    );
    assert_eq!(
        basis_text(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0]),
        "2.00·z_μ2"
    );
    rejects("a zero component listed", || {
        assert_eq!(basis_text(&MOCK_Q1), "1.00·z_α + 0.00·z_β")
    });
    // The editor opens by its button, and by the keyboard's focus inside it, and its fields edit the basis.
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let closed = nodes(&mut h, &mut app)
        .iter()
        .filter(|n| n.role == Role::SpinButton)
        .count();
    let button = place(&app, mv::Q1_EDIT).center();
    click_at(&mut h, &mut app, button);
    let open = nodes(&mut h, &mut app);
    let fields = open.iter().filter(|n| n.role == Role::SpinButton).count();
    assert_eq!(fields, closed + 8, "q₁'s eight fields");
    assert!(
        app.manifold.editing[0] && !app.manifold.editing[1],
        "{:?}",
        app.manifold.editing
    );
    // Two rows of four, under the button, in order.
    let mut editor: Vec<&Node> = open
        .iter()
        .filter(|n| {
            n.role == Role::SpinButton
                && n.value.is_some()
                && place(&app, mv::Q1_COMPONENTS[0]).min.y <= n.rect.center().y
        })
        .filter(|n| {
            mv::Q1_COMPONENTS
                .iter()
                .any(|id| place(&app, id).contains(n.rect.center()))
        })
        .collect();
    editor.sort_by(|a, b| {
        (a.rect.min.y, a.rect.min.x)
            .partial_cmp(&(b.rect.min.y, b.rect.min.x))
            .unwrap()
    });
    let values: Vec<f64> = editor.iter().filter_map(|n| n.value).collect();
    assert_eq!(values, MOCK_Q1, "q₁'s components in order");
    assert!(editor[3].rect.min.y < editor[4].rect.min.y, "four to a row");
    assert_eq!(editor[0].rect.min.y, editor[3].rect.min.y);
    type_at(&mut h, &mut app, editor[1].rect.center(), "0.5");
    let q1 = app.snapshot().sim.plane.q1;
    assert_eq!(
        q1,
        [1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        "z_β of q₁ typed"
    );
    // The button again closes it.
    let button = place(&app, mv::Q1_EDIT).center();
    click_at(&mut h, &mut app, button);
    assert!(!app.manifold.editing[0]);
    let after = nodes(&mut h, &mut app)
        .iter()
        .filter(|n| n.role == Role::SpinButton)
        .count();
    assert_eq!(after, closed);
    // The keyboard inside q₂'s editor opens it.
    let mut app = mock_app();
    let mut h = headless();
    keys(
        &mut h,
        &mut app,
        &[
            Key::Tab,
            Key::Tab,
            Key::Enter,
            Key::Enter,
            Key::Enter,
            Key::ArrowDown,
            Key::ArrowDown,
            Key::Enter,
        ],
    );
    assert_eq!(
        focus(&app),
        [MANIFOLD_VIEW, mv::CHART, mv::Q2_EDIT, mv::Q2_COMPONENTS[0]]
    );
    let n = nodes(&mut h, &mut app)
        .iter()
        .filter(|n| n.role == Role::SpinButton)
        .count();
    assert_eq!(n, closed + 8, "q₂'s editor open while the focus is in it");
    assert!(
        !app.manifold.editing[1],
        "opened by the focus, not the button"
    );
    rejects("the editor closed", || assert_eq!(n, closed));
}

#[test]
fn mock_manifold_view_enter_unlocks_and_opens_the_inspector() {
    let mut locked = mock_snapshot();
    locked.sim.lock = Lock {
        locked: true,
        z_locked: MOCK_Z0,
    };
    let mut out = Out::default();
    activate(mv::UNLOCK, &locked, &mut Scratch::default(), &mut out);
    assert_eq!(
        out.edits,
        vec![lock_edit(Lock {
            locked: false,
            z_locked: [0.0; 8]
        })]
    );
    let mut out = Out::default();
    activate(
        mv::OPEN_INSPECTOR,
        &locked,
        &mut Scratch::default(),
        &mut out,
    );
    assert_eq!(
        out.windows,
        vec![engine::contract::view_ui::Window::Inspector]
    );
    rejects("Enter on unlock doing nothing", || {
        let mut out = Out::default();
        activate("lock_nothing", &locked, &mut Scratch::default(), &mut out);
        assert!(!out.edits.is_empty());
    });
}

#[test]
fn mock_manifold_view_reading_views_at_their_edges() {
    // A preset's frame holds a basis whose stray components are at the floating point's noise, at any scale.
    let mut q1 = MOCK_Q1.map(|v| v * 1e-5);
    q1[3] = 1e-15;
    let q2 = MOCK_Q2.map(|v| v * 1e-5);
    assert_eq!(
        View::of(&Plane {
            z0: MOCK_Z0,
            q1,
            q2
        })
        .preset,
        Some(0)
    );
    let mut stray = q1;
    stray[3] = 1e-9;
    assert_eq!(
        View::of(&Plane {
            z0: MOCK_Z0,
            q1: stray,
            q2
        })
        .preset,
        None
    );
    // τ₁ = 90°: q₁ lies along the depth, and γ reads from the vertical axis.
    let (s, c) = 0.5f64.sin_cos();
    let q1 = [0.0, s, 0.0, 0.0, 0.0, c, 0.0, 0.0];
    let q2 = [0.0, c, 0.0, 0.0, 0.0, -s, 0.0, 0.0];
    let v = View::of(&Plane {
        z0: MOCK_Z0,
        q1,
        q2,
    });
    assert!(
        (v.gamma - 0.5).abs() < 1e-12 && (v.tau1 - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
        "{v:?}"
    );
    // τ₂ = 90°: q₂'s horizontal part reads γ.
    let q1 = [c, 0.0, 0.0, 0.0, 0.0, s, 0.0, 0.0];
    let q2 = [-s, 0.0, 0.0, 0.0, 0.0, c, 0.0, 0.0];
    let v = View::of(&Plane {
        z0: MOCK_Z0,
        q1,
        q2,
    });
    assert!(
        (v.gamma - 0.5).abs() < 1e-12 && (v.tau2 - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
        "{v:?}"
    );
    // A zero angle reads +0, never −0.
    let v = View::of(&Plane {
        z0: MOCK_Z0,
        q1: MOCK_Q1,
        q2: MOCK_Q2,
    });
    assert!(v.gamma.is_sign_positive(), "γ {:?}", v.gamma);
    let mut q1 = MOCK_Q1;
    q1[SLICE_AXIS] = -0.0;
    let mut q2 = MOCK_Q2;
    q2[0] = -0.0;
    let v = View::of(&Plane {
        z0: MOCK_Z0,
        q1,
        q2,
    });
    assert!(v.tau1.is_sign_positive(), "τ₁ {:?}", v.tau1);
    let mut q2 = MOCK_Q2;
    q2[0] = -0.0;
    q2[SLICE_AXIS] = -0.0;
    let v = View::of(&Plane {
        z0: MOCK_Z0,
        q1,
        q2,
    });
    assert!(v.tau2.is_sign_positive(), "τ₂ {:?}", v.tau2);
    rejects("a −0 read as it is", || {
        assert!((-0.0f64).is_sign_positive())
    });
}

#[test]
fn mock_manifold_view_the_stand_in_projects_the_chart() {
    use crate::mock::canvas::{project, HIDDEN_WEIGHTS};
    let v = [0.5, -0.25, 1.0, 2.0, 0.0, 0.0, 0.0, -1.0];
    let hidden = HIDDEN_WEIGHTS[0] + 2.0 * HIDDEN_WEIGHTS[1] - HIDDEN_WEIGHTS[5];
    assert_eq!(project(&v), [0.5, -0.25, hidden as f32, 0.0]);
    rejects("the weights divided", || {
        assert_eq!(project(&v)[2], (1.0 / 0.7 + 2.0 / 0.5 - 1.0 / 0.4) as f32)
    });
}

#[test]
fn mock_manifold_view_a_long_message_stops_short_of_the_footers_right_end() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    // A basis edit's message is longer than the footer.
    let view = View {
        tau1: 0.3,
        gamma: 0.7,
        ..View::of(&Plane {
            z0: MOCK_Z0,
            q1: MOCK_Q1,
            q2: MOCK_Q2,
        })
    };
    app.set_field(basis_edit(view.basis()));
    let _ = h.frame(&mut app, Vec::new());
    let all = nodes(&mut h, &mut app);
    let footer = crate::layout::Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT).footer;
    let tag = all
        .iter()
        .find(|n| n.text == crate::explore::footer::MOCK_TAG)
        .expect("the mock tag")
        .rect;
    let message = all
        .iter()
        .find(|n| n.text.starts_with("SetField Plane.q1") && footer.contains(n.rect.center()))
        .expect("the latest message")
        .rect;
    assert!(
        message.max.x <= tag.min.x - 8.0,
        "the message {message:?} stops short of {tag:?}"
    );
    assert!(
        message.max.x >= tag.min.x - 24.0,
        "and uses the room up to it: {message:?}, {tag:?}"
    );
    rejects("a message running under the tag", || {
        assert!(message.max.x > tag.min.x)
    });
}

// --- the pin: locked, every basis edit turns about it (render_gui_spec §G4, §G2) -----------------------------------

/// The mock's state locked at its centre, then moved off the anchor by an excursion of `+0.4 q₁ − 0.2 q₂`: the anchor
/// sits at the chart point `(0.3, 0.6)`, off the centre.
fn after_an_excursion() -> Snapshot {
    let mut snapshot = mock_snapshot();
    snapshot.sim.lock = Lock {
        locked: true,
        z_locked: MOCK_Z0,
    };
    snapshot.sim.plane.z0 = add(&add(&MOCK_Z0, 0.4, &MOCK_Q1), -0.2, &MOCK_Q2);
    snapshot
}

/// The plane `edits` leave, applied in order to `plane`.
fn applied(plane: &Plane, edits: &[SetField]) -> Plane {
    let mut plane = plane.clone();
    for edit in edits {
        match &edit.edit {
            Edit::Sim(SimField::Z0(z0)) => plane.z0 = *z0,
            Edit::Sim(SimField::Basis { q1, q2 }) => (plane.q1, plane.q2) = (*q1, *q2),
            _ => {}
        }
    }
    plane
}

/// That the anchor keeps its chart point from `before` on `after`: the pin did not move in the chart.
fn holds_the_anchor(before: &Plane, after: &Plane, anchor: &Latent, what: &str) {
    let (s, t) = chart_coords(before, anchor).expect("a chart");
    assert!(
        close(&point(after, s, t), anchor, 1e-12),
        "{what}: the anchor left ({s}, {t})"
    );
}

/// The basis edits of the manifold view and the compass, each made against `snapshot`: the zoom, the compass's tilt,
/// the τ₁, τ₂ and γ sliders, and the next preset.
fn basis_edits(snapshot: &Snapshot) -> Vec<(&'static str, Vec<SetField>)> {
    use crate::keyboard::scopes::StepKind::{Angle, Tilt, ZoomLog2};
    let mut all = Vec::new();
    for (id, kind, direction) in [
        (ZOOM, ZoomLog2, Direction::Up),
        (COMPASS, Tilt, Direction::Right),
        (COMPASS, Tilt, Direction::Up),
        (TAU1, Angle, Direction::Up),
        (mv::TAU2, Angle, Direction::Up),
        (GAMMA, Angle, Direction::Up),
    ] {
        let a = Adjust {
            scope: id,
            kind,
            times: 4.0,
            direction,
            shift: false,
        };
        let mut out = Out::default();
        adjust(&a, snapshot, &mut Scratch::default(), &mut out);
        all.push((id, out.edits));
    }
    let mut out = Out::default();
    activate(mv::PRESET, snapshot, &mut Scratch::default(), &mut out);
    all.push((mv::PRESET, out.edits));
    all
}

#[test]
fn mock_manifold_view_locked_every_basis_edit_turns_about_the_pin() {
    let snapshot = after_an_excursion();
    let (plane, anchor) = (&snapshot.sim.plane, &snapshot.sim.lock.z_locked);
    assert_eq!(
        chart_coords(plane, anchor).map(|(s, t)| ((s * 10.0).round(), (t * 10.0).round())),
        Some((3.0, 6.0))
    );
    for (id, edits) in basis_edits(&snapshot) {
        assert!(
            matches!(edits.as_slice(), [e] if matches!(sim_field(e), SimField::Basis { .. })),
            "{id}: one basis edit, {edits:?}"
        );
        let pinned = mv::about_the_pin(edits.clone(), &snapshot.sim);
        assert!(
            matches!(pinned.as_slice(), [z, b] if matches!(sim_field(z), SimField::Z0(_)) && *b == edits[0]),
            "{id}: the centre moved first, then the basis as it was: {pinned:?}"
        );
        holds_the_anchor(plane, &applied(plane, &pinned), anchor, id);
        rejects("a basis edit turned about z₀", || {
            holds_the_anchor(plane, &applied(plane, &edits), anchor, id)
        });
    }
}

#[test]
fn mock_manifold_view_free_or_at_the_centre_the_basis_edits_pass_as_they_are() {
    // Free: no pin, every edit as it was.
    let mut free = after_an_excursion();
    free.sim.lock.locked = false;
    for (id, edits) in basis_edits(&free) {
        assert_eq!(
            mv::about_the_pin(edits.clone(), &free.sim),
            edits,
            "{id} free"
        );
    }
    // Locked with the anchor at the centre: z₀ is the pivot already, no edit on it.
    let mut centred = after_an_excursion();
    centred.sim.plane.z0 = MOCK_Z0;
    for (id, edits) in basis_edits(&centred) {
        assert_eq!(
            mv::about_the_pin(edits.clone(), &centred.sim),
            edits,
            "{id} at the centre"
        );
    }
    rejects("the pinned edits of the excursion", || {
        let snapshot = after_an_excursion();
        let edits = basis_edits(&snapshot).remove(0).1;
        assert_eq!(mv::about_the_pin(edits.clone(), &snapshot.sim), edits);
    });
}

#[test]
fn mock_manifold_view_the_pin_follows_the_edits_before_it() {
    let centred = {
        let mut s = after_an_excursion();
        s.sim.plane.z0 = MOCK_Z0;
        s
    };
    let excursion = after_an_excursion();
    let zoom = zoom_by(&excursion.sim.plane, 1.0);
    // An excursion and a zoom in one batch: the zoom turns about the anchor where the excursion left it.
    let edits = vec![z0_edit(excursion.sim.plane.z0), zoom.clone()];
    let pinned = mv::about_the_pin(edits.clone(), &centred.sim);
    assert_eq!(pinned.len(), 3, "{pinned:?}");
    let plane = &excursion.sim.plane;
    holds_the_anchor(
        plane,
        &applied(&centred.sim.plane, &pinned),
        &MOCK_Z0,
        "after the excursion",
    );
    rejects(
        "the zoom read against the centre before the excursion",
        || assert_eq!(mv::about_the_pin(edits.clone(), &centred.sim), edits),
    );
    // A lock and a zoom in one batch, from free: the zoom turns about the new anchor.
    let mut free = excursion.clone();
    free.sim.lock.locked = false;
    let edits = vec![lock_edit(excursion.sim.lock.clone()), zoom];
    let pinned = mv::about_the_pin(edits.clone(), &free.sim);
    assert_eq!(pinned.len(), 3, "{pinned:?}");
    holds_the_anchor(plane, &applied(plane, &pinned), &MOCK_Z0, "after the lock");
    rejects("the zoom read as free", || {
        assert_eq!(mv::about_the_pin(edits.clone(), &free.sim), edits)
    });
}

#[test]
fn mock_manifold_view_a_locked_wheel_after_an_excursion_keeps_the_pin() {
    let mut app = mock_app();
    let mut h = headless();
    let excursion = after_an_excursion();
    app.set_field(lock_edit(excursion.sim.lock.clone()));
    app.set_field(z0_edit(excursion.sim.plane.z0));
    let _ = h.frame(&mut app, Vec::new());
    let before = app.snapshot().sim.plane.clone();
    assert_eq!(before, excursion.sim.plane);
    let at = crate::layout::Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT)
        .figure
        .center();
    let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
    let _ = h.frame(
        &mut app,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 8.0),
            phase: egui::TouchPhase::Move,
            modifiers: NONE,
        }],
    );
    let _ = h.frame(&mut app, Vec::new());
    let after = app.snapshot().sim.plane.clone();
    assert_eq!(after.q1, MOCK_Q1.map(|v| v / 2.0), "one octave in");
    holds_the_anchor(&before, &after, &MOCK_Z0, "the wheel");
    rejects("the wheel about z₀", || {
        let about_z0 = Plane {
            z0: before.z0,
            ..after.clone()
        };
        holds_the_anchor(&before, &about_z0, &MOCK_Z0, "the wheel")
    });
}
