//! The first slice (TASK-M2-86; REQ-DEC-057; R-926 items 5 and 6, R-935's RQ-766 (a)): the latent chart, the shape
//! sphere's exponential map and the `(L_z, E)` chart through the CPU's f64 evaluation, the per-quad forward table, the
//! narrowed records and the CPU reference integrator, against today's CPU f32 decode (`sample_with::<…, f32>`), on
//! REQ-DEC-057's grids (`engine::first_slice::GRIDS`). The GPU decode stage's arm is `first_slice_gpu`.
//!
//! **The predicates, PASS required:**
//! - `first_slice_records_are_write_ic` (1): the host-written records of each quad equal `write_ic` of the same
//!   narrowed `Ic<f32>`, word for word, the narrowing and the f64 evaluation formed here, apart from the host's.
//! - `first_slice_runs_bitwise` (2): two runs of the slice give the same records, bitwise.
//! - `first_slice_narrowed_correctly_rounded` (3): each narrowed component is the correctly rounded value of its f64
//!   evaluation, within half an f32 ulp of it, checked against its f32 neighbours.
//! - `first_slice_equal_records_integrate_equal` (4): equal records integrate to equal `SimState`s, bitwise.
//!
//! **Measured and reported, never asserted** (`first_slice_measure_*`, one per chart; run with `--no-capture` to read
//! the report): per grid, the narrowed f64 records against today's f32 decode per component in f32 ulps at
//! `max(1, |x|)`; the branch selects that differ between the two; on the `(L_z, E)` chart, the same split by whether the
//! sample's neighbourhood crosses one of the chart's kinks; the share of neighbouring sample pairs whose narrowed records
//! are bitwise identical; and the integrated arm with its control arm, the chaos baseline (R-84). What each asserts is
//! that the measurement is like for like: the selects are read on the very decode the record came from, at each
//! precision. Every differing record is listed with its size in a file under the test's target directory.
//!
//! - `first_slice_feasibility_triple`: the re-plan's O7, the kernel's exact feasibility predicate against a plain
//!   comparison on an f32 triple where the two disagree, each verdict reported.
//!
//! **The integrated arms** (the measurement's design, its criterion unchanged): they run on the default occupant,
//! Heggie × RK4 under Eq. 22 at the CPU field's values (`engine::first_slice::HeggieRk4`), and beside it on KDK, the
//! stand-in the first measurement ran on before the default occupant was built, labelled so in the report. Both at
//! the cap dispatch admits, `T = 200` (`T_HORIZON_MAX`), the longest a march may run to end in another class, on every
//! 16th sample across and up (a 16 × 16 sub-lattice of each quad), in a release build: the report's run, `cargo test
//! --release -p engine --test first_slice -- --nocapture first_slice_measure`. The debug suite leaves them out, within
//! its per-test limits, and proves the arm can fire instead (`first_slice_arm_can_fire`). Each march reads the key's
//! collision threshold, Part 3's default `r_coll = 1e-3` (`first_slice_march_collides`), and the wrapper's cadence
//! labels escape (`first_slice_march_escapes`), so a march may end in collision, escape or bounded at the horizon (a
//! regularised march also `running`, its budget spent, or `sim_failed`). **The horizon and each grid's criterion,
//! written before the run:** `T = 200`; on each grid the arm is read against its control, never alone: the control's
//! flip share is above 0 on at least one grid of each chart, and on a grid where it is 0 the arm is reported as
//! uninformative at that horizon, not as "0 flips". The report also counts the control marches that ended in the
//! narrowed record's very `SimState`, where the one-ulp nudge was lost in the march and could not flip.
//!
//! **Also** (the measurement's design):
//! - predicate (4) runs at `T = 4` on every 16th sample, on the default occupant and on KDK: equal records marching to
//!   equal states is bitwise, at any horizon;
//! - a sample whose point the chart's `validate` does not find `Valid` (on the `(L_z, E)` chart's origin grids, the
//!   rows below its floor `t = 0`, which validation projects to the floor) is not a state the system integrates: it is
//!   counted in a bucket of its own and left out of every measure.
//!
//! **Domain** (pitfalls §10): each decode at its fixed chart and window; each march from its record alone.

#[path = "first_slice/measure.rs"]
mod measure;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use engine::contract::sim_uniforms::dt_macro_for;
use engine::decode_dispatch::decode_only_with;
use engine::first_slice::{
    evaluate, forward_table, march, outcome, outcome_of, records, today, uniforms, FieldValues,
    HeggieRk4, MarchKey, Outcome, Quad, SliceChart,
};
use kernel::chart::invariant::WarpReal;
use kernel::chart::shape_sphere::{inverse_pole, inverse_pq};
use kernel::chart::{InvariantLE, Latent, ShapeSphere};
use kernel::decode::invariant::{
    construct_expansion, construction_short, doubling_input, feasibility_input, infeasible,
    seed_word,
};
use kernel::decode::{descriptor, Decoded, Ic};
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::Advance;
use kernel::generated::links::{Defaults, LinkReal};
use kernel::occupant::kdk::Kdk;
use kernel::payload::{
    PayloadReal, SimStateBase, STATE_BOUNDED, STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE,
    STATE_RUNNING, STATE_SIM_FAILED,
};
use kernel::regularisation::heggie::{HgConfig, HgParams};
use kernel::stage::decode::{ic_words, read_ic, tag, write_ic};
use measure::{compare, identical_pairs, in_domain, Diff, COMPONENTS};
use validation::first_slice::{Grid, Preset, GRIDS, SIDE};
use validation::negative_control;

/// Predicate (4)'s horizon (applied per R-369; the module's note).
const T: f64 = 4.0;

/// Predicate (4)'s sub-lattice: every `STRIDE`th sample across and up.
const STRIDE: usize = 16;

/// The integrated arms' horizon: the cap dispatch admits, `T_HORIZON_MAX` (R-610), the longest a march may run to end
/// in another class (the module's note).
fn t_arms() -> f64 {
    ledger::constants::T_HORIZON_MAX.number()
}

/// The integrated arms' sub-lattice: every 16th sample across and up, a 16 × 16 sub-lattice of each quad.
const ARM_STRIDE: usize = 16;

/// Whether this build runs the integrated arms: a release build, the report's run (the module's note); the debug suite
/// runs the comparison, the selects and the pairs, and proves the arm can fire (`first_slice_arm_can_fire`).
const ARMS: bool = !cfg!(debug_assertions);

/// The seed in force, named in the report (the task's deliverable).
const SEED: &str = "seed in force: R-934's expansion w = r/√I on every pixel, no seed branch; R-82's rule, the \
                    largest-norm seed decided on z_α's sign, is the named legacy option, not run here";

/// The quad of `grid` on the chart `C`, REQ-DEC-057's 256 × 256.
fn quad<C: Preset>(grid: Grid) -> Quad<C> {
    C::quad(grid, [SIDE, SIDE])
}

/// `f(grid)` on each of REQ-DEC-057's grids, side by side, in [`GRIDS`]' order.
fn each_grid<T: Send>(f: impl Fn(Grid) -> T + Sync) -> Vec<T> {
    let f = &f;
    std::thread::scope(|s| {
        let handles: Vec<_> = GRIDS.iter().map(|&g| s.spawn(move || f(g))).collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a grid's thread panicked"))
            .collect()
    })
}

// ── The harness's addressing ───────────────────────────────────────────────────────────────────────────────────────

/// A quad's samples and their points: sample `(x, y)` at the window's `((x + ½)/w, (y + ½)/h)`, row by row, past the
/// quad too; its sim key the physics kernel's, with the window and the latent chart's plane, widened; and the loop
/// values of a horizon, dispatch's refusal for one it refuses.
fn check_addressing(at: fn(&Quad<InvariantLE<f32>>, i64, i64) -> [f64; 2]) {
    let window = engine::contract::sim_config::Window {
        centre: [0.25, 0.5],
        half: [0.5, 2.0],
    };
    let q = Quad {
        chart: <InvariantLE<f32> as Preset>::preset(),
        window,
        size: [4, 2],
    };
    assert_eq!((q.len(), q.is_empty()), (8, false), "the samples");
    let empty = Quad { size: [0, 3], ..q };
    assert!(empty.is_empty(), "a quad with no column has samples");
    assert_eq!(at(&q, 0, 0), [-0.125, -0.5], "sample (0, 0)'s point");
    assert_eq!(at(&q, 3, 1), [0.625, 1.5], "sample (3, 1)'s point");
    assert_eq!(at(&q, -1, 2), [-0.375, 3.5], "a neighbour past the quad");
    assert_eq!(q.uv(5), q.at(1, 1), "sample 5 is (1, 1)");
    assert_eq!(q.uv(6), q.at(2, 1), "sample 6 is (2, 1)");
    let key = q.sim_key();
    assert_eq!(key.chart.window, window, "the sim key's window");
    assert_eq!(
        key.kernel_variant,
        engine::contract::sim_config::KernelVariant::Physics
    );
    let l = <Latent<f32> as Preset>::quad(GRIDS[1], [2, 2]);
    let plane = l.sim_key().plane;
    assert_eq!(plane.z0, l.chart.z0.map(f64::from), "the latent plane's z0");
    assert_eq!(plane.q1, l.chart.q1.map(f64::from), "the latent plane's q1");
    assert_eq!(plane.q2, l.chart.q2.map(f64::from), "the latent plane's q2");
    assert_eq!(q.sim_key().plane.q1, [0.0; 8], "a curved chart's plane");
    let u = uniforms(T).expect("T is a valid horizon");
    assert_eq!(u.loop_values.horizon_steps, 4000, "T = 4's steps");
    assert_eq!(u.loop_values.dt_macro, 1e-3, "T = 4's dt_macro");
    // Past 65.535 the horizon's own dt_macro rises above the floor (R-132), so it is the horizon's, not a default's.
    let long = uniforms(100.0).expect("T = 100 is a valid horizon");
    assert_eq!(
        (long.loop_values.dt_macro, long.loop_values.horizon_steps),
        (dt_macro_for(100.0) as f32, 65535),
        "T = 100's dt_macro and steps"
    );
    assert!(
        uniforms(0.1).is_err(),
        "a horizon below R-610's floor was accepted"
    );
}

#[test]
fn first_slice_quad_addressing() {
    check_addressing(Quad::at);
}

negative_control!(
    first_slice_quad_addressing,
    "a point at the sample's corner, not its centre, must fail",
    expected = "sample (0, 0)'s point",
    check_addressing(|q, x, y| q.window.at(x as f64 / 4.0, y as f64 / 2.0))
);

// ── Predicate 1: the host's records are `write_ic` of the narrowed `Ic<f32>` ──────────────────────────────────────────

/// `quad`'s records as written here: `decode_only_with` at each sample, each component rounded to f32 by `as`, and
/// `write_ic`.
fn written<C: SliceChart>(quad: &Quad<C>) -> Vec<u32> {
    let key = quad.sim_key();
    let mut out = vec![0u32; quad.len() * ic_words()];
    for i in 0..quad.len() {
        let ic = decode_only_with::<C, Defaults>(&quad.chart, quad.uv(i), &key, None)
            .expect("the physics kernel decodes")
            .ic;
        let narrowed = Ic {
            m: ic.m.map(|x| x as f32),
            r: ic.r.map(|v| v.map(|x| x as f32)),
            p: ic.p.map(|v| v.map(|x| x as f32)),
        };
        write_ic(&mut out, i, &narrowed);
    }
    out
}

/// `host`, the quad's host-written records, equals `want` word for word.
fn check_written(host: &[u32], want: &[u32]) {
    assert_eq!(
        host.len(),
        want.len(),
        "the host wrote another number of words"
    );
    let n = ic_words();
    for (i, (a, b)) in host.chunks(n).zip(want.chunks(n)).enumerate() {
        assert_eq!(
            a, b,
            "sample {i}: the host's record is not write_ic of its narrowed Ic<f32>"
        );
    }
}

fn check_charts_written<C: Preset>(host: fn(&Quad<C>) -> Vec<u32>) {
    for grid in GRIDS {
        let q = quad::<C>(grid);
        check_written(&host(&q), &written(&q));
    }
}

/// The host's records: the slice's evaluation.
fn host<C: SliceChart>(q: &Quad<C>) -> Vec<u32> {
    evaluate(q).expect("the physics kernel decodes")
}

#[test]
fn first_slice_records_are_write_ic() {
    std::thread::scope(|s| {
        s.spawn(|| check_charts_written::<Latent<f32>>(host));
        s.spawn(|| check_charts_written::<ShapeSphere<f32>>(host));
        s.spawn(|| check_charts_written::<InvariantLE<f32>>(host));
    });
}

negative_control!(
    first_slice_records_are_write_ic,
    "a host whose records swap two words of one sample must fail",
    expected = "is not write_ic of its narrowed",
    check_charts_written::<Latent<f32>>(|q| {
        let mut r = host(q);
        r.swap(3, 9);
        r
    })
);

// ── Predicate 2: two runs, the same records ──────────────────────────────────────────────────────────────────────────

/// Two runs, `first` and `second`, give the same records, bitwise.
fn check_runs(first: &[u32], second: &[u32]) {
    assert!(
        first == second,
        "two runs of the slice gave different records"
    );
}

fn check_charts_runs<C: Preset>(second: fn(&Quad<C>) -> Vec<u32>) {
    for grid in GRIDS {
        let q = quad::<C>(grid);
        check_runs(&host(&q), &second(&q));
    }
}

#[test]
fn first_slice_runs_bitwise() {
    std::thread::scope(|s| {
        s.spawn(|| check_charts_runs::<Latent<f32>>(host));
        s.spawn(|| check_charts_runs::<ShapeSphere<f32>>(host));
        s.spawn(|| check_charts_runs::<InvariantLE<f32>>(host));
    });
}

negative_control!(
    first_slice_runs_bitwise,
    "a second run one bit off in one word must fail",
    expected = "two runs of the slice gave different records",
    check_charts_runs::<ShapeSphere<f32>>(|q| {
        let mut r = host(q);
        r[7] ^= 1;
        r
    })
);

// ── Predicate 3: each narrowed component is the correctly rounded f64 value ──────────────────────────────────────────

/// Whether `x32` is the f32 nearest `x64` (ties either way), so within half an f32 ulp of it: no f32 neighbour of
/// `x32` is nearer, by exact f64 differences; a non-finite `x64` narrows to its own kind, and an infinite `x32` from a
/// finite `x64` only past `f32::MAX` and half its ulp.
fn correctly_rounded(x64: f64, x32: f32) -> bool {
    if x64.is_nan() {
        return x32.is_nan();
    }
    if x32.is_infinite() {
        let limit = f64::from(f32::MAX) + f64::powi(2.0, 103);
        return x64.is_sign_negative() == x32.is_sign_negative() && x64.abs() >= limit;
    }
    if x64.is_infinite() {
        return false;
    }
    let d = (f64::from(x32) - x64).abs();
    let (below, above) = (x32.next_down(), x32.next_up());
    let near = |n: f32| !n.is_finite() || d <= (f64::from(n) - x64).abs();
    near(below) && near(above)
}

/// Every word of `records` is the correctly rounded value of its node's component in `table`.
fn check_rounded(table: &[Ic<f64>], records: &[u32]) {
    for (i, node) in table.iter().enumerate() {
        let wide = [
            node.m[0],
            node.m[1],
            node.m[2],
            node.r[0][0],
            node.r[0][1],
            node.r[1][0],
            node.r[1][1],
            node.r[2][0],
            node.r[2][1],
            node.p[0][0],
            node.p[0][1],
            node.p[1][0],
            node.p[1][1],
            node.p[2][0],
            node.p[2][1],
        ];
        let words = &records[i * ic_words()..][..ic_words()];
        for (k, (&x64, &w)) in wide.iter().zip(words).enumerate() {
            let x32 = f32::from_bits(w);
            assert!(
                correctly_rounded(x64, x32),
                "sample {i}, {}: {x32:e} is not the correctly rounded value of {x64:e}",
                COMPONENTS[k]
            );
        }
    }
}

fn check_charts_rounded<C: Preset>(narrowing: fn(&[Ic<f64>]) -> Vec<u32>) {
    for grid in GRIDS {
        let table = forward_table(&quad::<C>(grid)).expect("the physics kernel decodes");
        check_rounded(&table, &narrowing(&table));
    }
}

#[test]
fn first_slice_narrowed_correctly_rounded() {
    std::thread::scope(|s| {
        s.spawn(|| check_charts_rounded::<Latent<f32>>(records));
        s.spawn(|| check_charts_rounded::<ShapeSphere<f32>>(records));
        s.spawn(|| check_charts_rounded::<InvariantLE<f32>>(records));
    });
}

/// `check_rounded`'s own cases: the nearest float, the halfway tie, the overflow edge and the non-finite values pass;
/// the next float out fails.
#[test]
fn first_slice_rounding_check() {
    let x = 1.0 + f64::powi(2.0, -30);
    assert!(correctly_rounded(x, 1.0));
    assert!(!correctly_rounded(x, 1.0f32.next_up()));
    let tie = 1.0 + f64::powi(2.0, -24);
    assert!(correctly_rounded(tie, 1.0) && correctly_rounded(tie, 1.0f32.next_up()));
    assert!(correctly_rounded(f64::NAN, f32::NAN) && !correctly_rounded(f64::NAN, 0.0));
    assert!(!correctly_rounded(f64::INFINITY, f32::MAX));
    let edge = f64::from(f32::MAX) + f64::powi(2.0, 103);
    assert!(correctly_rounded(edge, f32::INFINITY) && correctly_rounded(-edge, f32::NEG_INFINITY));
    assert!(!correctly_rounded(edge.next_down(), f32::INFINITY));
    assert!(correctly_rounded(edge.next_down(), f32::MAX));
    assert!(!correctly_rounded(-edge, f32::INFINITY));
}

negative_control!(
    first_slice_narrowed_correctly_rounded,
    "a narrowing one float off in one component must fail",
    expected = "is not the correctly rounded value of",
    check_charts_rounded::<InvariantLE<f32>>(|t| {
        let mut r = records(t);
        r[4] = (f32::from_bits(r[4]).next_up()).to_bits();
        r
    })
);

negative_control!(
    first_slice_rounding_check,
    "a check that passes the next float out must fail",
    expected = "assertion failed",
    assert!(correctly_rounded(
        1.0 + f64::powi(2.0, -30),
        1.0f32.next_up()
    ))
);

// ── Predicate 4: equal records, equal `SimState`s ─────────────────────────────────────────────────────────────────────

/// `s`'s fields as words, bitwise: `r`, `p`, the f32 members, the packed words and the counters.
fn state_bits(s: &SimStateBase) -> Vec<u32> {
    let mut w: Vec<u32> =
        s.r.iter()
            .chain(&s.p)
            .flatten()
            .map(|x| x.to_bits())
            .collect();
    for x in [s.S, s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0, s.closure_min] {
        w.push(x.to_bits());
    }
    w.extend([s.packed_a, s.packed_b, s.times, s.total_substeps]);
    w.extend([u32::from(s.closure_step), u32::from(s._reserved)]);
    w
}

/// Records `a` and `b` of two runs, sample by sample on the sub-lattice, where equal, integrate to bitwise equal
/// `SimState`s under `u`; `nudge` changes `b`'s record before its march (the control's).
fn check_equal_march<A: Advance>(
    a: &[u32],
    b: &[u32],
    size: [usize; 2],
    nudge: fn(Ic<f32>) -> Ic<f32>,
) {
    let u = uniforms(T).expect("T is a valid horizon");
    let mut marched = 0;
    for y in (0..size[1]).step_by(STRIDE) {
        for x in (0..size[0]).step_by(STRIDE) {
            let i = y * size[0] + x;
            let (ra, rb) = (read_ic(a, i), read_ic(b, i));
            if thin_bits(&ra) != thin_bits(&rb) {
                continue;
            }
            marched += 1;
            assert_eq!(
                state_bits(&march::<A>(&ra, &u)),
                state_bits(&march::<A>(&nudge(rb), &u)),
                "sample {i}: equal records integrated to different SimStates"
            );
            assert!(
                outcome::<A>(&ra, &u).t_end_step > 0,
                "sample {i}: the march took no macro-step"
            );
        }
    }
    assert!(marched > 0, "no equal records were integrated");
}

fn check_charts_march<C: Preset, A: Advance>(nudge: fn(Ic<f32>) -> Ic<f32>) {
    for grid in [GRIDS[0], GRIDS[2]] {
        let q = quad::<C>(grid);
        check_equal_march::<A>(&host(&q), &host(&q), q.size, nudge);
    }
}

/// Predicate (4) on the default occupant, Heggie × RK4, and on KDK, the stand-in the first measurement ran on.
#[test]
fn first_slice_equal_records_integrate_equal() {
    std::thread::scope(|s| {
        s.spawn(|| check_charts_march::<Latent<f32>, HeggieRk4>(|ic| ic));
        s.spawn(|| check_charts_march::<ShapeSphere<f32>, HeggieRk4>(|ic| ic));
        s.spawn(|| check_charts_march::<InvariantLE<f32>, HeggieRk4>(|ic| ic));
        s.spawn(|| check_charts_march::<Latent<f32>, Kdk>(|ic| ic));
        s.spawn(|| check_charts_march::<ShapeSphere<f32>, Kdk>(|ic| ic));
        s.spawn(|| check_charts_march::<InvariantLE<f32>, Kdk>(|ic| ic));
    });
}

negative_control!(
    first_slice_equal_records_integrate_equal,
    "one record's march on the default occupant moved by a float in one component must fail",
    expected = "equal records integrated to different SimStates",
    check_charts_march::<Latent<f32>, HeggieRk4>(|ic| nudged(&ic, 3))
);

// ── The measurement ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A float the selects are read at: the decode's own bounds and `From<f32>`, the charts' parameter type.
trait Q: LinkReal + PayloadReal + WarpReal + From<f32> + Into<f64> {}
impl Q for f32 {}
impl Q for f64 {}

/// The branch selects compared, in the order [`Selects`] holds them: the `DEGENERATE` tag; the descriptor's poles
/// (R-436); `rho_angle`'s three (R-437), the last its upper/lower select; the shape sphere inverse's hemisphere (its
/// upper/lower select) and its `p = q = 0` pole (R-437); the `(L_z, E)` chart's feasibility and its seed, the seed the
/// decode in force takes: R-934's `w = r/√I`, word 5, where `I > 0`, else none, word 4 (its only select; R-82's rule,
/// decided on `z_α`'s sign, is the named legacy option and is not read). Today's decode has no sheet select (R-937's
/// second sheet is not built).
const SELECTS: [&str; 11] = [
    "tag",
    "ratio_pole",
    "collision_pole",
    "virial_pole",
    "cross_zero",
    "dot_zero",
    "dot_negative",
    "sphere_upper",
    "sphere_pole",
    "feasibility",
    "seed",
];

/// One sample's selects, each its outcome word; 0 where a chart has no such select.
type Selects = [u32; 11];

/// A chart of the slice, read for its own selects.
trait Reads: Preset {
    /// Whether the chart has kinks, so its measures are split by them.
    const KINKS: bool = false;

    /// The chart's own selects at `(u, v)`, at `R`, on the decode `d` there, written into `s`.
    fn reads<R: Q>(&self, _u: R, _v: R, _d: &Decoded<R>, _s: &mut Selects) {}

    /// Whether sample `(x, y)`'s neighbourhood on `q`, itself and its four neighbours, crosses one of the chart's
    /// kinks; none on a chart that has none.
    fn kinked(_q: &Quad<Self>, _x: usize, _y: usize) -> bool {
        false
    }
}

impl Reads for Latent<f32> {}

impl Reads for ShapeSphere<f32> {
    fn reads<R: Q>(&self, u: R, v: R, _d: &Decoded<R>, s: &mut Selects) {
        let n = self.point::<R>(u, v);
        s[7] = u32::from(n[0] >= R::zero());
        let [p, q] = inverse_pq(n, <R as From<f32>>::from(self.inertia));
        s[8] = u32::from(inverse_pole(p, q));
    }
}

impl Reads for InvariantLE<f32> {
    const KINKS: bool = true;

    fn reads<R: Q>(&self, u: R, v: R, d: &Decoded<R>, s: &mut Selects) {
        let t = self.0.targets::<R>(u, v);
        let f = feasibility_input(R::one(), t.k, t.lz);
        s[9] = u32::from(construction_short(&f, doubling_input(t.k)));
        s[10] = seed_word(construct_expansion(&d.masses, &d.config, t.lz, t.k).seed);
    }

    /// The feasibility edge `s ∈ {0, 1}`, the floor `t = 0` and the line `z_α = 0` (the frozen control, the same at
    /// every sample): crossed where the neighbourhood's range of `u`, of `v` or `z_α` holds it. `z_α = 0` is not a seam
    /// of the seed in force: it is R-82's legacy seed's seam and the `α` link's widest step (RQ-820 (closed by R-1022)).
    fn kinked(q: &Quad<Self>, x: usize, y: usize) -> bool {
        let (x, y) = (x as i64, y as i64);
        let pts = [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)].map(|(dx, dy)| q.at(x + dx, y + dy));
        let range = |k: usize| {
            pts.iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                    (lo.min(p[k]), hi.max(p[k]))
                })
        };
        let holds = |(lo, hi): (f64, f64), c: f64| lo <= c && c <= hi;
        let (u, v) = (range(0), range(1));
        holds(u, 0.0) || holds(u, 1.0) || holds(v, 0.0) || q.chart.0.z_cfg[0] == 0.0
    }
}

/// The configuration's `ρ̃`, `λ̃` and `ρ̃·λ̃` at one sample, widened to f64: what `dot_negative` reads.
type Dot = [f64; 5];

/// `chart`'s selects at `(u, v)` at `R`, read on the decode `decode_at` gives there, whose `ic` is returned with them,
/// and the configuration's [`Dot`].
fn selects<C: Reads, R: Q>(chart: &C, u: R, v: R) -> (Selects, Ic<R>, Dot) {
    let d = chart.decode_at::<Defaults, R>(u, v);
    let p = d.descriptor;
    let (cross, dot) = descriptor::cross_dot(d.config.rho_t, d.config.lambda_t);
    let mut s: Selects = [0; 11];
    s[0] = tag(d.degenerate);
    s[1] = u32::from(descriptor::ratio_pole(p.lambda_mag));
    s[2] = u32::from(descriptor::collision_pole(p.r_min_pair_0));
    s[3] = u32::from(descriptor::virial_pole(p.V_0));
    s[4] = u32::from(descriptor::cross_zero(cross));
    s[5] = u32::from(descriptor::dot_zero(dot));
    s[6] = u32::from(descriptor::dot_negative(dot));
    chart.reads(u, v, &d, &mut s);
    let [rx, ry] = d.config.rho_t;
    let [lx, ly] = d.config.lambda_t;
    (
        s,
        d.ic,
        [rx.into(), ry.into(), lx.into(), ly.into(), dot.into()],
    )
}

/// The `dot_negative` samples where the two decodes select differently: how many, how many of them have the f32
/// decode's error in `ρ̃·λ̃`, `|dot₃₂ − dot₆₄|`, above the true value's size, `|dot₆₄|` (so the f32 sign is the sign of
/// its error), and the first sample's `ρ̃`, `λ̃` and dot at each precision.
#[derive(Default)]
struct DotFlips {
    n: usize,
    error_exceeds: usize,
    largest_true: f64,
    least_error: f64,
    first: Option<(usize, Dot, Dot)>,
}

impl DotFlips {
    fn add(&mut self, i: usize, wide: Dot, thin: Dot) {
        let (t, e) = (wide[4].abs(), (thin[4] - wide[4]).abs());
        if self.n == 0 {
            self.least_error = e;
        }
        self.n += 1;
        self.error_exceeds += usize::from(e > t);
        self.largest_true = self.largest_true.max(t);
        self.least_error = self.least_error.min(e);
        self.first.get_or_insert((i, wide, thin));
    }
}

/// What one grid's measurement holds.
struct Measured {
    grid: Grid,
    /// The samples outside the chart's domain ([`in_domain`]), left out of every measure.
    outside: usize,
    /// The narrowed f64 records against today's f32 decode's, every sample.
    diff: Diff,
    /// The same over the samples whose neighbourhood crosses a kink, and over the rest.
    kinked: Option<(Diff, Diff)>,
    /// Per select, the samples where today's decode and the f64 evaluation select differently, and the first few.
    selects: [usize; 11],
    examples: Vec<String>,
    /// The `dot_negative` samples that differ, measured.
    dots: DotFlips,
    /// Neighbouring pairs with bitwise identical narrowed records, and with identical records of today's decode.
    pairs: [(usize, usize); 2],
    /// The integrated arms on each of [`OCCUPANTS`], in its order.
    arms: [Arms; 2],
    /// Where the differing records are listed.
    listing: PathBuf,
}

/// `ic`'s words, `write_ic`'s.
fn thin_bits(ic: &Ic<f32>) -> [u32; 15] {
    let mut w = [0u32; 15];
    write_ic(&mut w, 0, ic);
    w
}

/// `ic`'s components' bits, in `write_ic`'s order.
fn wide_bits(ic: &Ic<f64>) -> Vec<u64> {
    ic.m.iter()
        .chain(ic.r.iter().flatten())
        .chain(ic.p.iter().flatten())
        .map(|x| x.to_bits())
        .collect()
}

/// `ic` with its word `k` (in `write_ic`'s order) one f32 up.
fn nudged(ic: &Ic<f32>, k: usize) -> Ic<f32> {
    let mut w = [0u32; 15];
    write_ic(&mut w, 0, ic);
    w[k] = f32::from_bits(w[k]).next_up().to_bits();
    read_ic(&w, 0)
}

/// The march outcome of record `ic` on the occupant `A`, or `decode_failed` for a sample whose decode is `DEGENERATE`,
/// which no march starts from.
fn class<A: Advance>(ic: &Ic<f32>, degenerate: bool, u: &MarchKey) -> Outcome {
    run::<A>(ic, degenerate, u).0
}

/// The total energy `Σ|p|²/2m − Σ m_i·m_j/r_ij` (`G = 1`) of masses `m` at `r` with momenta `p`, in f64.
fn energy(m: [f32; 3], r: [[f32; 2]; 3], p: [[f32; 2]; 3]) -> f64 {
    let w = |x: f32| f64::from(x);
    let mut e = 0.0;
    for i in 0..3 {
        e += (w(p[i][0]).powi(2) + w(p[i][1]).powi(2)) / (2.0 * w(m[i]));
        for j in i + 1..3 {
            let d = (w(r[i][0]) - w(r[j][0])).hypot(w(r[i][1]) - w(r[j][1]));
            e -= w(m[i]) * w(m[j]) / d;
        }
    }
    e
}

/// [`class`] with the `SimState`'s words ([`state_bits`]) the march ended in and its energy error at its end,
/// `|E − E₀|/max(|E₀|, 10⁻³)`, `E₀` the record's; none and 0 where no march starts.
fn run<A: Advance>(ic: &Ic<f32>, degenerate: bool, u: &MarchKey) -> (Outcome, Vec<u32>, f64) {
    if degenerate {
        let failed = Outcome {
            state: STATE_DECODE_FAILED,
            detail: 0,
            t_end_step: 0,
        };
        return (failed, Vec::new(), 0.0);
    }
    let s = march::<A>(ic, u);
    let e0 = energy(ic.m, ic.r, ic.p);
    let drift = (energy(ic.m, s.r, s.p) - e0).abs() / e0.abs().max(1e-3);
    (outcome_of(&s), state_bits(&s), drift)
}

/// The occupants the integrated arms run on, in [`Measured::arms`]' order: the default occupant, then the stand-in
/// the first measurement ran on before the default was built.
const OCCUPANTS: [&str; 2] = [
    "Heggie x RK4 (Eq. 22), the default occupant",
    "KDK, the stand-in for the default occupant (the first measurement's)",
];

/// One class's marches: how many, the least and largest `t_end_step`, and each march's energy error at its end.
type ClassTally = (usize, u32, u32, Vec<f64>);

/// One occupant's integrated arms on one grid.
#[derive(Default)]
struct Arms {
    /// (records compared, outcome-class flips, `t_end_step` flips): today's against the narrowed, then the control.
    arms: [(usize, usize, usize); 2],
    /// The control's marches that ended in the very `SimState` of the narrowed record's, bitwise: where the nudge was
    /// lost in the march, so the control could not flip.
    control_same: usize,
    /// The outcome classes `(state, detail)` of the narrowed records' marches, with the least and largest
    /// `t_end_step` of each, and the median and largest energy error at the march's end ([`run`]).
    classes: BTreeMap<(u32, u32), ClassTally>,
}

/// The integrated arms on the occupant `A` over every `stride`th sample across and up of a quad of `size` (none at
/// stride 0): each sample `inside` its chart's domain marched from its narrowed record, and where today's record
/// differs, from today's and from the control's, the narrowed record one ulp up in one component.
fn arms<A: Advance>(
    narrowed: &[u32],
    old: &[kernel::stage::decode::DecodeOut<f32>],
    inside: &[bool],
    failed: &[[bool; 2]],
    size: [usize; 2],
    stride: usize,
) -> Arms {
    let u = uniforms(t_arms()).expect("the arms' horizon is valid");
    let mut out = Arms::default();
    if stride == 0 {
        return out;
    }
    for y in (0..size[1]).step_by(stride) {
        for x in (0..size[0]).step_by(stride) {
            let i = y * size[0] + x;
            if !inside[i] {
                continue;
            }
            let new = read_ic(narrowed, i);
            let [degenerate, was] = failed[i];
            let (base, base_bits, drift) = run::<A>(&new, degenerate, &u);
            let entry = out.classes.entry((base.state, base.detail)).or_insert((
                0,
                u32::MAX,
                0,
                Vec::new(),
            ));
            entry.0 += 1;
            entry.1 = entry.1.min(base.t_end_step);
            entry.2 = entry.2.max(base.t_end_step);
            entry.3.push(drift);
            // The control moves the same records the comparison reads (those where the two decodes differ), so both
            // arms' rates are of one population.
            if thin_bits(&new) != thin_bits(&old[i].ic) {
                tally(&mut out.arms[0], base, class::<A>(&old[i].ic, was, &u));
                let (control, control_bits, _) =
                    run::<A>(&nudged(&new, 3 + i % 12), degenerate, &u);
                tally(&mut out.arms[1], base, control);
                out.control_same += usize::from(!degenerate && control_bits == base_bits);
            }
        }
    }
    out
}

/// One arm's tally, (records, outcome-class flips, `t_end_step` flips): `other`'s outcome against `base`'s, the class
/// at payload §2's grain, `(state, detail)`.
fn tally(arm: &mut (usize, usize, usize), base: Outcome, other: Outcome) {
    arm.0 += 1;
    arm.1 += usize::from((other.state, other.detail) != (base.state, base.detail));
    arm.2 += usize::from(other.t_end_step != base.t_end_step);
}

/// One grid's measurement on chart `C` (the module's note), its selects read on `alter` of the quad's chart (the
/// chart itself in the measurement; another in its control), its integrated arms on every `stride`th sample across and
/// up (none at stride 0).
fn measure_grid<C: Reads>(grid: Grid, alter: fn(C) -> C, stride: usize) -> Measured {
    let q = quad::<C>(grid);
    let read = alter(q.chart);
    let table = forward_table(&q).expect("the physics kernel decodes");
    let narrowed = records(&table);
    let old = today(&q);
    assert_eq!(
        old.len(),
        q.len(),
        "the selects were read on another decode than the record's: today's decode has another number of samples"
    );
    let inside: Vec<bool> = (0..q.len()).map(|i| in_domain(&q.chart, q.uv(i))).collect();
    let mut old_records = vec![0u32; q.len() * ic_words()];
    let mut sel = [0usize; 11];
    let mut examples = Vec::new();
    let mut failed = Vec::with_capacity(q.len());
    let mut dots = DotFlips::default();
    for (i, d) in old.iter().enumerate() {
        write_ic(&mut old_records, i, &d.ic);
        let [u, v] = q.uv(i);
        let (wide, wide_ic, wide_dot) = selects::<C, f64>(&read, u, v);
        let (thin, thin_ic, thin_dot) = selects::<C, f32>(&read, u as f32, v as f32);
        assert!(
            wide_bits(&wide_ic) == wide_bits(&table[i]) && thin_bits(&thin_ic) == thin_bits(&d.ic),
            "sample {i}: the selects were read on another decode than the record's"
        );
        failed.push([wide[0] != 0, thin[0] != 0]);
        if !inside[i] {
            continue;
        }
        if wide[6] != thin[6] {
            dots.add(i, wide_dot, thin_dot);
        }
        let mut which = Vec::new();
        for k in 0..SELECTS.len() {
            if wide[k] != thin[k] {
                sel[k] += 1;
                which.push(format!("{} f64 {} f32 {}", SELECTS[k], wide[k], thin[k]));
            }
        }
        if !which.is_empty() && examples.len() < 8 {
            examples.push(format!(
                "sample ({}, {}) at (u, v) = ({u:e}, {v:e}): {}",
                i % q.size[0],
                i / q.size[0],
                which.join("; ")
            ));
        }
    }
    let diff = compare(&narrowed, &old_records, |i| inside[i]);
    let kink = |i: usize| C::kinked(&q, i % q.size[0], i / q.size[0]);
    let kinked = C::KINKS.then(|| {
        (
            compare(&narrowed, &old_records, |i| inside[i] && kink(i)),
            compare(&narrowed, &old_records, |i| inside[i] && !kink(i)),
        )
    });
    let listing = list(C::NAME, grid, &q, &diff);
    let pairs = [
        identical_pairs(&narrowed, q.size, |i| inside[i]),
        identical_pairs(&old_records, q.size, |i| inside[i]),
    ];
    let stride = if ARMS { stride } else { 0 };
    let (default, stand_in) = std::thread::scope(|s| {
        let d = s.spawn(|| arms::<HeggieRk4>(&narrowed, &old, &inside, &failed, q.size, stride));
        let k = s.spawn(|| arms::<Kdk>(&narrowed, &old, &inside, &failed, q.size, stride));
        (
            d.join().expect("the default occupant's arms"),
            k.join().expect("the stand-in's arms"),
        )
    });
    Measured {
        grid,
        outside: inside.iter().filter(|&&a| !a).count(),
        diff,
        kinked,
        selects: sel,
        examples,
        dots,
        pairs,
        arms: [default, stand_in],
        listing,
    }
}

/// Writes every differing record of `diff`, with its size, to a file of its own, and returns its path.
fn list<C: SliceChart>(chart: &str, grid: Grid, q: &Quad<C>, diff: &Diff) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("first_slice");
    std::fs::create_dir_all(&dir).expect("the listing's directory");
    let path = dir.join(format!("{chart}-{:?}-{}.tsv", grid.centre, grid.zoom).to_lowercase());
    let mut text = String::from("x\ty\tu\tv\tlargest_ulps\tcomponent\n");
    for d in &diff.differing {
        let [u, v] = q.uv(d.sample);
        let _ = writeln!(
            text,
            "{}\t{}\t{u:e}\t{v:e}\t{:e}\t{}",
            d.sample % q.size[0],
            d.sample / q.size[0],
            d.ulps,
            COMPONENTS[d.component]
        );
    }
    std::fs::write(&path, text).expect("the listing");
    path
}

/// `a` of `b`, and its share.
fn share((a, b): (usize, usize)) -> String {
    format!("{a} of {b} ({:.4})", a as f64 / b.max(1) as f64)
}

/// The integrated arm's reading on one grid: read against the control's flips, never alone (R-84); where the control
/// registers no flip the arm is uninformative at this horizon, not "0 flips".
fn arm_reading([(n0, c0, s0), (n1, c1, s1)]: [(usize, usize, usize); 2]) -> String {
    if n1 == 0 || c1 + s1 == 0 {
        return "arm reading: the control registers no flip here, so the arm is uninformative at this horizon"
            .to_owned();
    }
    let rate = |a: usize, n: usize| a as f64 / n.max(1) as f64;
    format!(
        "arm reading: today's against the narrowed flips {:.4} of classes and {:.4} of t_end_step, against the \
         control's {:.4} and {:.4} (the chaos baseline){}",
        rate(c0, n0),
        rate(s0, n0),
        rate(c1, n1),
        rate(s1, n1),
        if n0 == 0 { "; no differing record marched" } else { "" }
    )
}

/// A class `(state, detail)` by name (payload §2).
fn class_name((state, detail): (u32, u32)) -> String {
    let name = match state {
        STATE_ESCAPE => "escape",
        STATE_BOUNDED => "bounded",
        STATE_COLLISION => "collision",
        STATE_RUNNING => "running",
        STATE_SIM_FAILED => "sim_failed",
        STATE_DECODE_FAILED => "decode_failed",
        _ => "unknown",
    };
    format!("{name}/{detail}")
}

/// The report of chart `C`'s grids, the integrated arms on every `stride`th sample.
fn report<C: SliceChart>(measured: &[Measured], stride: usize) -> String {
    let mut r = format!(
        "first slice, {}: the narrowed f64 records against today's f32 decode, {SIDE} x {SIDE} a grid; the \
         integrated arms on {} and on {}, T = {}, r_coll = Part 3's default 1e-3, escape labelled by the wrapper's \
         cadence, every {stride}th sample across and up, a {} build\n  {SEED}\n",
        C::NAME,
        OCCUPANTS[0],
        OCCUPANTS[1],
        t_arms(),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    for m in measured {
        let _ = writeln!(r, "  {}:", m.grid.name());
        if m.outside > 0 {
            let _ = writeln!(
                r,
                "    outside the chart's domain (its validate does not find the point Valid), left out of every \
                 measure: {} of {} samples",
                m.outside,
                SIDE * SIDE
            );
        }
        r.push_str(&m.diff.to_string());
        if let Some((kinked, plain)) = &m.kinked {
            let _ = writeln!(r, "   neighbourhood crossing a kink:");
            r.push_str(&kinked.to_string());
            let _ = writeln!(r, "   neighbourhood crossing none:");
            r.push_str(&plain.to_string());
        }
        let differing: Vec<String> = SELECTS
            .iter()
            .zip(m.selects)
            .filter(|(_, n)| *n > 0)
            .map(|(s, n)| format!("{s} {n}"))
            .collect();
        let _ = writeln!(
            r,
            "    branch selects differing: {}",
            if differing.is_empty() {
                "none".to_owned()
            } else {
                differing.join(", ")
            }
        );
        for e in &m.examples {
            let _ = writeln!(r, "      {e}");
        }
        let d = &m.dots;
        if let Some((i, w, t)) = d.first {
            let _ = writeln!(
                r,
                "    dot_negative's {} differing samples, measured: the f32 decode's error in ρ̃·λ̃ exceeds the true \
                 |ρ̃·λ̃| at {} of them; |f64 ρ̃·λ̃| at most {:e}, |f32 − f64| at least {:e}; at sample ({}, {}): f64 ρ̃ \
                 ({:e}, {:e}) λ̃ ({:e}, {:e}) ρ̃·λ̃ {:e}; f32 ρ̃ ({:e}, {:e}) λ̃ ({:e}, {:e}) ρ̃·λ̃ {:e}",
                d.n,
                d.error_exceeds,
                d.largest_true,
                d.least_error,
                i % SIDE,
                i / SIDE,
                w[0],
                w[1],
                w[2],
                w[3],
                w[4],
                t[0],
                t[1],
                t[2],
                t[3],
                t[4]
            );
        }
        let _ = writeln!(
            r,
            "    neighbouring pairs bitwise identical: narrowed f64 {}; today's f32 {}",
            share(m.pairs[0]),
            share(m.pairs[1])
        );
        if !ARMS {
            let _ = writeln!(
                r,
                "    integrated arms: not run in a debug build; the report's release run runs them (the module's note)"
            );
            let _ = writeln!(r, "    differing records listed in {}", m.listing.display());
            continue;
        }
        for (name, a) in OCCUPANTS.iter().zip(&m.arms) {
            let [(n0, c0, s0), (n1, c1, s1)] = a.arms;
            let classes: Vec<String> = a
                .classes
                .iter()
                .map(|(&k, (n, lo, hi, drift))| {
                    let mut d = drift.clone();
                    d.sort_by(f64::total_cmp);
                    format!(
                        "{} {n} (t_end_step {lo}..={hi}; |dE/E0| median {:.1e}, largest {:.1e})",
                        class_name(k),
                        d[d.len() / 2],
                        d[d.len() - 1]
                    )
                })
                .collect();
            let _ = writeln!(r, "    integrated on {name}:");
            let _ = writeln!(
                r,
                "      today's against the narrowed, {n0} differing records: class flips {}, t_end_step flips {}; the \
                 control (one ulp in one component), the same {n1} records: class flips {}, t_end_step flips {}; \
                 control marches ending in the narrowed record's SimState bitwise: {}",
                share((c0, n0)),
                share((s0, n0)),
                share((c1, n1)),
                share((s1, n1)),
                share((a.control_same, n1))
            );
            let _ = writeln!(
                r,
                "      classes of the narrowed records' marches: {}",
                classes.join(", ")
            );
            let _ = writeln!(r, "      {}", arm_reading(a.arms));
        }
        let _ = writeln!(r, "    differing records listed in {}", m.listing.display());
    }
    r
}

fn measure_chart<C: Reads + Send + Sync>(stride: usize) {
    let measured = each_grid(|g| measure_grid::<C>(g, |c| c, stride));
    println!("{}", report::<C>(&measured, stride));
}

#[test]
fn first_slice_measure_latent() {
    measure_chart::<Latent<f32>>(ARM_STRIDE);
}

#[test]
fn first_slice_measure_shape_sphere() {
    measure_chart::<ShapeSphere<f32>>(ARM_STRIDE);
}

#[test]
fn first_slice_measure_invariant_lz_e() {
    measure_chart::<InvariantLE<f32>>(ARM_STRIDE);
}

/// The integrated arms' sub-lattice of the full-grid run: `FIRST_SLICE_ARM_STRIDE` if set, else 1, every sample.
fn full_stride() -> usize {
    std::env::var("FIRST_SLICE_ARM_STRIDE").ok().map_or(1, |s| {
        s.parse().expect("FIRST_SLICE_ARM_STRIDE is a stride")
    })
}

/// The measurement with the integrated arms on every sample of every grid (or every `FIRST_SLICE_ARM_STRIDE`th), all
/// three charts. At stride 1 it is some 65 536 marches per occupant per grid, hours of CPU: `cargo test --release -p
/// engine --test first_slice -- --ignored --nocapture first_slice_measure_full_grid`.
#[test]
#[ignore = "full grid: run on the Mac"]
fn first_slice_measure_full_grid() {
    let stride = full_stride();
    measure_chart::<Latent<f32>>(stride);
    measure_chart::<ShapeSphere<f32>>(stride);
    measure_chart::<InvariantLE<f32>>(stride);
}

negative_control!(
    first_slice_measure_full_grid,
    "selects read on another chart's decode must fail",
    expected = "the selects were read on another decode",
    mismatched()
);

/// The measurement with its selects read on the latent preset's chart with `z₀`'s first component one f32 up: not
/// the records' decode.
#[cfg(feature = "controls")]
fn mismatched() {
    measure_grid::<Latent<f32>>(
        GRIDS[0],
        |mut c| {
            c.z0[0] = c.z0[0].next_up();
            c
        },
        0,
    );
}

negative_control!(
    first_slice_measure_latent,
    "selects read on another chart's decode must fail",
    expected = "the selects were read on another decode",
    mismatched()
);

negative_control!(
    first_slice_measure_shape_sphere,
    "selects read on another chart's decode must fail",
    expected = "the selects were read on another decode",
    mismatched()
);

negative_control!(
    first_slice_measure_invariant_lz_e,
    "selects read on another chart's decode must fail",
    expected = "the selects were read on another decode",
    mismatched()
);

// ── The integrated arm can fire ───────────────────────────────────────────────────────────────────────────────────────

/// The arm registers a flip where the outcomes differ, and reads against its control: one record of the latent home
/// quad marched on the default occupant at `T = 4`, bounded to the horizon (its 4000 macro-steps, each completed,
/// whatever the occupant's own grid), against the same sample's decode failed, which no march starts from
/// (`decode_failed`, at step 0), so a class flip and a `t_end_step` flip; two escapes of different bodies, a class flip
/// at payload §2's grain, `(state, detail)`; and the reading uninformative where the control
/// registers no flip, else both arms' rates.
fn check_arm(tally: fn(&mut (usize, usize, usize), Outcome, Outcome)) {
    let u = uniforms(T).expect("T is a valid horizon");
    let q = <Latent<f32> as Preset>::quad(GRIDS[0], [1, 1]);
    let record = read_ic(&evaluate(&q).expect("the physics kernel decodes"), 0);
    let marched = class::<HeggieRk4>(&record, false, &u);
    let failed = class::<HeggieRk4>(&record, true, &u);
    let mut arm = (0, 0, 0);
    tally(&mut arm, marched, marched);
    tally(&mut arm, marched, failed);
    let escaped = |detail| Outcome {
        state: 0,
        detail,
        t_end_step: 7,
    };
    tally(&mut arm, escaped(1), escaped(2));
    assert_eq!(
        (marched.t_end_step, arm),
        (4000, (3, 2, 1)),
        "the arm cannot register a flip"
    );
    assert!(
        arm_reading([(3, 0, 0), (5, 0, 0)]).contains("uninformative")
            && arm_reading([(4, 1, 0), (5, 0, 2)]).contains(
                "flips 0.2500 of classes and 0.0000 of t_end_step, against \
             the control's 0.0000 and 0.4000"
            ),
        "the arm is not read against its control"
    );
}

#[test]
fn first_slice_arm_can_fire() {
    check_arm(tally);
}

negative_control!(
    first_slice_arm_can_fire,
    "an arm that counts records and no flip must fail",
    expected = "the arm cannot register a flip",
    check_arm(|arm, _, _| arm.0 += 1)
);

// ── The march reads the key's collision threshold ────────────────────────────────────────────────────────────────

/// The march fires the collision branch at the key's `r_coll`, Part 3's default `1e-3`: the latent home record with
/// body 1 moved to `5·10⁻⁴` of body 0, inside `r_coll`, ends `collision` at step 0 (integrator_contract Part 5), where at
/// `r_coll = 0` it would march on.
fn check_collides<A: Advance>(key: fn(MarchKey) -> MarchKey) {
    let u = uniforms(T).expect("T is a valid horizon");
    assert_eq!(
        u.radius,
        CollisionRadius::from_r_coll(1e-3),
        "the key's collision threshold"
    );
    let q = <Latent<f32> as Preset>::quad(GRIDS[0], [1, 1]);
    let mut record = read_ic(&evaluate(&q).expect("the physics kernel decodes"), 0);
    record.r[1] = [record.r[0][0] + 5e-4, record.r[0][1]];
    let got = outcome::<A>(&record, &key(u));
    assert_eq!(
        (got.state, got.t_end_step),
        (STATE_COLLISION, 0),
        "a record inside r_coll did not end in collision at step 0"
    );
}

/// On the default occupant and on the stand-in.
#[test]
fn first_slice_march_collides() {
    check_collides::<HeggieRk4>(|u| u);
    check_collides::<Kdk>(|u| u);
}

negative_control!(
    first_slice_march_collides,
    "a march on the default occupant at r_coll = 0 must fail",
    expected = "a record inside r_coll did not end in collision at step 0",
    check_collides::<HeggieRk4>(|u| MarchKey {
        radius: CollisionRadius::none(),
        ..u
    })
);

// ── The march labels escape ──────────────────────────────────────────────────────────────────────────────────────────

/// Three equal bodies on an equilateral triangle of circumradius 1 about the origin, each moving straight out from it
/// at `speed`: momenta summing to zero, and at a speed well past the escape speed every pair flies apart.
fn spreading(speed: f32) -> Ic<f32> {
    let dirs = [0.0f32, 2.0, 4.0].map(|k| {
        let a = k * std::f32::consts::FRAC_PI_3;
        [a.cos(), a.sin()]
    });
    let m = 1.0 / 3.0;
    Ic {
        m: [m; 3],
        r: dirs,
        p: dirs.map(|[x, y]| [m * speed * x, m * speed * y]),
    }
}

/// The harness's march labels escape, so the arms can read an escape flip: three bodies spreading at speed 10 (each
/// body's `E_rel = ½·100·(3/2)² − 1/1.5 > 0`, and the shape point fixed, the triangle growing self-similarly) end
/// `escape`, on the default occupant and on the stand-in. The label is live (escape-live-label): the march goes on to
/// `T = 4` and the label is the one at the horizon, so the stop-time half of this check, that it ended before the horizon,
/// went with the stop option (`kernel::driver::cadence`'s note).
fn check_escapes(speed: f32) {
    let u = uniforms(T).expect("T is a valid horizon");
    let ic = spreading(speed);
    for (name, got) in [
        ("Heggie x RK4", outcome::<HeggieRk4>(&ic, &u)),
        ("KDK", outcome::<Kdk>(&ic, &u)),
    ] {
        assert!(
            got.state == STATE_ESCAPE,
            "{name}: three spreading bodies did not end in escape before the horizon: state {} at t_end_step {}",
            got.state,
            got.t_end_step
        );
    }
}

#[test]
fn first_slice_march_escapes() {
    check_escapes(10.0);
}

negative_control!(
    first_slice_march_escapes,
    "three bodies released at rest, which fall together, must fail",
    expected = "did not end in escape before the horizon",
    check_escapes(0.0)
);

// ── The default occupant marches at the CPU field's values ──────────────────────────────────────────────────────────

/// `values`, Heggie's step-control values at f32, are the CPU field's (`engine::field::heggie_params`), field by field.
fn check_values(values: HgParams<f32>) {
    let field = engine::field::heggie_params();
    let words = |p: &HgParams<f32>| {
        (
            [
                p.eta,
                p.delta_sync,
                p.tiny,
                p.predictive_f,
                p.step_budget_rate,
            ]
            .map(f32::to_bits),
            (p.n_min, p.clamp, p.secant_tries, p.bisect_tries, p.time),
        )
    };
    assert!(
        words(&values) == words(&field),
        "the default occupant's Heggie values are not the CPU field's"
    );
}

#[test]
fn first_slice_default_occupant_values() {
    check_values(FieldValues::params::<f32>());
}

negative_control!(
    first_slice_default_occupant_values,
    "values with η doubled must fail",
    expected = "the default occupant's Heggie values are not the CPU field's",
    check_values(HgParams {
        eta: 0.02,
        ..FieldValues::params::<f32>()
    })
);

// ── O7: the kernel's exact feasibility predicate against a plain comparison ──────────────────────────────────────────

/// The triple `(I, K*, L_z)` at f32: `L_z = 1 + 2⁻²³`, `2K* = 1 + 2⁻²²`, `I = 1`. Exactly, `L_z² = 1 + 2⁻²² + 2⁻⁴⁶`
/// exceeds `2·I·K*`, so the target is infeasible; the f32 product `L_z·L_z` rounds the `2⁻⁴⁶` away, so a plain f32
/// comparison finds it feasible. In f64 the products of f32s are exact, so f64's plain comparison is the exact verdict.
const TRIPLE: (f32, f32, f32) = (1.0, 0.5 + 1.0 / 8_388_608.0, 1.0 + 1.0 / 8_388_608.0);

/// The plain comparison `2·I·K* < L_z²` at `R`, the prototype's.
fn plain<R: Copy + std::ops::Add<Output = R> + std::ops::Mul<Output = R> + PartialOrd>(
    i: R,
    k: R,
    l: R,
) -> bool {
    (i + i) * k < l * l
}

/// The O7 criterion on `(i, k, l)` through `predicate`: it gives f64's verdict, and the plain f32 comparison does not.
fn check_triple((i, k, l): (f32, f32, f32), predicate: fn(f32, f32, f32) -> bool) -> String {
    let wide = plain(f64::from(i), f64::from(k), f64::from(l));
    let kernel = predicate(i, k, l);
    let thin = plain(i, k, l);
    assert_eq!(
        kernel, wide,
        "the kernel's predicate does not give f64's verdict on the triple"
    );
    assert_ne!(
        thin, wide,
        "the plain f32 comparison, the control, gives f64's verdict too"
    );
    format!(
        "O7 triple (I, K*, L_z) = ({i:e}, {k:e}, {l:e}): infeasible by the f64 comparison {wide}, by the kernel's \
         predicate {kernel}, by the plain f32 comparison {thin}"
    )
}

#[test]
fn first_slice_feasibility_triple() {
    println!("{}", check_triple(TRIPLE, infeasible::<f32>));
}

negative_control!(
    first_slice_feasibility_triple,
    "the plain f32 comparison in the kernel's place must fail",
    expected = "the kernel's predicate does not give f64's verdict",
    {
        check_triple(TRIPLE, plain::<f32>);
    }
);
