//! The harness's post-escape march and pitfalls §2.4's checks (`validation::escape_checks`; TASK-M3-33):
//! - `escape_harness_same_discretisation`: a `config stability` pixel that fires escape, marched by production and by
//!   the harness: the harness's state at its firing is production's stored state, bit for bit, with production's
//!   `detail`, and the harness marches on past it to its own horizon. Control: production under another sync quantum
//!   (`Δ_sync = 1/16`), a different discretisation, which the check refuses.
//! - `escape_harness_payload_untouched`: the production buffer's bytes are the same after the harness march over it as
//!   before (REQ-VAL-054's harness arm). Control: a harness that writes its own state into the buffer.
//! - `escape_ground_truth_reads_distances`: check 2's ground truth on synthetic marches: linear and parabolic growth
//!   escape whatever the energy says, an orbit that turns back and growth slower than the parabola are bound whatever
//!   the energy says, a collision is censored. Control: a ground truth read from the energy sign.
//! - `escape_rebind_offsets`: the five offsets are prin-rs's `k·13/32`, and a re-binding at an offset is read at the
//!   first sync boundary at or after it. Control: a reader that takes the boundary before.
//!
//! **Domain** (pitfalls §10): f64, Heggie × RK4 as the field backend marches it, one `config stability` pixel and
//! synthetic marches; the population measurements are the gates' (`cargo xtask gate escape-checks`, `escape-rebind`).

use kernel::decode::Ic;
use kernel::payload::{sd_state, tm_t_end_step, STATE_ESCAPE};
use kernel::regularisation::az::AzReal;
use kernel::regularisation::heggie::{HgConfig, HgParams};

use engine::precision::{MarchKey64, SimState64};

use validation::escape_checks::check2::{self, Truth};
use validation::escape_checks::march::{self, Firing, Mark, Point, PostEscape, Stop};
use validation::escape_checks::rebind::{self, Reading};
use validation::escape_checks::{config_stability, GroundTruthOccupant};
use validation::negative_control;

/// The `config stability` pixel (16 × 16, row-major from the bottom left) that fires escape at `t = 14.5` under its
/// horizon 50, and the population's key for it.
fn firing_pixel() -> (Ic<f64>, MarchKey64) {
    let pop = config_stability(16, 50.0);
    (pop.ics[104], pop.keys[104])
}

/// The harness's horizon in these tests: 60, past the slice's 50 and the pixel's firing.
const HARNESS_STEPS: u32 = 60_000;

/// Heggie's field-backend values at another sync quantum, `Δ_sync = 1/16`.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
struct Sixteenth;

impl HgConfig for Sixteenth {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            delta_sync: R::at_width(0.0625, 0.0625),
            ..validation::ground_truth::FieldHeggie::params::<R>()
        }
    }
}

/// Heggie × RK4 at `Δ_sync = 1/16`.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
type OtherGrid = kernel::regularisation::Composed<
    kernel::occupant::rk4::Rk4,
    kernel::regularisation::heggie::Heggie<Sixteenth>,
>;

/// `production`, the pixel's stored state, against the harness march: the same discretisation, a firing, and a march
/// past it.
fn check_same(production: SimState64) {
    let (ic, key) = firing_pixel();
    let horizon = f64::from(key.loop_values.horizon_steps) * key.loop_values.dt_macro;
    let run = march::march_past::<GroundTruthOccupant>(&ic, &key, HARNESS_STEPS);
    if let Err(e) = march::same_discretisation(&production, &run, horizon) {
        panic!("the harness march is not production's: {e}");
    }
    let f = run.firing.expect("the pixel fires");
    println!(
        "pixel 104: fired at {} (production stopped at step {}), the harness on to {} ({:?})",
        f.t,
        tm_t_end_step(production.times),
        run.t_end,
        run.stop
    );
    assert_eq!(
        sd_state(production.packed_a),
        STATE_ESCAPE,
        "production fired"
    );
    assert_eq!(run.stop, Stop::Horizon, "the harness reached its horizon");
    assert!(
        run.t_end + 0.125 >= f64::from(HARNESS_STEPS) * key.loop_values.dt_macro,
        "the harness stopped at {} short of its horizon",
        run.t_end
    );
    assert!(
        run.points.iter().filter(|p| p.t > f.t).count() > 100,
        "the harness did not march past the firing"
    );
}

#[test]
fn escape_harness_same_discretisation() {
    let (ic, key) = firing_pixel();
    check_same(march::production::<GroundTruthOccupant>(&[ic], &[key])[0]);
}

negative_control!(
    escape_harness_same_discretisation,
    "production under another sync quantum, Δ_sync = 1/16: a different discretisation",
    expected = "the harness march is not production's",
    {
        let (ic, key) = firing_pixel();
        check_same(march::production::<OtherGrid>(&[ic], &[key])[0])
    }
);

/// A harness over a buffer: the population's ICs and keys, the buffer, the harness horizons.
type HarnessFn = fn(&mut [SimState64], &[Ic<f64>], &[MarchKey64], &[u32]);

/// The harness as it is: it reads the buffer through a shared borrow.
fn real(buffer: &mut [SimState64], ics: &[Ic<f64>], keys: &[MarchKey64], steps: &[u32]) {
    march::harness::<GroundTruthOccupant>(buffer, ics, keys, steps).expect("the harness");
}

/// A harness that writes its own end state into the buffer.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn leaky(buffer: &mut [SimState64], ics: &[Ic<f64>], keys: &[MarchKey64], steps: &[u32]) {
    let runs =
        march::harness::<GroundTruthOccupant>(buffer, ics, keys, steps).expect("the harness");
    for (s, r) in buffer.iter_mut().zip(&runs) {
        if let Some(f) = r.firing {
            s.r = f.r.map(|x| x.map(|c| c + 1.0));
        }
    }
}

/// The buffer's bytes across `harness`, over the firing pixel and three of its neighbours.
fn check_untouched(harness: HarnessFn) {
    let pop = config_stability(16, 50.0);
    let pick = [104, 105, 88, 120];
    let ics: Vec<Ic<f64>> = pick.iter().map(|&i| pop.ics[i]).collect();
    let keys: Vec<MarchKey64> = pick.iter().map(|&i| pop.keys[i]).collect();
    let mut buffer = march::production::<GroundTruthOccupant>(&ics, &keys);
    let before = march::buffer_bytes(&buffer);
    assert_eq!(
        before.len(),
        4 * std::mem::size_of::<SimState64>(),
        "the bytes"
    );
    harness(&mut buffer, &ics, &keys, &[HARNESS_STEPS; 4]);
    assert!(
        march::buffer_bytes(&buffer) == before,
        "the payload buffer changed across the harness march"
    );
}

#[test]
fn escape_harness_payload_untouched() {
    check_untouched(real);
}

negative_control!(
    escape_harness_payload_untouched,
    "a harness that writes its own state into the buffer",
    expected = "the payload buffer changed",
    check_untouched(leaky)
);

/// A synthetic march to `t_h` on the sync grid: body 2's distance `d(t)` and `E_rel` `e`, bodies 0 and 1 at 0.5.
fn synthetic(t_h: f64, d: impl Fn(f64) -> f64, e: f64, stop: Stop) -> PostEscape {
    let points: Vec<Point> = (0..=(t_h * 8.0) as u32)
        .map(|k| {
            let t = f64::from(k) / 8.0;
            Point {
                t,
                d: [0.5, 0.5, d(t)],
                e_rel: [-1.0, -1.0, e],
            }
        })
        .collect();
    PostEscape {
        firing: None,
        t_end: points.last().map_or(0.0, |p| p.t),
        points,
        marks: Vec::<Mark>::new(),
        stop,
    }
}

/// A ground truth.
type TruthFn = fn(&PostEscape) -> Truth;

/// A ground truth read from the energy sign: the farthest body escapes where its `E_rel > 0` at the end.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn energy_sign(run: &PostEscape) -> Truth {
    let p = run.points.last().expect("a point");
    let k = (0..3)
        .max_by(|&i, &j| p.d[i].total_cmp(&p.d[j]))
        .expect("three");
    if p.e_rel[k] > 0.0 {
        Truth::Escapes(k as u32)
    } else {
        Truth::Bound
    }
}

/// `truth` on the synthetic marches (the file's note).
fn check_truth(truth: TruthFn) {
    let t_h = 60.0;
    let rho = check2::rho();
    assert!((rho - check2::RHO).abs() < 1e-15, "RHO is 2^(2/3) − 1");
    let cases: [(&str, PostEscape, Truth); 6] = [
        (
            "linear growth, bound energy",
            synthetic(t_h, |t| 1.0 + t, -0.5, Stop::Horizon),
            Truth::Escapes(2),
        ),
        (
            "parabolic from t = 0, bound energy",
            synthetic(t_h, |t| 1.0 + t.powf(2.0 / 3.0), -0.5, Stop::Horizon),
            Truth::Escapes(2),
        ),
        (
            "parabolic from just before T_h/2, bound energy",
            synthetic(
                t_h,
                |t| 1.0 + (t - 29.0).max(0.0).powf(2.0 / 3.0),
                -0.5,
                Stop::Horizon,
            ),
            Truth::Escapes(2),
        ),
        (
            "an orbit that turns back, unbound energy",
            synthetic(
                t_h,
                |t| 6.0 + 4.0 * (std::f64::consts::PI * t / t_h).sin(),
                0.5,
                Stop::Horizon,
            ),
            Truth::Bound,
        ),
        (
            "growth as √(t − T_h/2), slower than the parabola, unbound energy",
            synthetic(
                t_h,
                |t| 1.0 + (t - 30.0).max(0.0).sqrt(),
                0.5,
                Stop::Horizon,
            ),
            Truth::Bound,
        ),
        (
            "a collision",
            synthetic(t_h, |t| 1.0 + t, 0.5, Stop::Collision),
            Truth::Censored,
        ),
    ];
    for (name, run, want) in cases {
        let got = truth(&run);
        println!("{name}: {got:?}, growth {:?}", check2::growth(&run));
        assert_eq!(
            got, want,
            "{name}: the ground truth read the energy, or misread the growth"
        );
    }
}

#[test]
fn escape_ground_truth_reads_distances() {
    check_truth(check2::truth);
}

negative_control!(
    escape_ground_truth_reads_distances,
    "a ground truth read from the energy sign, the term check 2 must not share with the criterion",
    expected = "the ground truth read the energy",
    check_truth(energy_sign)
);

/// The sampling point before `t`, the reader the control takes.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn before(points: &[Point], t: f64) -> Option<&Point> {
    points.iter().rev().find(|p| p.t < t)
}

/// The offsets, and a re-binding at `+2` read by `pick`.
fn check_offsets(pick: rebind::Pick) {
    assert_eq!(
        rebind::offsets_burrau(),
        [0.40625, 0.8125, 1.21875, 1.625, 3.25],
        "the offsets are k·13/32 Burrau units, k = 1, 2, 3, 4, 8"
    );
    // Fired at 10; the escaper's E_rel dips to bound only on the boundary at 10.875, the first at or after 10.8125.
    let mut run = synthetic(20.0, |t| 1.0 + t, 0.3, Stop::Horizon);
    for p in &mut run.points {
        if p.t == 10.875 {
            p.e_rel[2] = -0.01;
        }
    }
    run.firing = Some(Firing {
        index: 80,
        t: 10.0,
        escaper: 2,
        detail: 2,
        r: [[0.0; 2]; 3],
        p: [[0.0; 2]; 3],
    });
    let r = rebind::readings_with(0, &run, 20.0, rebind::offsets(1.0), pick).expect("a firing");
    println!("{:?}", r.readings);
    assert!(
        matches!(r.readings[1], Reading::Rebound(_)),
        "the re-binding at +2 was not read"
    );
    let clean = rebind::Rebind {
        readings: [Reading::Unbound(0.3); 5],
        ..r
    };
    assert!(
        rebind::judge(&[clean; 20]).is_ok(),
        "twenty clean firings failed the judge"
    );
    let mut one = vec![clean; 19];
    one.push(r);
    assert!(
        rebind::judge(&one).is_err(),
        "a re-bound firing among twenty passed the judge"
    );
}

#[test]
fn escape_rebind_offsets() {
    check_offsets(rebind::at_or_after);
}

negative_control!(
    escape_rebind_offsets,
    "a reader that takes the sync boundary before the offset",
    expected = "the re-binding at +2 was not read",
    check_offsets(before)
);
