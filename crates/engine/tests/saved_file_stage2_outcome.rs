//! The saved-file conversion's outcome-class arm, on integrated runs: `saved_file_stage2_outcome`.
//!
//! Every stage-1 record of `saved_file_stage2_conversion`'s fixtures (a free view, a lock on the affine chart off the
//! centre, a lock on the mixed chart, an old lock with no pin, a free morph frame whose ends are locked views), rebuilt
//! here through the public API (that test's file is another task's), is converted on load (`FrameRecord::read_converted`),
//! and at every pixel of the 8² grid and at the pin of each end the old view's state is set against the converted
//! one's, each decoded at f64, the state rounded once to f32, then marched by the default occupant, Heggie × RK4, to the
//! default horizon `T = 50` (`engine::trajectory::Integrated`), the dispatch-time label first. (Not the field's record
//! bit for bit: the field narrows the plane to f32 before it decodes, and the mixed chart's views never reach the
//! field; the march is the field's.) The
//! class is the field's `(state, detail)`, the escaping body and the colliding pair included.
//!
//! Pass, written before the run: no pixel outside the excluded set ends in another class. The excluded set is the
//! stated landmarks': a pixel whose old or new configuration is within `δ = ε/ε_phys = 1/64` of its extent from a binary
//! collision, where the decode's conditioning is what `ε_phys` does not cover (`path_canonical_conversion`'s `δ`). A
//! pixel whose two records are the same f32 bits ends in one class by the march's purity
//! (`trajectory_interface_integrator_reads_order_free`) and is counted, not re-marched; every other pixel is marched
//! both ways. Reported per record: the pixels, the excluded (and of them, those whose records differ), the same
//! records and the pairs marched.
//!
//! Control: a conversion that moves each locked view (its centre to the pixel `(¾, ¼)`) before taking its lock's record
//! flips a class.

mod path_shim;

use engine::contract::sim_config::{Latent, Mapping, Plane, SimConfig, Window};
use engine::lookup::Tolerances;
use engine::path::anchor;
use engine::path::blend::{Blend, LatentLeaves, Leaves, Pixel};
use engine::path::{BlendPath, FrameRecord, Map, PathError, Segment, View};
use engine::trajectory::{same_bits, Integrated};
use kernel::chart::mixed::{Alpha, ZMu1};
use kernel::chart::Mixed;
use kernel::decode::{Decoded, Degenerate, Ic};
use kernel::generated::links::Defaults;
use path_shim::sim_of;
use validation::negative_control;
use validation::roundtrip::phys;

// ── The fixtures, as `path_lock.rs` builds them ────────────────────────────────────────────────────────────────────

fn plane_a() -> Plane {
    Plane {
        z0: [0.3, -0.0, 0.1, -0.2, 0.05, 0.0, 0.4, -0.25],
        q1: [0.7, 0.2, -0.0, 0.1, 0.0, 0.0, 0.05, 0.0],
        q2: [0.037, 0.333, 0.111, -0.074, 0.0, 0.037, 0.0, -0.0],
    }
}

fn plane_b() -> Plane {
    Plane {
        z0: [-0.6, 0.45, -0.3, 0.25, 0.4, -0.15, -0.8, 0.6],
        q1: [0.1, -0.9, 0.2, 0.0, -0.1, 0.05, 0.3, 0.0],
        q2: [0.8, 0.1, 0.0, 0.2, 0.15, 0.0, -0.1, 0.4],
    }
}

const FROZEN: Latent = [0.4, -0.3, 0.2, -0.1, 0.3, 0.15, 0.0, -0.4];

fn mixed(z0: Latent) -> Mixed<ZMu1, Alpha, f64> {
    Mixed::new(z0, [-1.0, 1.0], [0.25, 1.3])
}

fn swapped(z0: Latent) -> Mixed<Alpha, ZMu1, f64> {
    Mixed::new(z0, [0.25, 1.3], [-1.0, 1.0])
}

fn mixed_view(window: Window) -> View {
    let mut sim: SimConfig = sim_of(&Plane {
        z0: FROZEN,
        ..plane_a()
    });
    sim.chart.window = window;
    View::kernel(&mixed(FROZEN), sim).expect("the mixed chart is a registry chart")
}

/// The mixed chart's leaves, the chart rebuilt at each view's own hidden controls.
struct MixedLeaves;

impl Leaves for MixedLeaves {
    fn decode(&self, view: &View, [s, t]: [f64; 2]) -> Result<Decoded<f64>, PathError> {
        if view.map != Map::Kernel(engine::path::KernelMap::of(&mixed(FROZEN))?) {
            return Err(PathError::WrongMap);
        }
        let [u, v] = view.sim.chart.window.at(s, t);
        match view.sim.chart.mapping[0] {
            Mapping::Linear => decode_on(view, swapped(view.sim.plane.z0), u, v),
            Mapping::Warped => decode_on(view, mixed(view.sim.plane.z0), u, v),
        }
    }
}

fn decode_on<C>(view: &View, chart: C, u: f64, v: f64) -> Result<Decoded<f64>, PathError>
where
    C: kernel::chart::tilt::Tiltable<Param = f64>,
{
    match view.tilt.is_some_and(|t| !t.spec().untilted()) {
        true => Ok(kernel::chart::Chart::decode::<Defaults, f64>(
            &view.tilted(chart)?,
            u,
            v,
        )),
        false => Ok(kernel::chart::Chart::decode::<Defaults, f64>(&chart, u, v)),
    }
}

fn leaves() -> (LatentLeaves<Defaults>, MixedLeaves) {
    (LatentLeaves::new(), MixedLeaves)
}

fn home() -> Window {
    Window {
        centre: [0.1, 0.7],
        half: [0.6, 0.3],
    }
}

fn locked_latent(plane: &Plane, (s, t): (f64, f64)) -> View {
    let view = View::latent(sim_of(plane));
    let locked = Segment::lock(&view, s, t).expect("a lock").b().clone();
    anchor::record(&locked, &leaves()).expect("a recorded anchor")
}

fn locked_mixed(window: Window, (s, t): (f64, f64)) -> View {
    let view = mixed_view(window);
    let tol = Tolerances::register(128.0 * f64::EPSILON);
    let (seg, _) =
        Segment::lock_on_chart::<Defaults, _, _>(&view, &mixed(FROZEN), (s, t), phys, &tol)
            .expect("a lock on the mixed chart");
    anchor::record(seg.b(), &leaves()).expect("a recorded anchor")
}

fn stage1_records() -> Vec<(&'static str, FrameRecord)> {
    let rec = |a: View, b: View| FrameRecord {
        a,
        b,
        family: engine::path::Family::Cut {
            label: engine::path::CutLabel::Lock,
        },
        s_bits: 0,
        align: None,
        retime_hash: None,
        anchor_words: None,
    };
    let strip = |v: &View| View {
        anchor: None,
        ..v.clone()
    };
    let free = View::latent(sim_of(&plane_a()));
    let affine = strip(&locked_latent(&plane_a(), (0.3, 0.65)));
    let nonlinear = strip(&locked_mixed(home(), (0.35, 0.6)));
    let mut old = View::latent(sim_of(&plane_a()));
    old.sim.lock.locked = true;
    old.sim.lock.z_locked = engine::lock::point(&plane_a(), 0.3, 0.65);
    let morph = View::blend(
        Blend::new(
            affine.clone(),
            View::latent(sim_of(&plane_b())),
            BlendPath::BlockLine,
            0.4,
        )
        .expect("a blend"),
    );
    vec![
        ("free", rec(free.clone(), free)),
        ("affine lock", rec(View::latent(sim_of(&plane_a())), affine)),
        ("nonlinear lock", rec(mixed_view(home()), nonlinear)),
        ("an old lock, no pin", rec(old.clone(), old)),
        ("a morph frame of a locked view", rec(morph.clone(), morph)),
    ]
}

/// The 8² grid and the pin of a view.
fn probes(v: &View) -> Vec<[f64; 2]> {
    let mut p: Vec<[f64; 2]> = (0..64)
        .map(|k| {
            [
                (k % 8) as f64 / 8.0 + 1.0 / 16.0,
                (k / 8) as f64 / 8.0 + 1.0 / 16.0,
            ]
        })
        .collect();
    if let Some((at, _)) = anchor::pin_address(v) {
        p.push(at);
    }
    p
}

// ── The arm ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// The smallest distance between two bodies over the configuration's extent.
fn collision_ratio(ic: &Ic<f64>) -> f64 {
    let d = |i: usize, j: usize| (ic.r[i][0] - ic.r[j][0]).hypot(ic.r[i][1] - ic.r[j][1]);
    let extent =
        ic.r.iter()
            .map(|r| r[0].hypot(r[1]))
            .fold(f64::MIN_POSITIVE, f64::max);
    d(0, 1).min(d(0, 2)).min(d(1, 2)) / extent
}

/// `δ = ε/ε_phys`, the module's note.
fn delta() -> f64 {
    f64::EPSILON / (ledger::constants::EPS_PHYS.number() * f64::EPSILON)
}

/// What the arm read of one record.
#[derive(Default, Debug)]
struct Classes {
    pixels: usize,
    excluded: usize,
    /// The excluded pixels whose two records are not the same bits: excluded and actually moved.
    excluded_moved: usize,
    same_record: usize,
    marched: usize,
}

/// A pixel's state and its decode's tag; panics on a blank.
fn state(pixel: Pixel) -> (Ic<f64>, Option<Degenerate>) {
    match pixel {
        Pixel::State { decoded, .. } => (decoded.ic, decoded.degenerate),
        Pixel::Blank(b) => panic!("a blank pixel: {b:?}"),
    }
}

/// Every stage-1 record converted by `convert`, compared by class with the old at the probes.
fn check_classes(convert: &dyn Fn(&FrameRecord) -> FrameRecord) {
    let provider = Integrated::new(50.0).expect("the default horizon");
    let mut total = Classes::default();
    for (name, old) in stage1_records() {
        let new = convert(&old);
        let mut c = Classes::default();
        for (o, n) in [(&old.a, &new.a), (&old.b, &new.b)] {
            for p in probes(n) {
                c.pixels += 1;
                let (a, ta) = state(anchor::state_at(o, &leaves(), p).expect("old"));
                let (b, tb) = state(anchor::state_at(n, &leaves(), p).expect("new"));
                if collision_ratio(&a) <= delta() || collision_ratio(&b) <= delta() {
                    c.excluded += 1;
                    let moved = !engine::trajectory::same_bits(
                        &Integrated::held(&a),
                        &Integrated::held(&b),
                    );
                    c.excluded_moved += usize::from(moved || ta != tb);
                    continue;
                }
                let (ha, hb) = (Integrated::held(&a), Integrated::held(&b));
                if same_bits(&ha, &hb) && ta == tb {
                    c.same_record += 1;
                    continue;
                }
                c.marched += 1;
                let class = |h: &Ic<f64>, tag| provider.outcome(h, tag).expect("a held start");
                assert_eq!(
                    class(&ha, ta),
                    class(&hb, tb),
                    "{name}: the converted view's pixel {p:?} ends in another class"
                );
            }
        }
        println!("saved_file_stage2_outcome: {name}: {c:?}");
        total.pixels += c.pixels;
        total.excluded += c.excluded;
        total.excluded_moved += c.excluded_moved;
        total.same_record += c.same_record;
        total.marched += c.marched;
    }
    println!("saved_file_stage2_outcome: every record: {total:?}");
    assert!(total.pixels > total.excluded, "the arm read no pixel");
}

/// The conversion on load: the record's bytes read back converted.
fn converted(old: &FrameRecord) -> FrameRecord {
    FrameRecord::read_converted(&old.to_bytes(), &leaves()).expect("an old record converts")
}

#[test]
fn saved_file_stage2_outcome() {
    check_classes(&converted);
}

negative_control!(
    saved_file_stage2_outcome,
    "a conversion that moves a locked view before recording it flips a class",
    expected = "ends in another class",
    check_classes(&|old: &FrameRecord| {
        let mut new = converted(old);
        for v in [&mut new.a, &mut new.b] {
            if v.anchor.is_some() && v.sim.lock.locked {
                let moved = View {
                    anchor: None,
                    sim: SimConfig {
                        plane: Plane {
                            z0: engine::lock::point(&v.sim.plane, 0.75, 0.25),
                            ..v.sim.plane.clone()
                        },
                        ..v.sim.clone()
                    },
                    ..v.clone()
                };
                *v = anchor::record(&moved, &leaves()).unwrap_or_else(|_| v.clone());
            }
        }
        new
    })
);
