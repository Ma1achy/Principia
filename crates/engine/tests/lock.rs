//! The lock (chart_decoder_contract Part 4, "The lock (projective microscope)" and "The lock's construction, defined";
//! render_gui_spec §G4; R-69; TASK-M2-23): each gesture's typed `SetField` edits, applied through the real state store,
//! and every pixel read through the kernel's own chart maps and decode.
//!
//! - `lock_set` (REQ-GUI-004): on the affine chart and on two nonlinear charts, the centre pixel's IC after the lock is
//!   the clicked pixel's, bit for bit, the nonlinear chart rebuilt on the post-lock `z₀`; a lock at a point `validate`
//!   refuses sets nothing.
//! - `lock_anchor_invariance` (REQ-GUI-005): a property under `validation::prop` (`CASES` = 256, REQ-VAL-151): random
//!   tilts, zooms and chart switches leave the centre IC bitwise constant; an excursion moves it, and snap-back
//!   restores the stored anchor bitwise.
//! - `lock_sliders` (REQ-GUI-006): a slider moved while locked is an excursion along its control, re-based on the
//!   anchor; unlock leaves the free state the last locked one.
//! - `lock_setfields` (REQ-GUI-007): every lock gesture emits undoable `SetField`s on `SimConfig`'s lock group and `z₀`,
//!   and on a nonlinear chart its window.
//! - `lock_kernel_uniform_identical` (REQ-SYS-012): the kernel's uniform has no lock field, and a locked and a free
//!   view of one plane give it bitwise; the GPU half is `lock_gpu.rs`.
//! - `lock_infeasible_tilt` (REQ-CHART-010): a tilt of a locked `(L_z, E)` view past the parabola tags the pixels it
//!   carries out, and lock validation gives one notice: the switch to the latent chart, with a re-centre to the
//!   centre's own nearest feasible point only where an excursion carried the centre out (R-593).
//! - `curve_tilt_requires_lock` (REQ-CHART-007): a curve axis's tangent tilt is refused free, and locked it reads
//!   `γ′(ν₀)` pulled back to the latent controls; it is refused, naming the controls, where a link's inverse clamps.
//! - `lock_window_undo` (REQ-SYS-011; R-69, R-466; TASK-M2-21): a nonlinear chart's window is held in `SimConfig`'s
//!   chart group, and the lock's command, `z₀`, the lock group and the re-centred window sent under one history key, is
//!   one history entry: one undo restores all three, and one redo sets them again.

use engine::chart_authoring::charts::{nu_embed, NU_CURVE};
use engine::chart_authoring::CurvePoint;
use engine::contract::interface::EngineInterface;
use engine::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, HistoryKey, SetField, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice, SliceFrame, Z_Q3,
};
use engine::contract::store::StateStore;
use engine::lock::{
    centre, curve_tangent, curve_tilt, kernel_chart, move_slider, point, recentre, set,
    set_on_chart, slider, snap_back, unlock, validate, ChartLock, FrozenControls, LockError,
    Nearest, Notice, Pixel, Report, Tag, Window,
};
use engine::lookup::Tolerances;
use engine::nav::directions::hidden;
use engine::nav::{oblique, slice, zoom, ChartConstructor};
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Alpha, ZMu1};
use kernel::chart::{
    ChartDecode, Constraint, InvariantLE, InvariantLK, Latent as LatentChart, Mixed,
    ValidationResult,
};
use kernel::decode::descriptor::potential;
use kernel::decode::{decode, Ic};
use kernel::encode::by_kind::{feasibility, invariants, Feasibility};
use kernel::encode::SimplexClamp;
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{
    Defaults, Link, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ, SoftmaxTanh,
    StickBreaking,
};
use kernel::stage::decode::DecodeUniforms;
use proptest::prelude::*;
use validation::negative_control;
use validation::prop::{check_with_seed, run};
use validation::roundtrip::phys;

type Sticks =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

fn sim(plane: Plane) -> SimConfig {
    SimConfig {
        chart: Chart {
            window: Window::SQUARE,
        },
        plane,
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 100.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

fn store(sim: SimConfig) -> StateStore {
    StateStore::new(
        sim,
        RenderState {
            stain_graph: StainGraph::new(),
            overlays: Overlays::default(),
            hover: Hover::default(),
            palette: Palette {},
            playhead: Playhead { t: 0.0 },
        },
    )
}

/// The `SimConfig` the store holds.
fn now(s: &mut StateStore) -> SimConfig {
    s.snapshot().sim
}

/// Applies `edits` through the store.
fn apply(s: &mut StateStore, edits: impl IntoIterator<Item = SetField>) {
    for e in edits {
        s.set_field(e);
    }
}

fn scale(a: &Latent, k: f64) -> Latent {
    a.map(|x| k * x)
}

/// An oblique, zoomed plane through an off-origin centre.
fn oblique_plane() -> Plane {
    let a = [0.3, -1.1, 0.4, 0.2, -0.7, 0.9, 0.05, -0.3];
    let b = [-0.6, 0.2, 1.3, -0.4, 0.1, 0.5, -0.8, 0.25];
    let z0 = [0.4, -0.2, 0.1, -0.3, 0.2, 0.05, 0.6, -0.5];
    let p = oblique(z0, &a, &b).expect("independent draws span a plane");
    Plane {
        z0: p.z0,
        q1: scale(&p.q1, 0.3),
        q2: scale(&p.q2, 0.3),
    }
}

/// The free decode of the latent `z` under the default links.
fn ic_of(z: &Latent) -> Ic<f64> {
    decode::<Defaults, f64>(&kernel::decode::latent::Latent::new(*z)).ic
}

/// The bits of every value of `x`, so an IC compares bit for bit.
fn bits(x: &Ic<f64>) -> Vec<u64> {
    x.m.iter()
        .chain(x.r.iter().flatten())
        .chain(x.p.iter().flatten())
        .map(|v| v.to_bits())
        .collect()
}

fn same_bits(a: &Latent, b: &Latent) -> bool {
    a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

/// The pixels the checks lock at, inside `[0,1]²` and past it.
const PIXELS: [(f64, f64); 6] = [
    (0.3, 0.8),
    (0.0, 1.0),
    (0.9, 0.15),
    (0.5, 0.5),
    (-0.4, 1.3),
    (0.62, 0.07),
];

fn tol() -> Tolerances {
    Tolerances::register(128.0 * f64::EPSILON)
}

// ---- lock_set (REQ-GUI-004) ----

/// Locks the affine `plane` at `(s, t)` through the store with `lock`, and checks that the centre pixel's IC is the
/// clicked pixel's, bit for bit, and the lock group's anchor is `z(s, t)`.
fn check_affine_lock(plane: &Plane, s: f64, t: f64, lock: fn(&Plane, f64, f64) -> [SetField; 2]) {
    let clicked = point(plane, s, t);
    let mut st = store(sim(plane.clone()));
    apply(&mut st, lock(plane, s, t));
    let after = now(&mut st);
    assert!(
        bits(&ic_of(&centre(&after.plane))) == bits(&ic_of(&clicked)),
        "the centre pixel's IC after the lock at ({s}, {t}) is not the clicked pixel's"
    );
    assert!(after.lock.locked, "the lock flag is not set");
    assert!(
        same_bits(&after.lock.z_locked, &clicked),
        "the anchor {:?} is not z(s, t) = {clicked:?}",
        after.lock.z_locked
    );
    assert_eq!(
        (after.plane.q1, after.plane.q2),
        (plane.q1, plane.q2),
        "the lock edited the basis"
    );
}

#[test]
fn lock_set_affine_centre_is_the_clicked_pixel() {
    let plane = oblique_plane();
    for (s, t) in PIXELS {
        check_affine_lock(&plane, s, t, |p, s, t| set(p, s, t).expect("a lock"));
    }
    for bad in [f64::NAN, f64::INFINITY] {
        assert_eq!(set(&plane, bad, 0.5), Err(LockError::NonFinite));
        assert_eq!(set(&plane, 0.5, bad), Err(LockError::NonFinite));
    }
    let huge = Plane {
        q1: [f64::MAX; 8],
        ..plane.clone()
    };
    assert_eq!(
        set(&huge, 1.5, 0.5),
        Err(LockError::NonFinite),
        "a point past f64's range is not locked"
    );
}

negative_control!(
    lock_set_affine_centre_is_the_clicked_pixel,
    "a lock at z₀ + s·q₁ + t·q₂, the unsigned offsets, is not the clicked pixel",
    expected = "is not the clicked pixel's",
    check_affine_lock(&oblique_plane(), 0.3, 0.8, |p, s, t| {
        let z: Latent = std::array::from_fn(|i| p.z0[i] + s * p.q1[i] + t * p.q2[i]);
        [
            SetField {
                edit: Edit::Sim(SimField::Z0(z)),
                no_history: false,
            },
            SetField {
                edit: Edit::Sim(SimField::Lock(Lock {
                    locked: true,
                    z_locked: z,
                })),
                no_history: false,
            },
        ]
    })
);

/// A mixed chart, `(z_μ1, α)`: `α` a derived quantity, so `Φ` is nonlinear.
fn mixed() -> Mixed<ZMu1, Alpha, f64> {
    Mixed::new(
        [0.4, -0.3, 0.2, -0.1, 0.3, 0.15, 0.0, -0.4],
        [-1.0, 1.0],
        [0.25, 1.3],
    )
}

/// The `(L_z, E)` chart at a frozen configuration and masses.
fn invariant_le() -> InvariantLE<f64> {
    InvariantLE(Invariant {
        z_cfg: [0.4, -0.3],
        z_mass: [0.2, -0.35],
        k_max: 1.5,
        gamma_k: 2.0,
    })
}

/// The window over the whole square.
const SQUARE: Window = Window {
    centre: [0.5, 0.5],
    half: [0.5, 0.5],
};

/// A nonlinear chart rebuilt on `z₀`: its hidden coordinates are `z₀`'s (chart_reference §2.1), its other parameters
/// kept.
trait Rebuild: Sized {
    fn on(&self, z0: &Latent) -> Self;
}

impl Rebuild for Mixed<ZMu1, Alpha, f64> {
    fn on(&self, z0: &Latent) -> Self {
        Mixed::new(*z0, self.h_range, self.v_range)
    }
}

impl Rebuild for InvariantLE<f64> {
    fn on(&self, z0: &Latent) -> Self {
        InvariantLE(Invariant {
            z_cfg: [z0[0], z0[1]],
            z_mass: [z0[6], z0[7]],
            ..self.0
        })
    }
}

/// The lock's own edits.
fn own_edits(lock: &ChartLock) -> Vec<SetField> {
    lock.edits.to_vec()
}

/// Locks `chart` at the pixel `(s, t)` of `window`, applies the edits `edits` gives through the store, and checks that
/// the chart rebuilt on the post-lock `z₀` decodes the clicked pixel's state bit for bit at the re-centred window's
/// centre, that `z_locked` keeps the chart's frozen controls bit for bit and decodes to the clicked state within the
/// encode's tolerance, and that the edits are the recentring on `z₀ = z_locked`, the lock group's and the window's.
/// `centred` is the window the check reads the centre in, given the window before the lock and the store's after it.
fn check_chart_lock<C>(
    chart: &C,
    window: &Window,
    s: f64,
    t: f64,
    centred: fn(&Window, &Window) -> Window,
    edits: fn(&ChartLock) -> Vec<SetField>,
) where
    C: kernel::encode::by_kind::ByKind<Param = f64> + FrozenControls + Rebuild,
{
    let [u, v] = window.at(s, t);
    let clicked = chart.decode_at::<Defaults, f64>(u, v).ic;
    let lock = set_on_chart::<Defaults, C, _>(chart, window, s, t, phys, &tol()).expect("a lock");
    let mut before = sim(oblique_plane());
    before.chart.window = *window;
    let mut st = store(before);
    apply(&mut st, edits(&lock));
    let after = now(&mut st);
    let rebuilt = chart.on(&after.plane.z0);
    let w = centred(window, &after.chart.window);
    let [cu, cv] = w.at(0.5, 0.5);
    let centre_ic = rebuilt.decode_at::<Defaults, f64>(cu, cv).ic;
    assert!(
        bits(&centre_ic) == bits(&clicked),
        "the centre pixel's IC after the lock at ({s}, {t}) on {} is not the clicked pixel's",
        chart.name()
    );
    assert!(
        bits(&lock.ic) == bits(&clicked),
        "the lock's state is not the clicked one"
    );
    assert_eq!(lock.window.half, window.half, "the lock changed the zoom");
    let [z0, l, re] = &lock.edits;
    let z_locked = match (&z0.edit, &l.edit, &re.edit) {
        (
            Edit::Sim(SimField::Z0(z)),
            Edit::Sim(SimField::Lock(k)),
            Edit::Sim(SimField::Window(w)),
        ) => {
            assert!(
                k.locked && same_bits(&k.z_locked, z),
                "the lock's anchor is not z₀"
            );
            assert_eq!(
                *w, lock.window,
                "the window edit is not the re-centred window"
            );
            *z
        }
        other => panic!("the lock's edits are {other:?}"),
    };
    let (frozen, set_by) = (chart.frozen(), chart.set_by_axes());
    for k in (0..8).filter(|&k| !set_by[k]) {
        assert_eq!(
            z_locked[k].to_bits(),
            frozen[k].to_bits(),
            "z_locked's hidden control {k} is not the chart's frozen one on {}",
            chart.name()
        );
    }
    let d = phys(&clicked, &ic_of(&z_locked));
    assert!(
        d <= 1e-12,
        "z_locked decodes {d} from the clicked state on {}",
        chart.name()
    );
}

#[test]
fn lock_set_nonlinear_centre_is_the_clicked_pixel() {
    let zoomed = Window {
        centre: [0.375, 0.625],
        half: [0.125, 0.25],
    };
    for (s, t) in [(0.3, 0.8), (0.9, 0.15), (0.5, 0.5), (0.0, 1.0)] {
        for window in [SQUARE, zoomed] {
            check_chart_lock(&mixed(), &window, s, t, |_, re| *re, own_edits);
            check_chart_lock(&invariant_le(), &window, s, t, |_, re| *re, own_edits);
        }
    }
    // The controls each chart's axes set: the mixed chart's `z_μ1` and `z_α`, the invariant chart's momenta.
    let axes = |set: [bool; 8]| (0..8).filter(|&k| set[k]).collect::<Vec<_>>();
    assert_eq!(axes(mixed().set_by_axes()), [0, 6]);
    assert_eq!(axes(invariant_le().set_by_axes()), [2, 3, 4, 5]);
    assert_eq!(axes(lk().set_by_axes()), [2, 3, 4, 5]);
    assert_eq!(lk().frozen(), lk().0.frozen::<f64>().z);
    // The window's placement, `c + ((2s−1)·h_u, (2t−1)·h_v)`, at its corners and centre.
    assert_eq!(zoomed.at(0.0, 1.0), [0.25, 0.875]);
    assert_eq!(zoomed.at(1.0, 0.0), [0.5, 0.375]);
    assert_eq!(zoomed.at(0.75, 0.25), [0.4375, 0.5]);
    assert_eq!(zoomed.at(0.5, 0.5), zoomed.centre);
}

negative_control!(
    lock_set_nonlinear_centre_is_the_clicked_pixel,
    "a window left where it was is not centred on the lock",
    expected = "is not the clicked pixel's",
    check_chart_lock(&mixed(), &SQUARE, 0.3, 0.8, |old, _| *old, own_edits)
);

negative_control!(
    lock_set_nonlinear_frozen_controls,
    "z_locked taken whole from the ladder's round trip does not keep the frozen controls, so the rebuilt chart moves",
    expected = "is not the clicked pixel's",
    check_chart_lock(&mixed(), &SQUARE, 0.3, 0.8, |_, re| *re, |lock| {
        let z = engine::lookup::lookup::<Defaults, _, _>(
            &mixed(),
            &engine::lookup::Exact::Physical(lock.ic),
            phys,
            &tol(),
        )
        .expect("a lookup")
        .z;
        vec![
            SetField {
                edit: Edit::Sim(SimField::Z0(z)),
                no_history: false,
            },
            SetField {
                edit: Edit::Sim(SimField::Lock(Lock {
                    locked: true,
                    z_locked: z,
                })),
                no_history: false,
            },
            lock.edits[2].clone(),
        ]
    })
);

/// The `(L_z, K)` chart, whose frozen controls the trait reads as `(L_z, E)`'s.
fn lk() -> InvariantLK<f64> {
    InvariantLK(Invariant {
        z_cfg: [-0.2, 0.5],
        z_mass: [0.1, 0.3],
        k_max: 2.0,
        gamma_k: 1.5,
    })
}

/// A lock at a chart point `validate` refuses sets nothing (R-469): the `(L_z, E)` chart's edge `s = 1` is a system,
/// one float past it is not.
#[test]
fn lock_set_refuses_a_point_that_is_not_a_system() {
    let chart = invariant_le();
    let at = |u: f64| Window {
        centre: [u, 0.5],
        half: [0.5, 0.5],
    };
    let edge = set_on_chart::<Defaults, _, _>(&chart, &at(1.0), 0.5, 0.5, phys, &tol());
    assert!(edge.is_ok(), "the parabola's edge is refused: {edge:?}");
    let past =
        set_on_chart::<Defaults, _, _>(&chart, &at(1.0f64.next_up()), 0.5, 0.5, phys, &tol());
    assert!(
        matches!(
            past,
            Err(LockError::NotASystem(ValidationResult::Project {
                failed: Constraint::LzEParabola,
                ..
            }))
        ),
        "a point past the parabola is locked: {past:?}"
    );
    assert_eq!(
        set_on_chart::<Defaults, _, _>(&chart, &SQUARE, f64::NAN, 0.5, phys, &tol()),
        Err(LockError::NonFinite)
    );
    let wide = Window {
        centre: [f64::MAX, 0.5],
        half: [f64::MAX, 0.5],
    };
    assert_eq!(
        set_on_chart::<Defaults, _, _>(&chart, &wide, 1.0, 0.5, phys, &tol()),
        Err(LockError::NonFinite),
        "a chart point past f64's range is not locked"
    );
    let strict = Tolerances {
        sanity: -1.0,
        ..tol()
    };
    assert!(
        matches!(
            set_on_chart::<Defaults, _, _>(&mixed(), &SQUARE, 0.3, 0.8, phys, &strict),
            Err(LockError::Refused(_))
        ),
        "a state the ladder refuses is locked"
    );
}

negative_control!(
    lock_set_refuses_a_point_that_is_not_a_system,
    "the parabola's edge itself is no refusal",
    expected = "the edge is refused",
    {
        let at = Window {
            centre: [1.0, 0.5],
            half: [0.5, 0.5],
        };
        let r = set_on_chart::<Defaults, _, _>(&invariant_le(), &at, 0.5, 0.5, phys, &tol());
        assert!(r.is_err(), "the edge is refused");
    }
);

// ---- lock_anchor_invariance (REQ-GUI-005) ----

/// `ê_k`.
fn e(k: usize) -> Latent {
    let mut v = [0.0; 8];
    v[k] = 1.0;
    v
}

/// One anchor-preserving edit of a locked view's basis, drawn by [`gestures`].
#[derive(Clone, Debug)]
enum Gesture {
    /// A raw tilt pair toward two distinct hidden directions, the `first` and the `(first + 1 + gap) mod 6`-th, by
    /// `tau` degrees each.
    Tilt {
        first: usize,
        gap: usize,
        tau: (f64, f64),
    },
    /// A zoom by `octaves`.
    Zoom(f64),
    /// A chart switch to the axis pair `(ê_a, ê_(a + 1 + gap) mod 8)` at scale `k`.
    Switch { a: usize, gap: usize, k: f64 },
}

/// Sequences of 1 to 24 gestures.
fn gestures() -> impl Strategy<Value = Vec<Gesture>> {
    let gesture = prop_oneof![
        (0..6usize, 0..5usize, -89.0..89.0f64, -89.0..89.0f64).prop_map(|(first, gap, t1, t2)| {
            Gesture::Tilt {
                first,
                gap,
                tau: (t1, t2),
            }
        }),
        (-3.0..3.0f64).prop_map(Gesture::Zoom),
        (0..8usize, 0..7usize, 0.05..2.0f64).prop_map(|(a, gap, k)| Gesture::Switch { a, gap, k }),
    ];
    prop::collection::vec(gesture, 1..=24)
}

/// The basis edit of `gesture` on `sim`'s plane. Two distinct hidden directions, each orthogonal to the plane and to
/// each other, tilted by less than 90° always span a plane, so a tilt pair that gives no edit fails the case.
fn anchor_preserving(sim: &SimConfig, gesture: &Gesture) -> SetField {
    let plane = &sim.plane;
    match *gesture {
        Gesture::Tilt { first, gap, tau } => {
            let targets = hidden(plane).expect("a plane");
            let d1 = targets[first].direction;
            let d2 = targets[(first + 1 + gap) % 6].direction;
            ChartConstructor::new(plane.clone())
                .tilt_pair(plane, (tau.0, &d1), (tau.1, &d2))
                .expect("a tilt pair toward two distinct hidden directions")
        }
        Gesture::Zoom(octaves) => zoom(plane, octaves).expect("a zoom"),
        Gesture::Switch { a, gap, k } => SetField {
            edit: Edit::Sim(SimField::Basis {
                q1: scale(&e(a), k),
                q2: scale(&e((a + 1 + gap) % 8), k),
            }),
            no_history: false,
        },
    }
}

/// Locks at a pixel, then applies `gestures` through `edit`, checking after each that `z₀` and the centre pixel's IC
/// are the anchor's bit for bit; then an excursion, which moves them, and snap-back by `home`, which restores both bit
/// for bit.
fn check_anchor_invariance(
    gestures: &[Gesture],
    edit: fn(&SimConfig, &Gesture) -> SetField,
    home: fn(&SimConfig) -> SetField,
) -> Result<(), TestCaseError> {
    let plane = oblique_plane();
    let mut st = store(sim(plane.clone()));
    apply(&mut st, set(&plane, 0.71, 0.33).expect("a lock"));
    let anchor = now(&mut st).lock.z_locked;
    let anchor_ic = bits(&ic_of(&anchor));
    for (step, g) in gestures.iter().enumerate() {
        let e = edit(&now(&mut st), g);
        apply(&mut st, [e]);
        let s = now(&mut st);
        prop_assert!(
            same_bits(&s.plane.z0, &anchor) && bits(&ic_of(&centre(&s.plane))) == anchor_ic,
            "step {step}, {g:?}: the centre IC moved"
        );
        prop_assert!(
            same_bits(&s.lock.z_locked, &anchor),
            "step {step}, {g:?}: the anchor moved"
        );
    }
    let s = now(&mut st);
    let d = hidden(&s.plane).expect("a plane")[1].direction;
    apply(&mut st, [slice(&s.plane, &d, 0.3).expect("a slice")]);
    let moved = move_slider(&now(&mut st), 6, 0.123).expect("a slider");
    apply(&mut st, [moved]);
    let away = now(&mut st);
    prop_assert!(
        !same_bits(&away.plane.z0, &anchor),
        "the excursion did not move the centre"
    );
    let delta = away.lock.excursion(&away.plane.z0).expect("locked");
    prop_assert!(
        delta.iter().any(|&x| x != 0.0),
        "δ is 0 after the excursion"
    );
    apply(&mut st, [home(&away)]);
    let back = now(&mut st);
    prop_assert!(
        same_bits(&back.plane.z0, &anchor),
        "snap-back did not restore the stored anchor bitwise"
    );
    prop_assert!(
        bits(&ic_of(&centre(&back.plane))) == anchor_ic,
        "snap-back's centre IC is not the anchor's"
    );
    prop_assert_eq!(
        back.lock.excursion(&back.plane.z0),
        Some([0.0; 8]),
        "δ is not 0 after snap-back"
    );
    Ok(())
}

/// Snap-back, the home a locked view returns by.
fn snap(s: &SimConfig) -> SetField {
    snap_back(s).expect("locked")
}

/// The property at a fixed seed, which the test runs beside `run`'s and each control runs broken: its failure, printed,
/// as the panic.
fn at_seed(edit: fn(&SimConfig, &Gesture) -> SetField, home: fn(&SimConfig) -> SetField) {
    if let Err(failure) = check_with_seed(0x23_23, &gestures(), |g| {
        check_anchor_invariance(&g, edit, home)
    }) {
        panic!("{failure}");
    }
}

#[test]
fn lock_anchor_invariance() {
    run(&gestures(), |g| {
        check_anchor_invariance(&g, anchor_preserving, snap)
    });
    at_seed(anchor_preserving, snap);
    let free = sim(oblique_plane());
    assert_eq!(snap_back(&free), Err(LockError::NotLocked));
}

negative_control!(
    lock_anchor_invariance,
    "a pan in place of each basis edit moves the centre",
    expected = "the centre IC moved",
    at_seed(
        |s, _| engine::nav::pan(&s.plane, 0.1, 0.0).expect("a pan"),
        snap
    )
);

negative_control!(
    lock_anchor_invariance_snap_back,
    "unlock in place of snap-back leaves the excursion",
    expected = "snap-back did not restore",
    at_seed(anchor_preserving, |s| unlock(s).expect("locked"))
);

// ---- lock_sliders (REQ-GUI-006) ----

/// Locks, moves each slider in `moves` through `mover`, checking each is an excursion along its control from the
/// anchor, re-based on it; then unlocks and checks the free state is the last locked one.
fn check_sliders(moves: &[(usize, f64)], mover: fn(&SimConfig, usize, f64) -> SetField) {
    let plane = oblique_plane();
    let mut st = store(sim(plane.clone()));
    apply(&mut st, set(&plane, 0.2, 0.9).expect("a lock"));
    let anchor = now(&mut st).lock.z_locked;
    for &(k, value) in moves {
        let before = now(&mut st);
        apply(&mut st, [mover(&before, k, value)]);
        let after = now(&mut st);
        for i in 0..8 {
            let want = if i == k { value } else { before.plane.z0[i] };
            assert!(
                after.plane.z0[i].to_bits() == want.to_bits(),
                "the slider {k} did not move along its control: z₀[{i}] = {} (want {want})",
                after.plane.z0[i]
            );
        }
        assert!(
            same_bits(&after.lock.z_locked, &anchor),
            "the slider moved the anchor"
        );
        let shown = slider(&after, k).expect("a control");
        assert_eq!(
            shown.value, value,
            "the slider shows {} not z₀",
            shown.value
        );
        assert_eq!(
            shown.anchor,
            Some(anchor[k]),
            "the anchor is not ghost-marked"
        );
        assert_eq!(shown.offset, Some(value - anchor[k]), "the offset is not δ");
    }
    let last = now(&mut st);
    apply(&mut st, [unlock(&last).expect("locked")]);
    let free = now(&mut st);
    assert!(!free.lock.locked, "unlock left the flag set");
    assert_eq!(free.lock.z_locked, [0.0; 8], "unlock left the anchor");
    assert_eq!(
        free.plane, last.plane,
        "unlock did not keep the last locked (z₀, q₁, q₂)"
    );
    let shown = slider(&free, 0).expect("a control");
    assert_eq!(
        (shown.anchor, shown.offset),
        (None, None),
        "a free slider is re-based"
    );
    assert_eq!(unlock(&free), Err(LockError::NotLocked));
}

#[test]
fn lock_sliders() {
    check_sliders(
        &[(0, 0.75), (6, -1.2), (0, 0.5), (7, 2.5), (3, 0.0)],
        |s, k, v| move_slider(s, k, v).expect("a slider"),
    );
    let s = sim(oblique_plane());
    assert!(
        move_slider(&s, 7, 1.0).is_ok(),
        "the last control is refused"
    );
    assert_eq!(move_slider(&s, 8, 1.0), Err(LockError::NoSuchControl(8)));
    assert_eq!(slider(&s, 8), Err(LockError::NoSuchControl(8)));
    assert_eq!(move_slider(&s, 2, f64::NAN), Err(LockError::NonFinite));
    assert_eq!(slider(&s, 7).expect("a control").value, s.plane.z0[7]);
}

negative_control!(
    lock_sliders,
    "a frozen slider, which ignores the move, is no excursion",
    expected = "did not move along its control",
    check_sliders(&[(0, 0.75)], |s, _, _| SetField {
        edit: Edit::Sim(SimField::Z0(s.plane.z0)),
        no_history: false,
    })
);

// ---- lock_setfields (REQ-GUI-007) ----

/// Each edit is an undoable `SimConfig` edit of `z₀`, the lock group or, on a nonlinear chart, the window, and the
/// store records it.
fn check_setfields(edits: &[SetField]) {
    for e in edits {
        assert!(
            !e.no_history,
            "the lock edit {e:?} is marked no history: not undoable"
        );
        assert!(
            matches!(
                e.edit,
                Edit::Sim(SimField::Z0(_) | SimField::Lock(_) | SimField::Window(_))
            ),
            "the lock edit {e:?} is not on z₀, the lock group or the window"
        );
    }
}

#[test]
fn lock_setfields() {
    let plane = oblique_plane();
    let free = sim(plane.clone());
    let mut st = store(free.clone());
    let lock = set(&plane, 0.3, 0.6).expect("a lock");
    check_setfields(&lock);
    assert!(
        matches!(lock[0].edit, Edit::Sim(SimField::Z0(_)))
            && matches!(lock[1].edit, Edit::Sim(SimField::Lock(_))),
        "the recentring is not a SetField on z₀ before the lock group's"
    );
    apply(&mut st, lock);
    let locked = now(&mut st);
    let more = [
        move_slider(&locked, 1, 0.4).expect("a slider"),
        snap_back(&locked).expect("locked"),
        unlock(&locked).expect("locked"),
    ];
    check_setfields(&more);
    let chart =
        set_on_chart::<Defaults, _, _>(&mixed(), &SQUARE, 0.4, 0.3, phys, &tol()).expect("a lock");
    check_setfields(&chart.edits);
    let nearest = Nearest {
        lz: 0.0,
        k: 0.0,
        z: [0.1; 8],
    };
    check_setfields(&recentre(&nearest));
    // Undoable (R-69). The lock is one command, sent under one history key as one undo entry (gui_state_contract §2,
    // R-466; `lock_window_undo`); sent here under none, it undoes edit by edit, and under a key the second undo finds
    // nothing left of the lock.
    st.undo();
    st.undo();
    assert_eq!(now(&mut st), free, "undo did not restore the free state");
    st.redo();
    st.redo();
    assert_eq!(now(&mut st), locked, "redo did not restore the lock");
}

negative_control!(
    lock_setfields,
    "a lock sent as a no-history edit is not undoable",
    expected = "not undoable",
    check_setfields(&[SetField {
        edit: Edit::Sim(SimField::Lock(Lock {
            locked: true,
            z_locked: [0.0; 8],
        })),
        no_history: true,
    }])
);

negative_control!(
    lock_setfields_basis,
    "a lock that edits the basis is not a lock edit",
    expected = "is not on z₀, the lock group or the window",
    check_setfields(&[SetField {
        edit: Edit::Sim(SimField::Basis { q1: e(0), q2: e(1) }),
        no_history: false,
    }])
);

// ---- lock_kernel_uniform_identical (REQ-SYS-012) ----

/// The decode uniform's words for `chart`.
fn uniform_bits(chart: &LatentChart<f32>) -> Vec<u32> {
    use kernel::stage::decode::UniformChart;
    chart
        .uniform()
        .iter()
        .flat_map(|r| r.to_array())
        .map(f32::to_bits)
        .collect()
}

/// The kernel's uniform for `a` and for `b`, the same plane, is one, bit for bit.
fn check_uniform(a: &SimConfig, b: &SimConfig, kernel: fn(&SimConfig) -> LatentChart<f32>) {
    assert!(
        uniform_bits(&kernel(a)) == uniform_bits(&kernel(b)),
        "the lock reached the kernel's uniform"
    );
}

#[test]
fn lock_kernel_uniform_identical() {
    // The uniform's fields: the chart's rows, the flags word and the energy row; a field added fails to compile here.
    let DecodeUniforms {
        chart,
        flags,
        energy,
    } = DecodeUniforms::default();
    let _ = (chart, flags, energy);
    assert_eq!(std::mem::size_of::<DecodeUniforms>(), 6 * 16 + 16 + 16);
    let free = sim(oblique_plane());
    let mut locked = free.clone();
    locked.lock = Lock {
        locked: true,
        z_locked: [0.9, -0.1, 0.3, 0.2, -0.6, 0.4, 0.05, 0.7],
    };
    check_uniform(&free, &locked, kernel_chart);
    let other = Lock {
        z_locked: free.plane.z0,
        ..locked.lock.clone()
    };
    check_uniform(
        &locked,
        &SimConfig {
            lock: other,
            ..locked.clone()
        },
        kernel_chart,
    );
    let c = kernel_chart(&free);
    assert_eq!(
        c.z0,
        free.plane.z0.map(|x| x as f32),
        "z₀ is not the kernel's"
    );
    assert_eq!(
        c.q1,
        free.plane.q1.map(|x| x as f32),
        "q₁ is not the kernel's"
    );
    assert_eq!(
        c.q2,
        free.plane.q2.map(|x| x as f32),
        "q₂ is not the kernel's"
    );
}

negative_control!(
    lock_kernel_uniform_identical,
    "a kernel chart that carries the lock flag in z₀ differs locked and free",
    expected = "the lock reached the kernel's uniform",
    {
        let free = sim(oblique_plane());
        let mut locked = free.clone();
        locked.lock.locked = true;
        check_uniform(&free, &locked, |s| {
            let mut c = kernel_chart(s);
            c.z0[0] += f32::from(u8::from(s.lock.locked));
            c
        });
    }
);

// ---- lock_infeasible_tilt (REQ-CHART-010) ----

/// The `(L_z, E)` chart frozen at the hidden coordinate `z_α`, the rest of [`invariant_le`]'s.
fn le_at(z_alpha: f64) -> InvariantLE<f64> {
    let mut c = invariant_le();
    c.0.z_cfg[0] = z_alpha;
    c
}

/// The locked `(L_z, E)` view's pixels, its range mapped linearly: the anchor `(L₀, E₀)`, `n × n` pixels at window
/// offsets `x, y ∈ [−1, 1]`, half-widths `(h_l, h_e)`, the horizontal axis tilted by `tau` degrees into the hidden
/// `z_α`, `span` its units (R-473's single-angle rotation: `(a, d) = (cos τ·x, sin τ·x)`), and the centre moved by an
/// excursion `ex` along `z_α`. `own` reads each pixel at its own `z_α`, as lock validation must; otherwise at the
/// anchor's. Returns the pixels and the centre pixel's index.
fn tilted_view(tau: f64, ex: f64, own: bool) -> (Vec<Pixel<InvariantLE<f64>>>, usize) {
    let anchor = invariant_le();
    let x0 = anchor.decode_at::<Defaults, f64>(0.98, 0.45).ic;
    let inv = invariants(&x0);
    let (l0, e0) = (inv.lz, inv.e);
    let (h_l, h_e, span, n) = (0.01 * l0.abs(), 0.01 * inv.k, 1.5, 9);
    let (sin, cos) = tau.to_radians().sin_cos();
    let mut pixels = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let x = -1.0 + 2.0 * i as f64 / (n - 1) as f64;
            let y = -1.0 + 2.0 * j as f64 / (n - 1) as f64;
            let z_alpha = anchor.0.z_cfg[0] + if own { sin * x * span + ex } else { 0.0 };
            pixels.push(Pixel {
                chart: le_at(z_alpha),
                lz: l0 + cos * x * h_l,
                y: e0 + y * h_e,
            });
        }
    }
    (pixels, n * n / 2)
}

/// The tilt, in degrees, that carries part of the view past the parabola.
const TILT: f64 = 60.0;

/// The excursion along `z_α` that carries the centre out: to the tilted view's edge, `x = ±1`, on the side the tilt
/// tags.
fn excursion() -> f64 {
    let span = 1.5;
    let (pixels, c) = tilted_view(TILT, 0.0, true);
    let right = validate::<Defaults, _>(&pixels[c + 4..c + 5], &pixels[c]).infeasible();
    TILT.to_radians().sin() * span * if right == 1 { 1.0 } else { -1.0 }
}

/// Checks lock validation's tags of a locked view: untilted every pixel feasible; tilted, pixels tagged past the
/// parabola, none dropped, each tag's nearest point feasible on `L_z`'s side, and the centre kept.
fn check_infeasible_tilt(own: bool) {
    let (flat, c) = tilted_view(0.0, 0.0, own);
    let untilted = validate::<Defaults, _>(&flat, &flat[c]);
    assert_eq!(untilted.infeasible(), 0, "the untilted view is infeasible");
    let (pixels, c) = tilted_view(TILT, 0.0, own);
    let report = validate::<Defaults, _>(&pixels, &pixels[c]);
    assert_eq!(report.tags.len(), pixels.len(), "a pixel was dropped");
    assert!(report.infeasible() > 0, "no pixel tagged past the parabola");
    assert_eq!(
        report.centre,
        Tag::Feasible,
        "the tilt moved the lock off its fibre"
    );
    let mut nearest = 0;
    for (i, tag) in report.tags.iter().enumerate() {
        let Tag::Infeasible { failed, nearest: n } = tag else {
            continue;
        };
        assert_eq!(
            *failed,
            Constraint::LzEParabola,
            "pixel {i} failed {failed:?}"
        );
        let Some(n) = n else { continue };
        nearest += 1;
        assert_eq!(
            feasibility(n.lz, n.k, Constraint::LzEParabola, Constraint::LzEParabola),
            Feasibility::Valid,
            "pixel {i}'s nearest feasible point is not feasible"
        );
        assert_eq!(
            n.lz.signum(),
            pixels[i].lz.signum(),
            "pixel {i}'s nearest point changed L_z's sign"
        );
    }
    assert!(
        nearest > 1,
        "fewer than two tagged pixels have a nearest point"
    );
}

#[test]
fn lock_infeasible_tilt() {
    check_infeasible_tilt(true);
}

negative_control!(
    lock_infeasible_tilt,
    "pixels read at the anchor's configuration, not their own, are never carried past the parabola",
    expected = "no pixel tagged past the parabola",
    check_infeasible_tilt(false)
);

/// Checks the offer, `offer`, of a locked view (R-593): untilted, no notice; tilted, many pixels tagged and the centre
/// the anchor, one notice offering only the switch to the latent chart; tilted with an excursion that carries the
/// centre out, one notice offering the re-centre to the centre's own nearest feasible point, a lock there.
fn check_offer(offer: fn(&Report) -> Option<Notice>) {
    let (flat, c) = tilted_view(0.0, 0.0, true);
    assert_eq!(
        offer(&validate::<Defaults, _>(&flat, &flat[c])),
        None,
        "a feasible view is given a notice"
    );
    let (pixels, c) = tilted_view(TILT, 0.0, true);
    let report = validate::<Defaults, _>(&pixels, &pixels[c]);
    assert!(
        report.infeasible() > 1,
        "the tilt tagged fewer than two pixels"
    );
    assert_eq!(
        offer(&report),
        Some(Notice { recentre: None }),
        "a tilt alone, the centre the anchor, is not offered only the switch to the latent chart"
    );
    let (out, c) = tilted_view(TILT, excursion(), true);
    let carried = validate::<Defaults, _>(&out, &out[c]);
    let Tag::Infeasible {
        failed: Constraint::LzEParabola,
        nearest: Some(n),
    } = carried.centre
    else {
        panic!(
            "the excursion did not carry the centre out: {:?}",
            carried.centre
        );
    };
    assert_eq!(
        carried.tags[c], carried.centre,
        "the centre pixel's tag is not the centre's"
    );
    assert!(
        carried.infeasible() > 1,
        "the excursion's view has no tagged pixel but the centre"
    );
    assert_eq!(
        offer(&carried),
        Some(Notice { recentre: Some(n) }),
        "the centre carried out is not offered its own nearest feasible point"
    );
    assert_eq!(
        feasibility(n.lz, n.k, Constraint::LzEParabola, Constraint::LzEParabola),
        Feasibility::Valid,
        "the centre's nearest feasible point is not feasible"
    );
    let [z0, _] = recentre(&n);
    assert!(matches!(z0.edit, Edit::Sim(SimField::Z0(z)) if same_bits(&z, &n.z)));
}

#[test]
fn lock_infeasible_tilt_offer() {
    check_offer(Report::offer);
}

negative_control!(
    lock_infeasible_tilt_offer,
    "an offer of a tagged pixel's nearest point, not the centre's, re-centres a tilt whose centre is the anchor",
    expected = "a tilt alone, the centre the anchor, is not offered only the switch",
    {
        /// The per-pixel offer R-593 refuses, as one notice: the first tagged pixel's nearest feasible point.
        fn first_tagged(report: &Report) -> Option<Notice> {
            let first = report.tags.iter().find_map(|t| match t {
                Tag::Infeasible { nearest, .. } => Some(*nearest),
                _ => None,
            })?;
            Some(Notice { recentre: first })
        }
        check_offer(first_tagged)
    }
);

/// The tags' arms, and the offer's: a pair below the apex has no nearest point that keeps `L_z`'s class; a notice
/// wherever a pixel or the centre is tagged infeasible, none where neither is, a degenerate pixel counted neither; a
/// re-centre only to the centre's own nearest point.
#[test]
fn lock_infeasible_tilt_tags() {
    let chart = invariant_le();
    let u = potential(&chart.decode_at::<Defaults, f64>(0.5, 0.0).ic);
    let feasible = Pixel {
        chart,
        lz: 0.0,
        y: u + 0.5,
    };
    let report = validate::<Defaults, _>(
        &[
            feasible,
            Pixel {
                chart,
                lz: 0.3,
                y: u - 0.1,
            },
            Pixel {
                chart,
                lz: 2.0,
                y: u + 0.5,
            },
            Pixel {
                chart,
                lz: f64::NAN,
                y: u,
            },
        ],
        &feasible,
    );
    assert_eq!(report.tags[0], Tag::Feasible);
    assert_eq!(
        report.centre,
        Tag::Feasible,
        "the centre is not read like a pixel"
    );
    assert_eq!(
        report.tags[1],
        Tag::Infeasible {
            failed: Constraint::LzEParabola,
            nearest: None
        },
        "a pair below the apex with L_z ≠ 0 has a nearest point"
    );
    let Tag::Infeasible {
        nearest: Some(near),
        ..
    } = report.tags[2]
    else {
        panic!(
            "a pair past the parabola has no nearest point: {:?}",
            report.tags[2]
        );
    };
    assert!(matches!(
        report.tags[3],
        Tag::Infeasible { nearest: None, .. }
    ));
    assert_eq!(report.infeasible(), 3);
    assert_eq!(
        report.offer(),
        Some(Notice { recentre: None }),
        "a feasible centre is offered a tagged pixel's re-centre"
    );
    let offer = |tags: Vec<Tag>, centre: Tag| Report { tags, centre }.offer();
    let (with, without) = (report.tags[2], report.tags[1]);
    // One tagged pixel, the fewest that gives a notice; none, no notice, the centre feasible or degenerate.
    assert_eq!(
        Report {
            tags: vec![Tag::Feasible, with, Tag::Degenerate],
            centre: Tag::Feasible
        }
        .infeasible(),
        1,
        "a degenerate pixel is counted infeasible"
    );
    for centre in [Tag::Feasible, Tag::Degenerate] {
        assert_eq!(
            offer(vec![Tag::Feasible, with], centre),
            Some(Notice { recentre: None }),
            "one tagged pixel, the centre {centre:?}, is not one notice of the switch alone"
        );
        assert_eq!(
            offer(vec![Tag::Feasible, Tag::Degenerate], centre),
            None,
            "an untagged view, the centre {centre:?}, is given a notice"
        );
    }
    // The centre tagged: its own nearest point, or none, whatever the other pixels are.
    assert_eq!(
        offer(vec![Tag::Feasible, without], with),
        Some(Notice {
            recentre: Some(near)
        }),
        "the centre carried out is not offered its own nearest point"
    );
    assert_eq!(
        offer(vec![Tag::Feasible], with),
        Some(Notice {
            recentre: Some(near)
        }),
        "a tagged centre alone is given no notice"
    );
    assert_eq!(
        offer(vec![with], without),
        Some(Notice { recentre: None }),
        "a centre with no nearest point is offered a tagged pixel's"
    );
}

negative_control!(
    lock_infeasible_tilt_tags,
    "a feasible pair is not tagged",
    expected = "has a nearest point",
    {
        let chart = invariant_le();
        let u = potential(&chart.decode_at::<Defaults, f64>(0.5, 0.0).ic);
        let p = Pixel {
            chart,
            lz: 0.0,
            y: u + 0.5,
        };
        let report = validate::<Defaults, _>(&[p], &p);
        assert_eq!(
            report.tags[0],
            Tag::Infeasible {
                failed: Constraint::LzEParabola,
                nearest: None
            },
            "a pair below the apex with L_z ≠ 0 has a nearest point"
        );
    }
);

// ---- curve_tilt_requires_lock (REQ-CHART-007) ----

/// `softmax(0, μ₁, μ₂)`.
fn softmax(mu1: f64, mu2: f64) -> [f64; 3] {
    let e = [1.0, mu1.exp(), mu2.exp()];
    let s = e[0] + e[1] + e[2];
    e.map(|x| x / s)
}

/// The latent embed of the Burrau curve under `S`: each control its link's inverse at `γ(ν)`.
fn latent_embed<S: Selection>(nu: f64) -> Latent {
    let CurvePoint {
        alpha,
        beta,
        mu1,
        mu2,
    } = nu_embed(nu);
    let [za] = <S::AlphaLink as Link<1, 1>>::inverse([alpha]);
    let [zb] = <S::BetaLink as Link<1, 1>>::inverse([beta]);
    let [z6, z7] = <S::MassLink as Link<2, 3>>::inverse(softmax(mu1, mu2));
    [za, zb, 0.0, 0.0, 0.0, 0.0, z6, z7]
}

fn unit(a: &Latent) -> Latent {
    let n = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    a.map(|x| x / n)
}

/// Checks the tangent tilt at `nu0` under `S`: refused free; locked, `τ = 0` is the latent embed's tangent at `nu0`,
/// by an independent central difference, and `τ = 90°` the target.
fn check_curve_tilt<S>(nu0: f64, at: f64)
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    let free = Lock {
        locked: false,
        z_locked: [0.0; 8],
    };
    let target = e(3);
    assert_eq!(
        curve_tilt::<S>(&free, &NU_CURVE, nu0, 1.0, 30.0, &target),
        Err(LockError::NotLocked),
        "a free curve-axis tilt is not refused"
    );
    let locked = Lock {
        locked: true,
        z_locked: latent_embed::<S>(nu0),
    };
    let h = 1e-6;
    let want = unit(&std::array::from_fn(|i| {
        latent_embed::<S>(at + h)[i] - latent_embed::<S>(at - h)[i]
    }));
    let got = curve_tilt::<S>(&locked, &NU_CURVE, nu0, 0.7, 0.0, &target).expect("a tilt");
    let got = unit(&got);
    let d: f64 = got
        .iter()
        .zip(&want)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    assert!(
        d <= 1e-7,
        "the tilt at τ = 0 is not γ′(ν₀) at ν₀ = {nu0}: {got:?} against {want:?}"
    );
    let raw = curve_tangent::<S>(&NU_CURVE, nu0).expect("a tangent");
    assert!(
        raw[2..6].iter().all(|&x| x == 0.0),
        "a rest curve's tangent has momenta"
    );
    let full = curve_tilt::<S>(&locked, &NU_CURVE, nu0, 0.7, 90.0, &target).expect("a tilt");
    let d: f64 = full
        .iter()
        .zip(&scale(&target, 0.7))
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    assert!(
        d <= 1e-12,
        "the tilt at τ = 90° is not the target: {full:?}"
    );
}

#[test]
fn curve_tilt_requires_lock() {
    for nu0 in [0.2, 0.5, 0.83] {
        check_curve_tilt::<Defaults>(nu0, nu0);
        check_curve_tilt::<Sticks>(nu0, nu0);
    }
    assert_eq!(
        curve_tangent::<Defaults>(&NU_CURVE, f64::NAN),
        Err(LockError::NonFinite)
    );
    let locked = Lock {
        locked: true,
        z_locked: [0.0; 8],
    };
    assert!(
        matches!(
            curve_tilt::<Defaults>(&locked, &NU_CURVE, 0.5, 1.0, 120.0, &e(3)),
            Err(LockError::Nav(_))
        ),
        "a tilt past 90° is not refused"
    );
    // A curve point that moves no mass: its mass tangent is 0, not a difference of nothing.
    let still = engine::chart_authoring::CurveMeta {
        parameter: "s",
        embed: |s| CurvePoint {
            alpha: 0.3 + 0.1 * s,
            beta: 1.0,
            mu1: 0.2,
            mu2: -0.1,
        },
        tangent: |_| CurvePoint {
            alpha: 0.1,
            beta: 0.0,
            mu1: 0.0,
            mu2: 0.0,
        },
    };
    let t = curve_tangent::<Defaults>(&still, 0.5).expect("a tangent");
    assert_eq!(
        &t[6..],
        &[0.0, 0.0],
        "a curve that moves no mass has a mass tangent"
    );
    assert!(t[0] > 0.0 && t[1] == 0.0);
    check_mass_step();
}

/// The mass controls' tangent is the mass link's inverse differenced along `(dμ₁, dμ₂)` at the contract's step
/// `h = ε^{1/3}·(1 + ‖μ‖)/‖dμ‖`, to rounding; and at positive logits, where the softmax's shift acts, it is the
/// derivative of the mass link's inverse.
fn check_mass_step() {
    let nu = 0.37;
    let (p, d) = (nu_embed(nu), (NU_CURVE.tangent)(nu));
    let h = f64::EPSILON.cbrt() * (1.0 + p.mu1.hypot(p.mu2)) / d.mu1.hypot(d.mu2);
    let at = |k: f64| {
        let m = softmax(p.mu1 + k * d.mu1, p.mu2 + k * d.mu2);
        <SoftmaxTanh as Link<2, 3>>::inverse(m)
    };
    let (hi, lo) = (at(h), at(-h));
    let want = [(hi[0] - lo[0]) / (2.0 * h), (hi[1] - lo[1]) / (2.0 * h)];
    let got = curve_tangent::<Defaults>(&NU_CURVE, nu).expect("a tangent");
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-13 * b.abs().max(1.0);
    assert!(
        close(got[6], want[0]) && close(got[7], want[1]),
        "the mass tangent {:?} is not the step's {want:?}",
        &got[6..]
    );
    let heavy = engine::chart_authoring::CurveMeta {
        parameter: "s",
        embed: |s| CurvePoint {
            alpha: 0.6,
            beta: 1.2,
            mu1: 1.3 + 0.5 * s,
            mu2: 0.4 - 0.2 * s,
        },
        tangent: |_| CurvePoint {
            alpha: 0.0,
            beta: 0.0,
            mu1: 0.5,
            mu2: -0.2,
        },
    };
    let z = |s: f64| <SoftmaxTanh as Link<2, 3>>::inverse(softmax(1.3 + 0.5 * s, 0.4 - 0.2 * s));
    let (hi, lo) = (z(1e-5), z(-1e-5));
    let got = curve_tangent::<Defaults>(&heavy, 0.0).expect("a tangent");
    for k in 0..2 {
        let want = (hi[k] - lo[k]) / 2e-5;
        assert!(
            (got[6 + k] - want).abs() <= 1e-8 * want.abs().max(1.0),
            "the mass tangent {:?} at positive logits is not the inverse's derivative",
            &got[6..]
        );
    }
}

negative_control!(
    curve_tilt_requires_lock,
    "the tangent at another point of the curve is not γ′(ν₀)",
    expected = "is not γ′(ν₀)",
    check_curve_tilt::<Defaults>(0.2, 0.6)
);

/// The controls named, by index.
fn named(controls: [bool; 8]) -> Vec<usize> {
    (0..8).filter(|&k| controls[k]).collect()
}

/// A curve at one point, `p`, moving along `d`.
macro_rules! at_point {
    ($p:expr, $d:expr) => {
        engine::chart_authoring::CurveMeta {
            parameter: "s",
            embed: |_| $p,
            tangent: |_| $d,
        }
    };
}

/// The tangent tilt at `nu0` on the Burrau curve under `S` is refused, naming exactly the controls `want`, both bare and
/// locked; `want` empty, it is a tangent.
fn check_saturated<S>(nu0: f64, want: &[usize])
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    let got = curve_tangent::<S>(&NU_CURVE, nu0);
    match got {
        Err(LockError::Saturated { controls }) => assert_eq!(
            named(controls),
            want,
            "the tangent at ν₀ = {nu0} names the wrong controls"
        ),
        Ok(_) => assert!(
            want.is_empty(),
            "the tangent at ν₀ = {nu0}, where {want:?} clamp, is not refused"
        ),
        other => panic!("the tangent at ν₀ = {nu0} gives {other:?}"),
    }
    let locked = Lock {
        locked: true,
        z_locked: [0.0; 8],
    };
    assert_eq!(
        curve_tilt::<S>(&locked, &NU_CURVE, nu0, 1.0, 10.0, &e(3)).err(),
        got.err(),
        "the locked tilt does not refuse as the tangent does"
    );
}

#[test]
fn curve_tilt_requires_lock_refuses_a_saturated_control() {
    // Near ν = 1 the mass link's inverse clamps `μ₂`, and near ν = 0 `μ₁` (physics review 5640537524 of PR #290, P2).
    check_saturated::<Defaults>(0.995, &[7]);
    check_saturated::<Defaults>(0.999, &[7]);
    check_saturated::<Defaults>(0.003, &[6]);
    check_saturated::<Defaults>(0.5, &[]);
    let cap = (1.0 - <f64 as ChartConstants>::EPS_MU) * <f64 as ChartConstants>::MU_MAX;
    // The curve point 1e-5 inside the clamp, and the step `h·‖dμ‖ = ε^{1/3}·(1 + ‖μ‖)`, about 3.6e-5, past it: refused
    // at a difference point.
    let edge = at_point!(
        CurvePoint {
            alpha: 0.6,
            beta: 1.2,
            mu1: 0.0,
            mu2: 1e-5 - (1.0 - <f64 as ChartConstants>::EPS_MU) * <f64 as ChartConstants>::MU_MAX,
        },
        CurvePoint {
            alpha: 0.0,
            beta: 0.0,
            mu1: 0.0,
            mu2: -1.0,
        }
    );
    let p = (edge.embed)(0.0);
    assert!(
        p.mu2 > -cap,
        "the edge curve's point is past the clamp: the case does not straddle"
    );
    assert_eq!(
        <SoftmaxTanh as SimplexClamp>::clamps(softmax(p.mu1, p.mu2)),
        0,
        "the edge curve's point clamps: the case does not straddle"
    );
    assert_eq!(
        curve_tangent::<Defaults>(&edge, 0.0),
        Err(LockError::Saturated {
            controls: [false, false, false, false, false, false, false, true],
        }),
        "a step past the clamp is not refused"
    );
    // The configuration's links: `α` at 0 and `β` at `π` clamp.
    let flat = at_point!(
        CurvePoint {
            alpha: 0.0,
            beta: 1.2,
            mu1: 0.0,
            mu2: 0.0,
        },
        CurvePoint {
            alpha: 0.1,
            beta: 0.1,
            mu1: 0.0,
            mu2: 0.0,
        }
    );
    assert_eq!(
        curve_tangent::<Defaults>(&flat, 0.0).map_err(|e| match e {
            LockError::Saturated { controls } => named(controls),
            other => panic!("{other:?}"),
        }),
        Err(vec![0])
    );
    let collinear = at_point!(
        CurvePoint {
            alpha: 0.6,
            beta: std::f64::consts::PI,
            mu1: 0.0,
            mu2: 0.0,
        },
        CurvePoint {
            alpha: 0.1,
            beta: 0.1,
            mu1: 0.0,
            mu2: 0.0,
        }
    );
    assert_eq!(
        curve_tangent::<Defaults>(&collinear, 0.0).map_err(|e| match e {
            LockError::Saturated { controls } => named(controls),
            other => panic!("{other:?}"),
        }),
        Err(vec![1])
    );
    check_both_steps_clamp(<StickBreaking as SimplexClamp>::clamps::<f64>, |mu| {
        curve_tangent::<Sticks>(&ACROSS, mu)
    });
}

/// The curve `μ₁ = μ₂ = s`, moving along `(dμ₁, dμ₂) = (1, −1)`, the configuration held inside its clamps.
const ACROSS: engine::chart_authoring::CurveMeta = engine::chart_authoring::CurveMeta {
    parameter: "s",
    embed: |s| CurvePoint {
        alpha: 0.6,
        beta: 1.2,
        mu1: s,
        mu2: s,
    },
    tangent: |_| CurvePoint {
        alpha: 0.0,
        beta: 0.0,
        mu1: 1.0,
        mu2: -1.0,
    },
};

/// Under the stick-breaking link, `z_μ1`'s inverse clamps where `m₀ = 1/(1 + e^μ₁ + e^μ₂)` falls below
/// `T = (1 − (1 − ε_μ)²)/2`. Along `(1, −1)` from `μ₁ = μ₂ = μ`, `m₀` is largest at the curve point, so with `μ` placed
/// a quarter of the step's drop, `h²/4`, above the clamp's edge, the curve point stays inside and both difference
/// points `μ ± h·dμ` fall past it: the step clamps at both ends and not between. `clamps` is the link's clamp test and
/// `tangent` the tangent of [`ACROSS`] at `μ`, which must be refused naming `z_μ1` alone.
fn check_both_steps_clamp(
    clamps: fn([f64; 3]) -> u32,
    tangent: fn(f64) -> Result<Latent, LockError>,
) {
    let keep = 1.0 - <f64 as ChartConstants>::EPS_MU;
    let edge = (1.0 - keep * keep) / 2.0;
    let mu_edge = ((1.0 / edge - 1.0) / 2.0).ln();
    // The contract's step, `ε^{1/3}·(1 + ‖μ‖)/‖dμ‖`, at `‖μ‖ = √2·μ` and `‖dμ‖ = √2`.
    let h =
        f64::EPSILON.cbrt() * (1.0 + std::f64::consts::SQRT_2 * mu_edge) / std::f64::consts::SQRT_2;
    let mu = ((1.0 - h * h / 4.0) / edge - 1.0).ln() - std::f64::consts::LN_2;
    let at = |k: f64| softmax(mu + k, mu - k);
    assert_eq!(
        (clamps(at(0.0)), clamps(at(h)), clamps(at(-h))),
        (0, 1, 1),
        "the case does not clamp at both difference points and not at the curve point (μ = {mu}, h = {h})"
    );
    assert_eq!(
        tangent(mu),
        Err(LockError::Saturated {
            controls: [false, false, false, false, false, false, true, false],
        }),
        "a step that clamps at both ends is not refused"
    );
}

negative_control!(
    curve_tilt_requires_lock_refuses_a_saturated_control,
    "the tangent at ν₀ = 0.5, inside every clamp, is no refusal",
    expected = "is not refused",
    check_saturated::<Defaults>(0.5, &[7])
);

negative_control!(
    curve_tilt_requires_lock_refuses_a_saturated_control_both_ends,
    "the curve a unit inside the clamp, where no difference point reaches it, is no refusal",
    expected = "is not refused",
    check_both_steps_clamp(<StickBreaking as SimplexClamp>::clamps::<f64>, |mu| {
        curve_tangent::<Sticks>(&ACROSS, mu - 1.0)
    })
);

// ---- lock_window_undo (REQ-SYS-011; TASK-M2-21) ----

/// How a lock's command reaches the store.
type Send = fn(&mut StateStore, [SetField; 3]);

/// The command as the binding sends it: every edit in one frame under one fresh history key (gui_state_contract §2,
/// R-466).
fn one_key(st: &mut StateStore, edits: [SetField; 3]) {
    for e in edits {
        st.set_field_under(e, HistoryKey(1));
    }
}

/// Locks the mixed chart at `(0.3, 0.8)` of a zoomed window held in the store, sends the lock's command with `send`, and
/// checks: the store's window is the re-centred one; one undo restores `z₀`, the lock group and the window, leaving no
/// entry; one redo sets all three again.
fn check_window_undo(send: Send) {
    let window = Window {
        centre: [0.375, 0.625],
        half: [0.125, 0.25],
    };
    let mut free = sim(oblique_plane());
    free.chart.window = window;
    let mut st = store(free.clone());
    let lock =
        set_on_chart::<Defaults, _, _>(&mixed(), &window, 0.3, 0.8, phys, &tol()).expect("a lock");
    assert_ne!(lock.window, window, "the lock did not move the window");
    send(&mut st, lock.edits.clone());
    let locked = now(&mut st);
    assert_eq!(
        locked.chart.window, lock.window,
        "the store's window is not the re-centred one"
    );
    assert!(locked.lock.locked, "the store is not locked");
    st.undo();
    let undone = st.snapshot();
    assert_eq!(
        undone.sim.chart.window, free.chart.window,
        "one undo did not restore the window"
    );
    assert!(
        same_bits(&undone.sim.plane.z0, &free.plane.z0),
        "one undo did not restore z₀"
    );
    assert_eq!(
        undone.sim.lock, free.lock,
        "one undo did not restore the lock group"
    );
    assert_eq!(
        undone.history.undo_depth, 0,
        "the lock's command is not one history entry"
    );
    st.redo();
    assert_eq!(now(&mut st), locked, "one redo did not set the lock again");
}

#[test]
fn lock_window_undo() {
    check_window_undo(one_key);
}

negative_control!(
    lock_window_undo,
    "the window edit sent under a second key is a second history entry, so one undo leaves z₀ and the lock set",
    expected = "one undo did not restore z₀",
    check_window_undo(|st, [z0, lock, window]| {
        st.set_field_under(z0, HistoryKey(1));
        st.set_field_under(lock, HistoryKey(1));
        st.set_field_under(window, HistoryKey(2));
    })
);

negative_control!(
    lock_window_undo_left_out,
    "the window edit left out of the command, sent alone after it, is undone alone",
    expected = "one undo did not restore z₀",
    check_window_undo(|st, [z0, lock, window]| {
        st.set_field_under(z0, HistoryKey(1));
        st.set_field_under(lock, HistoryKey(1));
        st.set_field(window);
    })
);

negative_control!(
    lock_window_undo_dropped,
    "a lock whose command drops the window edit leaves the window where it was",
    expected = "the store's window is not the re-centred one",
    check_window_undo(|st, [z0, lock, _]| {
        st.set_field_under(z0, HistoryKey(1));
        st.set_field_under(lock, HistoryKey(1));
    })
);
