//! Today's operations through the chart-path object, bit for bit against the baseline recorded from the
//! code they replaced (`fixtures/chart_composition/digest.json`, written on this PR's baseline commits from `main`'s
//! code, before the re-expression; `crates/engine/tests/path_support/mod.rs` says what a digest holds): every case of pan,
//! zoom, slice, raw tilt (one vector and the pair, from an untilted and from a tilted plane), the oblique slice, the lock
//! (on the affine chart and on three nonlinear charts, its refusals, snap-back, unlock, recentre, a slider's move, the
//! kernel's chart, a curve axis's tangent tilt), the chart switch's end views, Save view as chart (latent, tilted and on
//! four curved parents, reopened) and the gui's six presets at their homes, each now a `engine::path::Segment` built by
//! its constructor, or a `engine::path::View`.
//!
//! **The criterion**, written before the first run: every case's digest equals the stored one exactly, text for text,
//! and every stored case is produced. One test per operation reads its own cases; `path_every_case` reads them all.
//! `digest.json` is never written here: the code that wrote it is gone. Stage 2 changes the lock's own cases,
//! locking not moving the view: they are re-recorded in `lock_stage2.json`, which overlays the baseline
//! (`PRINCIPIA_RECORD_LOCK_STAGE2=1`, by hand; the section "The lock's re-recorded cases" below says which and why), and with the
//! centre-on-pin setting on every lock case is still its stage-1 digest.
//!
//! **Controls**: one floating-point operation reordered in the pan family; the general geodesic (a rotation of `q` in
//! the plane of `q` and the target) in the raw tilt family's place; a path triple `(A, B, family, s)` in a zoom's sim
//! key. Each must fail.
//!
//! **Every platform.** Every transcendental the chart path calls rounds correctly (`engine::exact`) or is the pinned
//! pure-software libm's, as the kernel's are, never the platform's own, so the digests are the same bits on Linux, macOS
//! and Windows; the Windows runner runs this file too (`.config/nextest.toml`'s `win-gpu-kernel`).

mod path_shim;
mod path_support;

use std::collections::BTreeMap;

use engine::chart_authoring::charts::NU_CURVE;
use engine::contract::sim_config::{Lock, Plane, Window};
use engine::lock::{curve_tilt, kernel_chart, Nearest};
use engine::lookup::Tolerances;
use engine::nav::directions::hidden;
use engine::nav::BasisVector;
use engine::path::{Map, PathError, Segment, View};
use kernel::chart::saved::Saved;
use kernel::chart::tilt::{TiltMap, TiltSpec};
use kernel::chart::{ChartDecode, ShapeSphere};
use kernel::decode::Ic;
use kernel::encode::by_kind::ByKind;
use kernel::generated::links::Defaults;
use path_shim::{
    oblique, pan, recentre, set_on_chart, set_on_chart_pinned, slice, zoom, ChartConstructor,
};
use path_support::*;
use validation::negative_control;
use validation::roundtrip::phys;

/// What runs each family: the object, or a control's stand-in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Variant {
    /// The object.
    Object,
    /// The object with the centre-on-pin setting on at every lock: the stage-1 lock, which re-centred.
    Centred,
    /// The lock re-centring the view where the setting is off.
    #[cfg(feature = "controls")]
    LockRecentres,
    /// The setting off where the stage-1 lock is checked.
    #[cfg(feature = "controls")]
    CentredSettingOff,
    /// The pan family with one floating-point operation reordered.
    #[cfg(feature = "controls")]
    PanReordered,
    /// The general geodesic in the raw tilt family's place.
    #[cfg(feature = "controls")]
    Geodesic,
    /// A path triple in a zoom's sim key.
    #[cfg(feature = "controls")]
    ZoomTriple,
    /// The slice family without its projection off the plane: `z₀ += t·d`.
    #[cfg(feature = "controls")]
    SliceUnprojected,
    /// The lock's pixel with its sum reordered, `z₀ + b·q₂ + a·q₁`, not the kernel's `Φ`.
    #[cfg(feature = "controls")]
    LockByPan,
    /// The chart switch showing its `A` at its end, as its interior does.
    #[cfg(feature = "controls")]
    SwitchShowsA,
    /// Save view as chart re-orthonormalising the view's vectors.
    #[cfg(feature = "controls")]
    SaveOrthonormal,
    /// A preset reopened through the oblique plane, re-orthonormalised.
    #[cfg(feature = "controls")]
    PresetOblique,
    /// One case not produced.
    #[cfg(feature = "controls")]
    DropOne,
}

/// A digest of the latent view `before` reached by the operation's result.
fn latent<E: std::fmt::Debug>(
    before: &engine::contract::sim_config::SimConfig,
    result: Result<Vec<engine::contract::set_field::SetField>, E>,
    with_image: bool,
) -> String {
    match result {
        Ok(edits) => latent_digest(before, &edits, with_image),
        Err(e) => err_digest(&e).replace('"', ""),
    }
}

/// A segment's edits.
fn edits_of(seg: Segment) -> Vec<engine::contract::set_field::SetField> {
    seg.edits().to_vec()
}

/// The lock's or the navigation's own error, as the stage-1 baseline digested it (the formula's error, not the path's).
fn bare(e: PathError) -> String {
    match e {
        PathError::Lock(e) => format!("{e:?}"),
        PathError::Nav(e) => format!("{e:?}"),
        other => format!("{other:?}"),
    }
}

/// The plane after a zoom by `octaves`, through the object.
fn zoomed(plane: &Plane, octaves: f64) -> Plane {
    after(
        &sim(plane.clone()),
        &[zoom(plane, octaves).expect("a zoom")],
    )
    .plane
}

/// Today's lock on a nonlinear chart, digested: its edits, the clicked state, the notices, and the probe of the chart
/// through the re-centred window.
fn chart_lock<C>(chart: &C, at: (f64, f64, f64), centred: bool) -> String
where
    C: ByKind + engine::lock::FrozenControls + ChartDecode,
    f64: From<C::Param>,
    f64: From<<C as kernel::chart::Chart>::Param>,
{
    let (s, t, h) = at;
    chart_lock_in(
        chart,
        &window([0.5, 0.5], h),
        (s, t),
        &Tolerances::register(128.0 * f64::EPSILON),
        centred,
    )
}

/// [`chart_lock`] through `win` at the pixel `(s, t)` under the tolerances `tol`: the setting off, or on where
/// `centred` (the stage-1 lock: the window re-centred, which the digest's `ic` probes through).
fn chart_lock_in<C>(
    chart: &C,
    win: &Window,
    (s, t): (f64, f64),
    tol: &Tolerances,
    centred: bool,
) -> String
where
    C: ByKind + engine::lock::FrozenControls + ChartDecode,
    f64: From<C::Param>,
    f64: From<<C as kernel::chart::Chart>::Param>,
{
    let digest =
        |edits: &[engine::contract::set_field::SetField], clicked: &Ic<f64>, notices, window| {
            let mut ic = Vec::new();
            ic_words(clicked, &mut ic);
            format!(
                "edits={} clicked={:016x} notices={:?} ic={:016x}",
                edits_words(edits),
                fnv(&ic),
                notices,
                chart_ic(chart, window)
            )
        };
    match centred {
        true => match set_on_chart::<Defaults, C, _>(chart, win, s, t, phys, tol) {
            Ok(lock) => digest(&lock.edits, &lock.ic, lock.notices, &lock.window),
            Err(e) => err_digest(&e),
        },
        false => match set_on_chart_pinned::<Defaults, C, _>(chart, win, s, t, phys, tol) {
            Ok(lock) => digest(&lock.edits, &lock.ic, lock.notices, &lock.window),
            Err(e) => err_digest(&e),
        },
    }
}

/// Every case, through the object, or through a control's stand-in where `variant` names one.
fn record(variant: Variant) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut put = |name: String, digest: String| {
        assert!(out.insert(name.clone(), digest).is_none(), "{name} twice");
    };
    for (pname, plane) in planes() {
        let before = sim(plane.clone());
        let dirs = hidden(&plane).expect("the hidden directions");
        for (i, (ds, dt)) in PANS.iter().enumerate() {
            let r = match variant {
                #[cfg(feature = "controls")]
                Variant::PanReordered => pan_reordered(&plane, *ds, *dt),
                _ => pan(&plane, *ds, *dt),
            }
            .map(|e| vec![e]);
            put(format!("pan/{pname}/{i}"), latent(&before, r, i == 2));
        }
        for (i, k) in ZOOMS.iter().enumerate() {
            let r = zoom(&plane, *k).map(|e| vec![e]);
            let digest = match variant {
                #[cfg(feature = "controls")]
                Variant::ZoomTriple => zoom_with_triple(&before, *k, latent(&before, r, i == 2)),
                _ => latent(&before, r, i == 2),
            };
            put(format!("zoom/{pname}/{i}"), digest);
        }
        let slice_dirs = [
            dirs[0].direction,
            dirs[3].direction,
            SLICE_RAW[0],
            SLICE_RAW[1],
        ];
        for (j, d) in slice_dirs.iter().enumerate() {
            for (i, t) in SLICE_STEPS.iter().enumerate() {
                let r = match variant {
                    #[cfg(feature = "controls")]
                    Variant::SliceUnprojected => slice_unprojected(&plane, d, *t),
                    _ => slice(&plane, d, *t),
                }
                .map(|e| vec![e]);
                put(
                    format!("slice/{pname}/{j}/{i}"),
                    latent(&before, r, j == 0 && i == 0),
                );
            }
        }
        let current = zoomed(&plane, TILT_ZOOM);
        let tilted_from = sim(current.clone());
        let constructor = ChartConstructor::new(plane.clone());
        for (i, (which, tau, k)) in TILTS.iter().enumerate() {
            let which = match which {
                1 => BasisVector::Q1,
                _ => BasisVector::Q2,
            };
            let r = match variant {
                #[cfg(feature = "controls")]
                Variant::Geodesic => geodesic(&plane, &current, which, *tau, &dirs[*k].direction),
                _ => constructor.tilt(&current, which, *tau, &dirs[*k].direction),
            }
            .map(|e| vec![e]);
            put(format!("tilt/{pname}/{i}"), latent(&tilted_from, r, i == 1));
        }
        // Raw tilts replace rather than compose: tilts from a current plane that already carries a raw tilt.
        let pre = constructor
            .tilt(&current, BasisVector::Q1, 30.0, &dirs[0].direction)
            .expect("a tilt");
        let pre_sim = after(&tilted_from, &[pre]);
        for (i, (which, tau, k)) in TILTS.iter().enumerate() {
            let which = match which {
                1 => BasisVector::Q1,
                _ => BasisVector::Q2,
            };
            let r = constructor
                .tilt(&pre_sim.plane, which, *tau, &dirs[*k].direction)
                .map(|e| vec![e]);
            put(
                format!("tilt_retilt/{pname}/{i}"),
                latent(&pre_sim, r, false),
            );
        }
        for (i, ((t1, k1), (t2, k2))) in TILT_PAIRS.iter().enumerate() {
            let r = constructor
                .tilt_pair(
                    &pre_sim.plane,
                    (*t1, &dirs[*k1].direction),
                    (*t2, &dirs[*k2].direction),
                )
                .map(|e| vec![e]);
            put(
                format!("tilt_pair_retilt/{pname}/{i}"),
                latent(&pre_sim, r, false),
            );
        }
        // Save view as chart of the tilted view: the oblique chart of a tilted plane.
        let tilted = &pre_sim.plane;
        let tilted_words = match Segment::save_view(&View::latent(pre_sim.clone()))
            .expect("a save")
            .b()
            .map
            .clone()
        {
            Map::Saved(saved) => saved.to_words(),
            other => panic!("Save view as chart gives {other:?}"),
        };
        put(
            format!("save/latent_tilted/{pname}"),
            format!(
                "words={:016x} view={:016x}",
                fnv(&tilted_words
                    .iter()
                    .flat_map(|w| w.to_bits().to_be_bytes())
                    .collect::<Vec<_>>()),
                latent_ic(tilted)
            ),
        );
        for (i, ((t1, k1), (t2, k2))) in TILT_PAIRS.iter().enumerate() {
            let r = constructor
                .tilt_pair(
                    &current,
                    (*t1, &dirs[*k1].direction),
                    (*t2, &dirs[*k2].direction),
                )
                .map(|e| vec![e]);
            put(
                format!("tilt_pair/{pname}/{i}"),
                latent(&tilted_from, r, i == 1),
            );
        }
        put(
            format!("oblique/{pname}"),
            match oblique(plane.z0, &plane.q1, &plane.q2) {
                Ok(p) => format!(
                    "plane=[{}|{}|{}] ic={:016x}",
                    hxs(&p.z0),
                    hxs(&p.q1),
                    hxs(&p.q2),
                    latent_ic(&p)
                ),
                Err(e) => err_digest(&e),
            },
        );
        // The lock, the setting off (the lock group alone, the view still), or on (`Variant::Centred`: the lock and
        // centre on pin in one command, the stage-1 lock); then a slider's move, snap-back and unlock from the lock
        // at `LOCKS[1]`, each through its segment from the locked view, its pin carried.
        let lock_at = |at: (f64, f64)| -> Result<Segment, PathError> {
            let from = View::latent(before.clone());
            match variant {
                Variant::Centred => Segment::lock_centred(&from, at.0, at.1),
                _ => Segment::lock(&from, at.0, at.1),
            }
        };
        for (i, (s, t)) in LOCKS.iter().enumerate() {
            let r = match variant {
                #[cfg(feature = "controls")]
                Variant::LockByPan => lock_by_pan(&plane, *s, *t)
                    .map(Vec::from)
                    .map_err(PathError::Lock),
                #[cfg(feature = "controls")]
                Variant::LockRecentres => {
                    Segment::lock_centred(&View::latent(before.clone()), *s, *t).map(edits_of)
                }
                _ => lock_at((*s, *t)).map(edits_of),
            };
            put(
                format!("lock/set/{pname}/{i}"),
                latent(&before, r.map_err(bare), i == 1),
            );
        }
        let locked = lock_at(LOCKS[1]).expect("a lock").b().clone();
        for (i, (k, v)) in SLIDERS.iter().enumerate() {
            let r = Segment::control(&locked, *k, *v)
                .map(edits_of)
                .map_err(bare);
            put(
                format!("lock/slider/{pname}/{i}"),
                latent(&locked.sim, r, false),
            );
        }
        let moved = Segment::control(&locked, 3, 0.75)
            .expect("a move")
            .b()
            .clone();
        let free = View::latent(before.clone());
        let ends = |seg: Result<Segment, PathError>| seg.map(edits_of).map_err(bare);
        put(
            format!("lock/snap_back/{pname}"),
            latent(&moved.sim, ends(Segment::snap_back(&moved)), false),
        );
        put(
            format!("lock/snap_back_free/{pname}"),
            latent(&before, ends(Segment::snap_back(&free)), false),
        );
        put(
            format!("lock/unlock/{pname}"),
            latent(&moved.sim, ends(Segment::unlock(&moved)), false),
        );
        put(
            format!("lock/unlock_free/{pname}"),
            latent(&before, ends(Segment::unlock(&free)), false),
        );
        let moved = moved.sim;
        let nearest = Nearest {
            lz: 0.3,
            k: 0.7,
            z: plane.q1,
        };
        put(
            format!("lock/recentre/{pname}"),
            latent::<()>(&before, Ok(Vec::from(recentre(&nearest))), false),
        );
        let narrow = kernel_chart(&moved);
        let f32s = |v: &[f32; 8]| {
            v.iter()
                .map(|x| format!("{:08x}", x.to_bits()))
                .collect::<Vec<_>>()
                .join(",")
        };
        put(
            format!("lock/kernel_chart/{pname}"),
            format!(
                "chart=[{}|{}|{}]",
                f32s(&narrow.z0),
                f32s(&narrow.q1),
                f32s(&narrow.q2)
            ),
        );
        for (i, locked_flag) in [true, false].iter().enumerate() {
            let lock = Lock {
                locked: *locked_flag,
                z_locked: plane.z0,
            };
            put(
                format!("lock/curve_tilt/{pname}/{i}"),
                match curve_tilt::<Defaults>(&lock, &NU_CURVE, 0.3, 0.8, 20.0, &dirs[1].direction) {
                    Ok(q) => format!("q=[{}]", hxs(&q)),
                    Err(e) => err_digest(&e),
                },
            );
        }
        // Save view as chart, a latent view: the oblique chart's words, and the chart they reopen as.
        let saved_from = match variant {
            #[cfg(feature = "controls")]
            Variant::SaveOrthonormal => match oblique(plane.z0, &plane.q1, &plane.q2) {
                Ok(p) => sim(p),
                Err(_) => before.clone(),
            },
            _ => before.clone(),
        };
        let commit = Segment::save_view(&View::latent(saved_from)).expect("a save");
        let Map::Saved(saved) = &commit.b().map else {
            panic!("Save view as chart gives no saved chart");
        };
        let words = saved.to_words();
        let reopened = match Saved::from_words(&words) {
            Some(Saved::Oblique(c)) => latent_ic(&Plane {
                z0: c.z0,
                q1: c.q1,
                q2: c.q2,
            }),
            other => panic!("a saved latent view reads back as {other:?}"),
        };
        put(
            format!("save/latent/{pname}"),
            format!(
                "words={:016x} ic={reopened:016x} view={:016x}",
                fnv(&words
                    .iter()
                    .flat_map(|w| w.to_bits().to_be_bytes())
                    .collect::<Vec<_>>()),
                latent_ic(&plane)
            ),
        );
        // The chart switch's end view onto the latent chart: the cut's `B`.
        let switch =
            Segment::chart_switch(&View::latent(sim(zero())), View::latent(before.clone()));
        let shown = match variant {
            #[cfg(feature = "controls")]
            Variant::SwitchShowsA => switch.a(),
            _ => switch.b(),
        };
        put(
            format!("chart_switch/latent/{pname}"),
            latent_digest(&shown.sim, &[], pname == "axis"),
        );
    }
    // The lock's refusals and edges on a nonlinear chart: the (L_z, E) parabola's edge, one float past it, a
    // non-finite pixel, a chart point past f64's range, and a state the ladder refuses under strict tolerances.
    let centred = variant == Variant::Centred;
    let reg = Tolerances::register(128.0 * f64::EPSILON);
    let strict = Tolerances {
        sanity: -1.0,
        ..reg
    };
    let edge = |u: f64| window([u, 0.5], 0.5);
    put(
        "lock/chart/le_edge".into(),
        chart_lock_in(&invariant_le(), &edge(1.0), (0.5, 0.5), &reg, centred),
    );
    put(
        "lock/chart/le_past_edge".into(),
        chart_lock_in(
            &invariant_le(),
            &edge(1.0f64.next_up()),
            (0.5, 0.5),
            &reg,
            centred,
        ),
    );
    put(
        "lock/chart/le_nan".into(),
        chart_lock_in(
            &invariant_le(),
            &Window::SQUARE,
            (f64::NAN, 0.5),
            &reg,
            centred,
        ),
    );
    let wide = Window {
        centre: [f64::MAX, 0.5],
        half: [f64::MAX, 0.5],
    };
    put(
        "lock/chart/le_wide".into(),
        chart_lock_in(&invariant_le(), &wide, (1.0, 0.5), &reg, centred),
    );
    put(
        "lock/chart/mixed_strict".into(),
        chart_lock_in(&mixed(), &Window::SQUARE, (0.3, 0.8), &strict, centred),
    );
    for (i, at) in CHART_LOCKS.iter().enumerate() {
        put(
            format!("lock/chart/lk/{i}"),
            chart_lock(&invariant_lk(), *at, centred),
        );
        put(
            format!("lock/chart/mixed/{i}"),
            chart_lock(&mixed(), *at, centred),
        );
        put(
            format!("lock/chart/le/{i}"),
            chart_lock(&invariant_le(), *at, centred),
        );
    }
    // The chart switch's end views onto nonlinear charts: each chart through its window, as the switch shows it.
    let from = View::latent(sim(zero()));
    let onto = |b: View| Segment::chart_switch(&from, b).b().sim.chart.window;
    put(
        "chart_switch/mixed".into(),
        format!(
            "ic={:016x}",
            chart_ic(&mixed(), &onto(kernel_view(&mixed())))
        ),
    );
    put(
        "chart_switch/le".into(),
        format!(
            "ic={:016x}",
            chart_ic(&invariant_le(), &onto(kernel_view(&invariant_le())))
        ),
    );
    put(
        "chart_switch/sphere".into(),
        format!(
            "ic={:016x}",
            chart_ic(&sphere(), &onto(kernel_view(&sphere())))
        ),
    );
    // Save view as chart, a curved view: the definition's words, and the chart reopened from them untilted.
    let specs = [
        TiltSpec {
            tau1: 0.0,
            tau2: 0.0,
            map: TiltMap::Sheet,
            x: 5,
            y: 5,
        },
        TiltSpec {
            tau1: 25.0,
            tau2: 0.0,
            map: TiltMap::Sheet,
            x: 5,
            y: 5,
        },
        TiltSpec {
            tau1: 25.0,
            tau2: -40.0,
            map: TiltMap::Lean,
            x: 5,
            y: 4,
        },
    ];
    let centre = [0.4, 0.55];
    let span = [1.0, 1.0];
    let untilted = TiltSpec {
        tau1: 0.0,
        tau2: 0.0,
        map: TiltMap::Sheet,
        x: 5,
        y: 5,
    };
    for (i, spec) in specs.iter().enumerate() {
        let def = saved_through_object(&sphere(), *spec, centre, span).expect("a curved parent");
        let words = def.to_words();
        let Some(Saved::Curved(curved)) = Saved::from_words(&words) else {
            panic!("a saved curved view does not read back");
        };
        let opened = curved
            .open::<ShapeSphere<f64>>(&untilted, centre, span)
            .expect("reopened");
        put(
            format!("save/curved/{i}"),
            format!(
                "words={:016x} ic={:016x}",
                fnv(&words
                    .iter()
                    .flat_map(|w| w.to_bits().to_be_bytes())
                    .collect::<Vec<_>>()),
                chart_ic(&opened, &window(centre, 0.25))
            ),
        );
    }
    // Save view as chart on other curved parents: the (L_z, E), (L_z, K) and mixed charts, each into one of its
    // hidden coordinates, reopened untilted; a parent the kernel refuses, or a composition it refuses, is recorded as
    // such.
    for (i, spec) in specs.iter().enumerate() {
        put(
            format!("save/curved_le/{i}"),
            saved_curved(&invariant_le(), *spec, centre, span, &untilted),
        );
        put(
            format!("save/curved_lk/{i}"),
            saved_curved(&invariant_lk(), *spec, centre, span, &untilted),
        );
        put(
            format!("save/curved_mixed/{i}"),
            saved_curved(&mixed(), *spec, centre, span, &untilted),
        );
    }
    for (name, plane) in presets() {
        put(
            format!("presets/{name}"),
            latent_digest(
                &View::latent(sim(match variant {
                    #[cfg(feature = "controls")]
                    Variant::PresetOblique => {
                        oblique(plane.z0, &plane.q1, &plane.q2).unwrap_or(plane)
                    }
                    _ => plane,
                }))
                .sim,
                &[],
                true,
            ),
        );
    }
    #[cfg(feature = "controls")]
    if variant == Variant::DropOne {
        out.remove("pan/axis/2");
    }
    out
}

/// A curved view of `parent` saved under `spec` about `centre`, digested: its words and the chart reopened from them
/// under `new`, or what the kernel refused.
fn saved_curved<C>(
    parent: &C,
    spec: TiltSpec,
    centre: [f64; 2],
    span: [f64; 2],
    new: &TiltSpec,
) -> String
where
    C: kernel::chart::saved::SaveParent
        + kernel::chart::saved::ParentParams
        + kernel::chart::tilt::Tiltable<Param = f64>
        + ChartDecode,
{
    let Some(def) = saved_through_object(parent, spec, centre, span) else {
        return "refused=save".into();
    };
    let words = def.to_words();
    let Some(Saved::Curved(curved)) = Saved::from_words(&words) else {
        return "refused=read".into();
    };
    let digest = fnv(&words
        .iter()
        .flat_map(|w| w.to_bits().to_be_bytes())
        .collect::<Vec<_>>());
    match curved.open::<C>(new, centre, span) {
        Some(opened) => format!(
            "words={digest:016x} ic={:016x}",
            chart_ic(&opened, &window(centre, 0.25))
        ),
        None => format!("words={digest:016x} refused=open"),
    }
}

/// A plane of zeros.
fn zero() -> Plane {
    Plane {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    }
}

/// A view of the registry chart `chart` over the whole square.
fn kernel_view<C: kernel::chart::Chart>(chart: &C) -> View {
    View::kernel(chart, sim(zero())).expect("a registry chart")
}

/// Save view as chart of a curved view of `parent`, through `Segment::save_curved`: the saved chart, or `None` where
/// the kernel refuses the parent.
fn saved_through_object<C>(
    parent: &C,
    spec: TiltSpec,
    centre: [f64; 2],
    span: [f64; 2],
) -> Option<Saved>
where
    C: kernel::chart::saved::SaveParent + kernel::chart::saved::ParentParams,
{
    let view = View::kernel(parent, sim(zero())).expect("a registry chart");
    match Segment::save_curved(&view, parent, spec, centre, span) {
        Ok(seg) => match &seg.b().map {
            Map::Saved(saved) => Some(**saved),
            other => panic!("Save view as chart gives {other:?}"),
        },
        Err(engine::path::PathError::Refused(_)) => None,
        Err(e) => panic!("Save view as chart refused by the path: {e:?}"),
    }
}

/// The stored cases whose names start with one of `prefixes`, against the same cases through `variant`.
fn check(prefixes: &[&str], variant: Variant) {
    let pick = |cases: BTreeMap<String, String>| -> BTreeMap<String, String> {
        cases
            .into_iter()
            .filter(|(name, _)| prefixes.iter().any(|p| name.starts_with(p)))
            .collect()
    };
    let expected = pick(match variant {
        Variant::Centred => with_live_images(stored(), "centred"),
        #[cfg(feature = "controls")]
        Variant::CentredSettingOff => with_live_images(stored(), "centred"),
        _ => with_live_images(expected(), "images"),
    });
    assert!(
        !expected.is_empty(),
        "no stored case starts with {prefixes:?}"
    );
    // The object's cases are computed once and shared by every test; a control computes its own.
    let actual = match variant {
        Variant::Object => object(),
        #[allow(unreachable_patterns)]
        other => record(other),
    };
    compare(&expected, &pick(actual));
}

#[test]
fn path_pan_bitwise() {
    check(&["pan/"], Variant::Object);
}

#[test]
fn path_zoom_bitwise() {
    check(&["zoom/"], Variant::Object);
}

#[test]
fn path_slice_bitwise() {
    check(&["slice/", "oblique/"], Variant::Object);
}

#[test]
fn path_tilt_bitwise() {
    check(&["tilt", "lock/curve_tilt/"], Variant::Object);
}

#[test]
fn path_lock_bitwise() {
    check(&["lock/"], Variant::Object);
}

/// The lock's own cases: the lock set on the affine and the nonlinear charts.
#[test]
fn path_lock_set_bitwise() {
    check(&["lock/set/", "lock/chart/"], Variant::Object);
}

/// The object's cases, computed once and shared by every test.
fn object() -> BTreeMap<String, String> {
    static OBJECT: std::sync::OnceLock<BTreeMap<String, String>> = std::sync::OnceLock::new();
    OBJECT.get_or_init(|| record(Variant::Object)).clone()
}

// ── The lock's re-recorded cases (stage 2: locking does not move the view) ─────────────────────────────────────────
//
// The stage-1 baseline, `digest.json`, is kept as it was recorded. Locking no longer moves the view, so the cases
// that read a lock's edits, or the view a lock leaves, change; they are re-recorded in `lock_stage2.json`, which
// overlays the baseline case by case. **The criterion**, written before the first run: the overlay's cases are exactly
// `RERECORDED`, each differs from its stage-1 digest, and every other case is bit for bit its stage-1 digest (the tests
// above); and with the centre-on-pin setting on at every lock (`Variant::Centred`), every case, the re-recorded ones
// included, is bit for bit its stage-1 digest: the stage-1 lock is the lock and centre on pin in one command.

/// The re-recorded cases' file, relative to the workspace root.
const STAGE2: &str = "fixtures/chart_composition/lock_stage2.json";

/// The cases stage 2 changes, by prefix, and why. Every other case must stay bit for bit.
const RERECORDED: [(&str, &str); 6] = [
    ("lock/set/", "the lock edits the lock group alone, not z₀"),
    ("lock/slider/", "the move starts from the unmoved z₀"),
    (
        "lock/snap_back/",
        "snap-back undoes the slice only: z₀ re-solved for the pin",
    ),
    ("lock/unlock/", "the view unlocked is the unmoved one"),
    (
        "lock/kernel_chart/",
        "the kernel's chart of the unmoved view",
    ),
    (
        "lock/chart/",
        "the nonlinear lock edits the lock group alone, the window not re-centred",
    ),
];

/// The re-recorded cases.
fn stage2() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root().join(STAGE2)).expect("the re-recorded cases");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    serde_json::from_value(value["cases"].clone()).expect("the cases")
}

/// The baseline with the re-recorded cases in place of their stage-1 digests.
fn expected() -> BTreeMap<String, String> {
    let mut cases = stored();
    cases.extend(stage2());
    cases
}

#[test]
fn path_lock_rerecorded_only_where_the_lock_changes() {
    if std::env::var_os(RECORD).is_some() {
        record_lock_stage2();
    }
    check_rerecorded(stage2());
}

/// Set, the test above first re-records the lock's cases: `PRINCIPIA_RECORD_LOCK_STAGE2=1 cargo test -p engine --test
/// path_bitwise path_lock_rerecorded_only_where_the_lock_changes`, by hand, when the lock's behaviour is meant to change.
const RECORD: &str = "PRINCIPIA_RECORD_LOCK_STAGE2";

/// The re-recorded cases `new` are exactly the ones the lock changes, each changed.
fn check_rerecorded(new: BTreeMap<String, String>) {
    let old = stored();
    let named = |name: &str| RERECORDED.iter().any(|(p, _)| name.starts_with(p));
    let wrong: Vec<String> = new
        .iter()
        .filter(|(name, digest)| !named(name) || old.get(*name) == Some(*digest))
        .map(|(name, _)| name.clone())
        .collect();
    assert!(
        wrong.is_empty(),
        "re-recorded but not a case the lock changes, or unchanged: {wrong:?}"
    );
    for (prefix, why) in RERECORDED {
        assert!(
            new.keys().any(|n| n.starts_with(prefix)),
            "no case under {prefix} re-recorded ({why})"
        );
    }
}

#[test]
fn path_lock_centred_is_the_stage1_lock() {
    check(&["lock/"], Variant::Centred);
}

negative_control!(
    path_lock_centred_is_the_stage1_lock,
    "the setting off where the stage-1 lock is checked",
    expected = "not bit for bit with the chart-path baseline",
    check(&["lock/"], Variant::CentredSettingOff)
);

/// The re-recorded cases with a pan's case among them.
#[cfg(feature = "controls")]
fn with_a_pan() -> BTreeMap<String, String> {
    let mut cases = stage2();
    cases.insert(
        "pan/axis/0".into(),
        "edits= key=0000000000000000 ic=0000000000000000".into(),
    );
    cases
}

negative_control!(
    path_lock_rerecorded_only_where_the_lock_changes,
    "a pan's case among the re-recorded ones",
    expected = "re-recorded but not a case the lock changes",
    check_rerecorded(with_a_pan())
);

/// Writes the re-recorded cases: every `lock/` case the object now digests differently from its stage-1 digest (see
/// [`RECORD`]); `path_lock_rerecorded_only_where_the_lock_changes` then checks what it wrote.
fn record_lock_stage2() {
    let old = stored();
    let cases: BTreeMap<String, String> = record(Variant::Object)
        .into_iter()
        .filter(|(name, digest)| name.starts_with("lock/") && old.get(name) != Some(digest))
        .collect();
    let text = serde_json::json!({
        "what": "The chart-path baseline's lock cases re-recorded for stage 2 (TASK-M2-62): locking does not move the view. Each overlays its case in digest.json; path_bitwise.rs's RERECORDED says why each prefix changes.",
        "command": "PRINCIPIA_RECORD_LOCK_STAGE2=1 cargo test -p engine --test path_bitwise path_lock_rerecorded_only_where_the_lock_changes",
        "cases": cases,
    });
    std::fs::write(
        root().join(STAGE2),
        serde_json::to_string_pretty(&text).expect("JSON") + "\n",
    )
    .expect("written");
}

// ── The images under the live escape label (escape-live-label) ─────────────────────────────────────────────────────
//
// The escape label became live by default and the default rule Fewbody, so the 8 × 8 field a latent case images at
// horizon 3 changes where a sample's label at the horizon changes; nothing of the chart path does. Measured before the
// overlay was written: with the old behaviour put back (the closure rule, the stop option on), every case is bit for bit
// its stored digest; with the new default, 14 cases differ, each in its `img=` alone. Those images are re-recorded in
// `escape_live.json`, which replaces the `img=` word of its cases and nothing else. **The criterion**, written before it
// was recorded: the overlay names only cases that carry an image, each image differs from the one it replaces, and
// every other word of every case is still bit for bit the baseline's (the tests above).

/// The re-recorded images' file, relative to the workspace root.
const LIVE_IMAGES: &str = "fixtures/chart_composition/escape_live.json";

/// Set, `path_live_images_only_where_an_image_is` first re-records the images: `PRINCIPIA_RECORD_ESCAPE_LIVE=1 cargo test
/// -p engine --test path_bitwise path_live_images_only_where_an_image_is`, by hand, when the field's physics is meant to
/// change.
const RECORD_LIVE: &str = "PRINCIPIA_RECORD_ESCAPE_LIVE";

/// The re-recorded images of `set`, case name to its `img=` word's value: `images` against the baseline with the lock's
/// stage-2 cases, `centred` against the stage-1 baseline the centre-on-pin lock is checked with.
fn live_images_of(set: &str) -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root().join(LIVE_IMAGES)).expect("the re-recorded images");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    serde_json::from_value(value[set].clone()).expect("the images")
}

/// The re-recorded images against the baseline with the lock's stage-2 cases.
fn live_images() -> BTreeMap<String, String> {
    live_images_of("images")
}

/// The `img=` word of `digest`, if it has one.
fn img_of(digest: &str) -> Option<&str> {
    digest.split(' ').find_map(|w| w.strip_prefix("img="))
}

/// `cases` with each re-recorded image of `set` in place of its `img=` word.
fn with_live_images(cases: BTreeMap<String, String>, set: &str) -> BTreeMap<String, String> {
    let images = live_images_of(set);
    cases
        .into_iter()
        .map(|(name, digest)| match images.get(&name) {
            Some(img) => {
                let words: Vec<String> = digest
                    .split(' ')
                    .map(|w| {
                        if w.starts_with("img=") {
                            format!("img={img}")
                        } else {
                            w.to_owned()
                        }
                    })
                    .collect();
                (name, words.join(" "))
            }
            None => (name, digest),
        })
        .collect()
}

/// The re-recorded images `images` replace images of `base` only, each a changed one.
fn check_live_images(images: BTreeMap<String, String>, base: BTreeMap<String, String>) {
    let wrong: Vec<String> = images
        .iter()
        .filter(
            |(name, img)| match base.get(*name).and_then(|d| img_of(d)) {
                Some(old) => old == img.as_str(),
                None => true,
            },
        )
        .map(|(name, _)| name.clone())
        .collect();
    assert!(
        wrong.is_empty(),
        "re-recorded but not a case with an image, or unchanged: {wrong:?}"
    );
    assert!(!images.is_empty(), "no image re-recorded");
}

#[test]
fn path_live_images_only_where_an_image_is() {
    if std::env::var_os(RECORD_LIVE).is_some() {
        record_live_images();
    }
    check_live_images(live_images(), expected());
    check_live_images(live_images_of("centred"), stored());
}

/// The re-recorded images with a case that has no image among them.
#[cfg(feature = "controls")]
fn with_an_imageless_case() -> BTreeMap<String, String> {
    let mut images = live_images();
    images.insert("chart_switch/le".into(), "0000000000000000".into());
    images
}

negative_control!(
    path_live_images_only_where_an_image_is,
    "a case with no image among the re-recorded ones",
    expected = "re-recorded but not a case with an image",
    check_live_images(with_an_imageless_case(), expected())
);

/// Writes the re-recorded images: every case whose `img=` the object now gives differently from the baseline's (see
/// [`RECORD_LIVE`]).
fn record_live_images() {
    let changed = |base: BTreeMap<String, String>, now: BTreeMap<String, String>| {
        now.into_iter()
            .filter_map(|(name, digest)| {
                let new = img_of(&digest)?.to_owned();
                let old = base.get(&name).and_then(|d| img_of(d))?;
                (old != new).then_some((name, new))
            })
            .collect::<BTreeMap<String, String>>()
    };
    let images = changed(expected(), record(Variant::Object));
    let centred = changed(
        stored(),
        record(Variant::Centred)
            .into_iter()
            .filter(|(name, _)| name.starts_with("lock/"))
            .collect(),
    );
    let text = serde_json::json!({
        "what": "The chart-path baseline's images re-recorded for escape-live-label: the escape label live by default and the default rule Fewbody change the 8 × 8 field at horizon 3 of these cases; every other word of every case is the baseline's.",
        "command": "PRINCIPIA_RECORD_ESCAPE_LIVE=1 cargo test -p engine --test path_bitwise path_live_images_only_where_an_image_is",
        "images": images,
        "centred": centred,
    });
    std::fs::write(
        root().join(LIVE_IMAGES),
        serde_json::to_string_pretty(&text).expect("JSON") + "\n",
    )
    .expect("written");
}

#[test]
fn path_chart_switch_bitwise() {
    check(&["chart_switch/"], Variant::Object);
}

#[test]
fn path_save_view_bitwise() {
    check(&["save/"], Variant::Object);
}

#[test]
fn path_presets_bitwise() {
    check(&["presets/"], Variant::Object);
}

#[test]
fn path_every_case() {
    check(&[""], Variant::Object);
}

/// A pan with one floating-point operation reordered: `z₀ + 2dt·q₂ + 2ds·q₁` in place of `z₀ + 2ds·q₁ + 2dt·q₂`.
#[cfg(feature = "controls")]
fn pan_reordered(
    plane: &Plane,
    ds: f64,
    dt: f64,
) -> Result<engine::contract::set_field::SetField, engine::nav::NavError> {
    let z0 = std::array::from_fn(|i| plane.z0[i] + 2.0 * dt * plane.q2[i] + 2.0 * ds * plane.q1[i]);
    if !(ds.is_finite() && dt.is_finite()) {
        return Err(engine::nav::NavError::NonFinite);
    }
    Ok(engine::contract::set_field::SetField {
        edit: engine::contract::set_field::Edit::Sim(engine::contract::set_field::SimField::Z0(z0)),
        no_history: false,
    })
}

/// The general geodesic in the raw tilt's place: `q` rotated by `τ` in the plane of `q` and the target's part
/// orthogonal to it, `q′ = cos τ·q + sin τ·‖q‖·t̂⊥`, from the committed plane at `current`'s length, the other vector
/// kept as the raw tilt keeps it. The same plane in exact arithmetic for a unit `q`, another formula in floating point.
#[cfg(feature = "controls")]
fn geodesic(
    committed: &Plane,
    current: &Plane,
    which: BasisVector,
    tau_deg: f64,
    target: &engine::contract::sim_config::Latent,
) -> Result<engine::contract::set_field::SetField, engine::nav::NavError> {
    use engine::contract::sim_config::Latent;
    let dot = |a: &Latent, b: &Latent| -> f64 { a.iter().zip(b).map(|(x, y)| x * y).sum() };
    let norm = |a: &Latent| dot(a, a).sqrt();
    let axpy =
        |a: &Latent, k: f64, b: &Latent| -> Latent { std::array::from_fn(|i| a[i] + k * b[i]) };
    let unit = |a: &Latent| -> Result<Latent, engine::nav::NavError> {
        let n = norm(a);
        match n > 0.0 {
            true => Ok(a.map(|x| x / n)),
            false => Err(engine::nav::NavError::ZeroVector),
        }
    };
    if !tau_deg.is_finite() {
        return Err(engine::nav::NavError::NonFinite);
    }
    if !(-90.0..=90.0).contains(&tau_deg) {
        return Err(engine::nav::NavError::TauOutOfRange);
    }
    let (q, kept, length, kept_length) = match which {
        BasisVector::Q1 => (
            committed.q1,
            committed.q2,
            norm(&current.q1),
            norm(&current.q2),
        ),
        BasisVector::Q2 => (
            committed.q2,
            committed.q1,
            norm(&current.q2),
            norm(&current.q1),
        ),
    };
    let q_hat = unit(&q)?;
    let t_perp = unit(&axpy(target, -dot(target, &q_hat), &q_hat))?;
    let (sin, cos) = engine::exact::sin_cos(tau_deg.to_radians());
    let turned: engine::contract::sim_config::Latent =
        std::array::from_fn(|i| length * (cos * q_hat[i] + sin * t_perp[i]));
    let kept_hat = unit(&kept)?;
    let kept: engine::contract::sim_config::Latent =
        std::array::from_fn(|i| kept_length * kept_hat[i]);
    let (q1, q2) = match which {
        BasisVector::Q1 => (turned, kept),
        BasisVector::Q2 => (kept, turned),
    };
    Ok(engine::contract::set_field::SetField {
        edit: engine::contract::set_field::Edit::Sim(
            engine::contract::set_field::SimField::Basis { q1, q2 },
        ),
        no_history: false,
    })
}

/// A zoom's digest with the path triple `(A, B, family, s)` of its half-way frame in its sim key.
#[cfg(feature = "controls")]
fn zoom_with_triple(
    before: &engine::contract::sim_config::SimConfig,
    octaves: f64,
    digest: String,
) -> String {
    let Ok(seg) = Segment::zoom(&View::latent(before.clone()), octaves) else {
        return digest;
    };
    let triple = seg.record(0.5).expect("a record").to_bytes();
    let mut key = engine::contract::canonical::to_string(&seg.b().sim)
        .expect("canonical")
        .into_bytes();
    key.extend(triple);
    let at = digest.find(" key=").expect("a key") + 5;
    format!("{}{:016x}{}", &digest[..at], fnv(&key), &digest[at + 16..])
}

negative_control!(
    path_pan_bitwise,
    "the pan with one floating-point operation reordered",
    expected = "not bit for bit with the chart-path baseline",
    check(&["pan/"], Variant::PanReordered)
);

negative_control!(
    path_tilt_bitwise,
    "the general geodesic in the raw tilt family's place",
    expected = "not bit for bit with the chart-path baseline",
    check(&["tilt/"], Variant::Geodesic)
);

negative_control!(
    path_zoom_bitwise,
    "a path triple in a zoom's sim key",
    expected = "not bit for bit with the chart-path baseline",
    check(&["zoom/"], Variant::ZoomTriple)
);

/// The slice family without its projection off the plane: `z₀ += t·d`.
#[cfg(feature = "controls")]
fn slice_unprojected(
    plane: &Plane,
    d: &engine::contract::sim_config::Latent,
    t: f64,
) -> Result<engine::contract::set_field::SetField, engine::nav::NavError> {
    if !t.is_finite() || !d.iter().all(|x| x.is_finite()) {
        return Err(engine::nav::NavError::NonFinite);
    }
    if d.iter().all(|x| *x == 0.0) {
        return Err(engine::nav::NavError::ZeroVector);
    }
    let z0 = std::array::from_fn(|i| plane.z0[i] + t * d[i]);
    Ok(engine::contract::set_field::SetField {
        edit: engine::contract::set_field::Edit::Sim(engine::contract::set_field::SimField::Z0(z0)),
        no_history: false,
    })
}

/// The lock's pixel with one floating-point operation reordered, `z₀ + b·q₂ + a·q₁` (`a = 2s − 1`, `b = 2t − 1`), in
/// place of the kernel's `Φ`, `z₀ + a·q₁ + b·q₂`: the lock group alone, as the lock sends it.
#[cfg(feature = "controls")]
fn lock_by_pan(
    plane: &Plane,
    s: f64,
    t: f64,
) -> Result<[engine::contract::set_field::SetField; 1], engine::lock::LockError> {
    use engine::contract::set_field::{Edit, SetField, SimField};
    if !(s.is_finite() && t.is_finite()) {
        return Err(engine::lock::LockError::NonFinite);
    }
    let (a, b) = (s + s - 1.0, t + t - 1.0);
    let z: engine::contract::sim_config::Latent =
        std::array::from_fn(|i| plane.z0[i] + b * plane.q2[i] + a * plane.q1[i]);
    let edit = |field| SetField {
        edit: Edit::Sim(field),
        no_history: false,
    };
    Ok([edit(SimField::Lock(engine::contract::sim_config::Lock {
        locked: true,
        z_locked: z,
    }))])
}

negative_control!(
    path_slice_bitwise,
    "the slice family without its projection off the plane",
    expected = "not bit for bit with the chart-path baseline",
    check(&["slice/"], Variant::SliceUnprojected)
);

negative_control!(
    path_lock_bitwise,
    "the lock's pixel with one floating-point operation reordered",
    expected = "not bit for bit with the chart-path baseline",
    check(&["lock/set/"], Variant::LockByPan)
);

negative_control!(
    path_lock_set_bitwise,
    "a lock that re-centres the view",
    expected = "not bit for bit with the chart-path baseline",
    check(&["lock/set/"], Variant::LockRecentres)
);

negative_control!(
    path_chart_switch_bitwise,
    "a chart switch showing its A at its end",
    expected = "not bit for bit with the chart-path baseline",
    check(&["chart_switch/latent/"], Variant::SwitchShowsA)
);

negative_control!(
    path_save_view_bitwise,
    "Save view as chart re-orthonormalising the view",
    expected = "not bit for bit with the chart-path baseline",
    check(&["save/latent/"], Variant::SaveOrthonormal)
);

negative_control!(
    path_presets_bitwise,
    "a preset reopened through the oblique plane",
    expected = "not bit for bit with the chart-path baseline",
    check(&["presets/"], Variant::PresetOblique)
);

negative_control!(
    path_every_case,
    "one case not produced",
    expected = "not bit for bit with the chart-path baseline",
    check(&[""], Variant::DropOne)
);
