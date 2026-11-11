//! The regularisation axis, `none`, the pair table and the largest-step diagnostic (`kernel::regularisation`;
//! integrator_contract Part 2b, Part 2a, Part 2c; R-19, R-561):
//! - `regularisation_compose_selects`: every stepper × regularisation pair, under every time transformation, `none`
//!   and the measured-bad pairs included, is selected as asked; `none` is among the kinds and the rows; its control is
//!   a selection that blocks the measured-bad pairs (REQ-INT-017's arm here; R-561).
//! - `regularisation_compose_warnings`: every cell of the shipped table is Part 2b's, transcribed here; a measured-bad
//!   pair carries the contract's warning and every other pair none; its control is a measured-bad pair with no warning.
//! - `regularisation_compose_cells`: the table holds one cell per stepper × regularisation × time transformation: a
//!   distinct status written to each of the 30 is read back from it; its control leaves one cell unwritten.
//! - `regularisation_compose_none_runs`: each of the five steppers composed with `none` runs, and its `ADVANCE` is the
//!   stepper's own, bit for bit, at f64 and f32, the cadence called once per step; its control compares against another
//!   stepper.
//! - `owns_time_mapping_composed`: over every stepper × regularisation pair, selected or composed as a type,
//!   `owns_time_mapping == (regularisation != none)`, whatever the regularisation's own profile says, and the rest of
//!   the composed profile is the pair's; its control reports the stepper's flag for a Heggie pair (REQ-INT-021). The
//!   kinds not built yet are stood in for by stubs here; Aarseth–Zare's own arm is TASK-M3-68's.
//! - `no_usable_step_sentinel`: a trajectory with no step, or with every step unusable, reports the explicit sentinel,
//!   not `0.0`; a zero step reads apart from it; the `2.209e128` overflow is reported as itself at f64 and is skipped
//!   and counted at f32, where it is `+∞`; the fold is reseeded from the first usable step; its control is the fold
//!   from `0.0` (REQ-TOOL-039; pitfalls §9).
//! - `dt_max_skipped_nan_is_counted`: a NaN among finite steps is skipped and counted, so the report reads apart from
//!   the same steps without it, at f64 and f32; its control drops the NaN uncounted.
//! - `dt_max_skipped_infinity_is_counted`: `+∞` and `−∞` among finite steps are skipped and counted, first, last or
//!   between, at f64 and f32 (Part 2b's step 8, "when finite"); its control folds every step that is a number.
//! - `no_usable_step_sentinel_counts_skipped`: a trajectory of non-finite steps alone reports the sentinel with their
//!   count, at f64 and f32, an all-overflow f32 trajectory among them; its control is a sentinel with no count.
//! - `dt_max_skipped_count_saturates`: the count of skipped steps stops at `u32::MAX`; its control wraps.

use kernel::driver::advance::{Advance, AdvanceParams, AdvanceReal, Advanced, Cadence};
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{
    CapabilityProfile, DebugOccupant, Geom, Occupant, Selection, State, Stepper,
};
use kernel::real::IeeeBits;
use kernel::regularisation::dt_max::{DtMax, DtMaxReport};
use kernel::regularisation::pairs::{
    Choice, HeggieTime, LogHMode, Pair, PairTable, Status, HEGGIE_KDK_WARNING, STEPPERS,
};
use kernel::regularisation::{Composed, Kind, NoRegularisation, Regularisation};
use validation::negative_control;

// ── Part 2b's table, transcribed ───────────────────────────────────────────────────────────────────────────────

/// The five steppers in the table's column order: Euler (debug), KDK, Yoshida-4, Yoshida-6, RK4.
const COLUMNS: [Selection; 5] = [
    Selection::Debug(DebugOccupant::Euler),
    Selection::Production(Occupant::Kdk),
    Selection::Production(Occupant::Yoshida4),
    Selection::Production(Occupant::Yoshida6),
    Selection::Production(Occupant::Rk4),
];

/// The six rows in the table's order.
const ROWS: [Choice; 6] = [
    Choice::None,
    Choice::AarsethZare,
    Choice::Heggie(HeggieTime::Eq20),
    Choice::Heggie(HeggieTime::Eq22),
    Choice::LogH(LogHMode::LogH),
    Choice::LogH(LogHMode::Ttl),
];

/// Part 2b's table, row by row: `-` not measured, `G` measured good, `B` measured bad.
const PART_2B: [[char; 5]; 6] = [
    ['-', '-', '-', '-', '-'],
    ['-', '-', '-', '-', 'G'],
    ['-', 'B', '-', '-', 'G'],
    ['-', 'B', '-', '-', 'G'],
    ['-', '-', '-', '-', '-'],
    ['-', '-', '-', '-', '-'],
];

/// The warning Part 2b's note 2 gives Heggie × KDK.
const WARNING: &str = "Heggie's Hamiltonian is not separable, so a leapfrog step is first order here and can diverge \
                       (figure-eight: order about 1, diverged within 100 orbits).";

/// Every pair: `(row, column, regularisation, stepper)`.
fn pairs() -> impl Iterator<Item = (usize, usize, Choice, Selection)> {
    ROWS.into_iter().enumerate().flat_map(|(i, c)| {
        COLUMNS
            .into_iter()
            .enumerate()
            .map(move |(j, s)| (i, j, c, s))
    })
}

// ── Selection ──────────────────────────────────────────────────────────────────────────────────────────────────

/// Checks that `select` gives every pair, as asked.
fn check_selects(select: impl Fn(Selection, Choice) -> Option<Pair>) {
    for (_, _, c, s) in pairs() {
        let pair = select(s, c).unwrap_or_else(|| panic!("{s:?} × {c:?} cannot be selected"));
        assert_eq!(
            (pair.stepper, pair.regularisation),
            (s, c),
            "the pair selected is not the pair asked for"
        );
    }
}

#[test]
fn regularisation_compose_selects() {
    assert_eq!(STEPPERS, COLUMNS, "the steppers, in the table's order");
    assert_eq!(Choice::ALL, ROWS, "the rows, in the table's order");
    assert_eq!(
        Kind::ALL,
        [Kind::None, Kind::AarsethZare, Kind::Heggie, Kind::LogH],
        "Part 2b's four kinds"
    );
    assert_eq!(
        ROWS.map(Choice::kind),
        [
            Kind::None,
            Kind::AarsethZare,
            Kind::Heggie,
            Kind::Heggie,
            Kind::LogH,
            Kind::LogH
        ],
        "each row's regularisation"
    );
    assert_eq!(NoRegularisation::KIND, Kind::None, "`none` is built");
    check_selects(|s, c| Some(Pair::select(s, c)));
}

negative_control!(
    regularisation_compose_selects,
    "a selection that blocks the measured-bad pairs",
    expected = "cannot be selected",
    check_selects(|s, c| {
        let pair = Pair::select(s, c);
        pair.warning(&PairTable::part_2b())
            .is_none()
            .then_some(pair)
    })
);

// ── The table's cells and warnings ─────────────────────────────────────────────────────────────────────────────

/// Checks `table` against Part 2b's: each cell's status, the contract's warning on each measured-bad pair and none on
/// any other.
fn check_warnings(table: &PairTable) {
    for (i, j, c, s) in pairs() {
        let pair = Pair::select(s, c);
        let status = pair.status(table);
        let warning = pair.warning(table);
        assert_eq!(status, table.cell(s, c), "a pair's status is its cell's");
        match PART_2B[i][j] {
            'B' => {
                assert!(
                    matches!(status, Status::MeasuredBad { .. }),
                    "{s:?} × {c:?} is measured bad in Part 2b, not {status:?}"
                );
                assert!(
                    warning.is_some_and(|w| !w.is_empty()),
                    "{s:?} × {c:?} is measured bad but carries no warning"
                );
                assert_eq!(warning, Some(WARNING), "the warning is Part 2b's");
            }
            mark => {
                let want = if mark == 'G' {
                    Status::MeasuredGood
                } else {
                    Status::NotMeasured
                };
                assert_eq!(status, want, "{s:?} × {c:?}'s status is not Part 2b's");
                assert_eq!(
                    warning, None,
                    "{s:?} × {c:?} carries a warning unmeasured bad"
                );
            }
        }
    }
}

#[test]
fn regularisation_compose_warnings() {
    assert_eq!(HEGGIE_KDK_WARNING, WARNING, "the shipped warning");
    check_warnings(&PairTable::part_2b());
}

negative_control!(
    regularisation_compose_warnings,
    "Heggie (Eq. 22) × KDK measured bad with an empty warning",
    expected = "is measured bad but carries no warning",
    check_warnings(&PairTable::part_2b().with(
        COLUMNS[1],
        ROWS[3],
        Status::MeasuredBad { warning: "" }
    ))
);

/// A distinct warning per cell.
const LABELS: [[&str; 5]; 6] = [
    ["00", "01", "02", "03", "04"],
    ["10", "11", "12", "13", "14"],
    ["20", "21", "22", "23", "24"],
    ["30", "31", "32", "33", "34"],
    ["40", "41", "42", "43", "44"],
    ["50", "51", "52", "53", "54"],
];

/// The table with every cell but `skip` written with its label.
fn labelled(skip: Option<(usize, usize)>) -> PairTable {
    pairs().filter(|&(i, j, _, _)| skip != Some((i, j))).fold(
        PairTable::not_measured(),
        |t, (i, j, c, s)| {
            t.with(
                s,
                c,
                Status::MeasuredBad {
                    warning: LABELS[i][j],
                },
            )
        },
    )
}

/// Checks that every cell of `table` holds its own label.
fn check_cells(table: &PairTable) {
    for (i, j, c, s) in pairs() {
        assert_eq!(
            table.cell(s, c).warning(),
            Some(LABELS[i][j]),
            "the cell of {s:?} × {c:?} is not its own"
        );
    }
}

#[test]
fn regularisation_compose_cells() {
    for (_, _, c, s) in pairs() {
        assert_eq!(
            PairTable::not_measured().cell(s, c),
            Status::NotMeasured,
            "an empty table's cell"
        );
    }
    assert_eq!(Status::NotMeasured.warning(), None);
    assert_eq!(Status::MeasuredGood.warning(), None);
    check_cells(&labelled(None));
}

negative_control!(
    regularisation_compose_cells,
    "a table with the logH (TTL) × Yoshida-6 cell unwritten",
    expected = "is not its own",
    check_cells(&labelled(Some((5, 3))))
);

// ── `none` runs ────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the tests run at.
trait P: AdvanceReal + Into<f64> {
    fn of(x: f64) -> Self;
}

impl P for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
}

impl P for f64 {
    fn of(x: f64) -> f64 {
        x
    }
}

/// A cadence that counts its calls.
#[derive(Default)]
struct Counting {
    calls: u32,
}

impl<R> Cadence<R> for Counting {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.calls += 1;
    }
    fn done(&self) -> bool {
        false
    }
}

/// Three bodies of masses 3, 4, 5 on Burrau's triangle, with momenta summing to zero.
fn fixture<R: P>() -> (State<R>, Geom<R>) {
    let v = |a: [[f64; 2]; 3]| a.map(|x| x.map(R::of));
    let m = [3.0, 4.0, 5.0].map(R::of);
    let r = v([[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]]);
    let p = v([[0.3, -0.1], [-0.2, 0.4], [-0.1, -0.3]]);
    (State { m, r, p }, Geom::new(&m, &r))
}

/// `A`'s `ADVANCE` over two macro-steps at `N_sub = 3`, with the cadence's calls.
fn run<A: Advance, R: P>() -> (Advanced<R>, u32) {
    let (s0, g0) = fixture::<R>();
    let params = AdvanceParams {
        dt_macro: R::of(1e-2),
        n_sub: 3,
    };
    let mut cadence = Counting::default();
    let out = A::advance(&s0, &g0, 5, 7, &params, &mut cadence);
    (out, cadence.calls)
}

/// The bits of a state's positions and momenta.
fn bits<R: P>(s: &State<R>) -> Vec<u64> {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .map(|&x| Into::<f64>::into(x).to_bits())
        .collect()
}

/// Checks that the occupant `A` runs and that its `ADVANCE` is the stepper `S`'s own, bit for bit.
fn check_runs<A: Advance, S: Stepper, R: P>(name: &str) {
    let (got, calls) = run::<A, R>();
    let (want, _) = run::<S, R>();
    assert_eq!(got.substeps, 6, "{name}: two macro-steps at N_sub = 3");
    assert_eq!(calls, 6, "{name}: the cadence is called once per step");
    assert_ne!(
        bits(&got.state),
        bits(&fixture::<R>().0),
        "{name}: the state did not move"
    );
    assert_eq!(
        bits(&got.state),
        bits(&want.state),
        "{name} ({}): the composed ADVANCE is not the stepper's own",
        R::NAME
    );
    assert_eq!(got, want, "{name}: the whole answer");
}

/// Runs `$f::<Stepper, …>(name)` for each of the five steppers.
macro_rules! each_stepper {
    ($f:ident $(, $t:ty)*) => {{
        $f::<Euler $(, $t)*>("Euler");
        $f::<Kdk $(, $t)*>("KDK");
        $f::<Yoshida4 $(, $t)*>("Yoshida-4");
        $f::<Yoshida6 $(, $t)*>("Yoshida-6");
        $f::<Rk4 $(, $t)*>("RK4");
    }};
}

fn none_runs<S: Stepper, R: P>(name: &str) {
    check_runs::<Composed<S, NoRegularisation>, S, R>(name);
}

#[test]
fn regularisation_compose_none_runs() {
    each_stepper!(none_runs, f64);
    each_stepper!(none_runs, f32);
}

negative_control!(
    regularisation_compose_none_runs,
    "RK4 × none compared with KDK's own ADVANCE",
    expected = "the composed ADVANCE is not the stepper's own",
    check_runs::<Composed<Rk4, NoRegularisation>, Kdk, f64>("control")
);

// ── owns_time_mapping ──────────────────────────────────────────────────────────────────────────────────────────

/// A stand-in regularisation of kind `$kind` whose own profile says `owns_time_mapping` `$owns`: the stepper's march
/// and the stepper's profile otherwise.
macro_rules! stub {
    ($name:ident, $kind:expr, $owns:expr) => {
        struct $name;

        impl Regularisation for $name {
            const KIND: Kind = $kind;

            fn profile<S: Stepper>() -> CapabilityProfile {
                CapabilityProfile {
                    owns_time_mapping: $owns,
                    ..S::profile()
                }
            }

            fn advance<S: Stepper, R: AdvanceReal, C: Cadence<R>>(
                state: &State<R>,
                geom: &Geom<R>,
                t_now: u32,
                t_target: u32,
                params: &AdvanceParams<R>,
                cadence: &mut C,
            ) -> Advanced<R> {
                <S as Advance>::advance(state, geom, t_now, t_target, params, cadence)
            }
        }
    };
}

stub!(AzStub, Kind::AarsethZare, false);
stub!(HeggieStub, Kind::Heggie, false);
stub!(LogHStub, Kind::LogH, false);
// `none`'s kind with a profile that claims the time mapping.
stub!(ClaimingNone, Kind::None, true);

/// Checks the flag `owns` reported for a pair under a regularisation of `kind`.
fn check_owns(who: &str, kind: Kind, owns: bool) {
    assert_eq!(
        owns,
        kind != Kind::None,
        "{who}: owns_time_mapping is not (regularisation != none)"
    );
}

/// Checks the composed profile of `S` under `G`: the flag, and every other field the pair's.
fn composed_owns<S: Stepper, G: Regularisation>(name: &str) {
    let profile = Composed::<S, G>::profile();
    check_owns(name, G::KIND, profile.owns_time_mapping);
    assert_eq!(
        profile,
        CapabilityProfile {
            owns_time_mapping: G::KIND != Kind::None,
            ..G::profile::<S>()
        },
        "{name}: the rest of the profile is the pair's"
    );
}

#[test]
fn owns_time_mapping_composed() {
    for kind in Kind::ALL {
        check_owns("the kind", kind, kind.owns_time_mapping());
    }
    for (_, _, c, s) in pairs() {
        let pair = Pair::select(s, c);
        check_owns(
            &format!("{s:?} × {c:?}"),
            c.kind(),
            pair.owns_time_mapping(),
        );
    }
    each_stepper!(composed_owns, NoRegularisation);
    each_stepper!(composed_owns, AzStub);
    each_stepper!(composed_owns, HeggieStub);
    each_stepper!(composed_owns, LogHStub);
    each_stepper!(composed_owns, ClaimingNone);
    assert_eq!(
        Composed::<Yoshida4, NoRegularisation>::profile(),
        Yoshida4::profile(),
        "a stepper under `none` keeps its own profile"
    );
}

negative_control!(
    owns_time_mapping_composed,
    "a Heggie pair reporting its stepper's flag",
    expected = "owns_time_mapping is not (regularisation != none)",
    check_owns(
        "control",
        HeggieStub::KIND,
        HeggieStub::profile::<Kdk>().owns_time_mapping
    )
);

// ── dt_max ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The diagnostic's fold of `steps`.
fn dt_max<R: IeeeBits>(steps: &[R]) -> DtMaxReport<R> {
    steps
        .iter()
        .fold(DtMax::new(), |d, &dt| d.observe(dt))
        .report()
}

/// The diagnostic's fold of `steps` at f32, each step rounded to f32 first and the report widened back.
fn dt_max_at_f32(steps: &[f64]) -> DtMaxReport<f64> {
    let steps: Vec<f32> = steps.iter().map(|&dt| dt as f32).collect();
    match dt_max(&steps) {
        DtMaxReport::NoUsableStep { skipped } => DtMaxReport::NoUsableStep { skipped },
        DtMaxReport::Largest { dt_max, skipped } => DtMaxReport::Largest {
            dt_max: f64::from(dt_max),
            skipped,
        },
    }
}

/// A fold written apart from the kernel's, for the controls: the largest of the steps `usable` passes, seeded from
/// the first, with the others counted where `counted`.
#[cfg(feature = "controls")]
fn model(steps: &[f64], usable: impl Fn(f64) -> bool, counted: bool) -> DtMaxReport<f64> {
    let kept: Vec<f64> = steps.iter().copied().filter(|&dt| usable(dt)).collect();
    let skipped = if counted {
        u32::try_from(steps.len() - kept.len()).expect("a short fixture")
    } else {
        0
    };
    match kept.split_first() {
        None => DtMaxReport::NoUsableStep { skipped },
        Some((&first, rest)) => DtMaxReport::Largest {
            dt_max: rest.iter().copied().fold(first, f64::max),
            skipped,
        },
    }
}

/// `Largest { dt_max, skipped }`.
fn largest(dt_max: f64, skipped: u32) -> DtMaxReport<f64> {
    DtMaxReport::Largest { dt_max, skipped }
}

/// The overflowed physical step the diagnostic was built to catch (pitfalls §9).
const OVERFLOW: f64 = 2.209e128;

/// Checks a largest-step fold over f64 steps.
fn check_dt_max(fold: impl Fn(&[f64]) -> DtMaxReport<f64>) {
    let nan = f64::NAN;
    for (unusable, skipped) in [(&[][..], 0), (&[nan], 1), (&[nan, -nan, nan], 3)] {
        assert_eq!(
            fold(unusable),
            DtMaxReport::NoUsableStep { skipped },
            "no usable step must be the sentinel ({unusable:?})"
        );
    }
    assert_eq!(fold(&[nan, 0.0, nan]), largest(0.0, 2));
    assert_eq!(
        fold(&[1e-3, OVERFLOW, 1e-3, nan]),
        largest(OVERFLOW, 1),
        "the overflow case is reported"
    );
    assert_eq!(fold(&[1.0, 3.0, 2.0]), largest(3.0, 0));
    assert_eq!(fold(&[3.0, 1.0]), largest(3.0, 0));
    assert_eq!(
        fold(&[-3.0, -1.0, -2.0]),
        largest(-1.0, 0),
        "the fold is reseeded from the first usable step"
    );
    // An equal step does not replace the maximum: `−0.0`, then `+0.0`, stays `−0.0`.
    match fold(&[-0.0, 0.0]) {
        DtMaxReport::Largest { dt_max, skipped } => {
            assert_eq!(dt_max.to_bits(), (-0.0f64).to_bits(), "an equal step");
            assert_eq!(skipped, 0);
        }
        DtMaxReport::NoUsableStep { .. } => panic!("a zero step is usable"),
    }
}

#[test]
fn no_usable_step_sentinel() {
    check_dt_max(dt_max::<f64>);
    assert_eq!(
        DtMax::<f64>::default().report(),
        DtMaxReport::NoUsableStep { skipped: 0 }
    );
    assert_eq!(
        dt_max(&[0.0f64]),
        largest(0.0, 0),
        "a step of zero is not `no usable step`"
    );
    // At f32 the overflow is `+∞`: skipped and counted, never the maximum.
    assert_eq!(dt_max::<f32>(&[]), DtMaxReport::NoUsableStep { skipped: 0 });
    assert_eq!(
        dt_max(&[f32::NAN, f32::NAN]),
        DtMaxReport::NoUsableStep { skipped: 2 }
    );
    assert_eq!(
        dt_max(&[1e-3f32, OVERFLOW as f32, f32::NAN]),
        DtMaxReport::Largest {
            dt_max: 1e-3f32,
            skipped: 2
        }
    );
    assert_eq!(
        dt_max(&[-2.0f32, -5.0]),
        DtMaxReport::Largest {
            dt_max: -2.0f32,
            skipped: 0
        }
    );
    assert_eq!(
        dt_max(&[0.0f32]),
        DtMaxReport::Largest {
            dt_max: 0.0f32,
            skipped: 0
        }
    );
}

negative_control!(
    no_usable_step_sentinel,
    "the fold from 0.0, which reports an all-unusable trajectory's largest step as zero",
    expected = "no usable step must be the sentinel",
    check_dt_max(|steps| largest(steps.iter().copied().fold(0.0, f64::max), 0))
);

/// Checks that a fold skips and counts a NaN among finite steps. The steps are exact at f32.
fn check_nan_counted(fold: impl Fn(&[f64]) -> DtMaxReport<f64>) {
    let nan = f64::NAN;
    let m = "a skipped NaN must be counted";
    assert_eq!(fold(&[0.5]), largest(0.5, 0), "{m}");
    assert_eq!(fold(&[0.5, nan]), largest(0.5, 1), "{m}");
    assert_ne!(fold(&[0.5, nan]), fold(&[0.5]), "{m}");
    assert_eq!(fold(&[nan, 0.5]), largest(0.5, 1), "{m}");
    assert_eq!(fold(&[0.25, nan, 2.0, -nan, 0.5]), largest(2.0, 2), "{m}");
    assert_eq!(fold(&[nan, -4.0, nan, nan]), largest(-4.0, 3), "{m}");
}

#[test]
fn dt_max_skipped_nan_is_counted() {
    check_nan_counted(dt_max::<f64>);
    check_nan_counted(dt_max_at_f32);
}

negative_control!(
    dt_max_skipped_nan_is_counted,
    "a fold that drops a NaN step and leaves no trace of it",
    expected = "a skipped NaN must be counted",
    check_nan_counted(|steps| model(steps, |dt| !dt.is_nan(), false))
);

/// Checks that a fold skips and counts `+∞` and `−∞` among finite steps. The steps are exact at f32.
fn check_infinity_counted(fold: impl Fn(&[f64]) -> DtMaxReport<f64>) {
    let (pos, neg) = (f64::INFINITY, f64::NEG_INFINITY);
    let m = "an infinite step must be skipped and counted";
    assert_eq!(fold(&[1.0, pos, 2.0]), largest(2.0, 1), "{m}");
    assert_eq!(fold(&[pos, 1.0]), largest(1.0, 1), "{m}");
    assert_eq!(fold(&[1.0, pos]), largest(1.0, 1), "{m}");
    assert_eq!(fold(&[neg, -3.0]), largest(-3.0, 1), "{m}");
    assert_eq!(fold(&[-3.0, neg]), largest(-3.0, 1), "{m}");
    assert_eq!(fold(&[2.0, neg, 1.0]), largest(2.0, 1), "{m}");
    assert_eq!(fold(&[pos, neg, 0.5, pos]), largest(0.5, 3), "{m}");
    assert_ne!(fold(&[1.0, pos]), fold(&[1.0]), "{m}");
    assert_ne!(fold(&[1.0, neg]), fold(&[1.0]), "{m}");
    // The largest finite step is finite, so usable, at each width.
    assert_eq!(
        fold(&[f64::from(f32::MAX), pos]),
        largest(f64::from(f32::MAX), 1),
        "{m}"
    );
    assert_eq!(
        fold(&[f64::from(f32::MIN), neg]),
        largest(f64::from(f32::MIN), 1),
        "{m}"
    );
}

#[test]
fn dt_max_skipped_infinity_is_counted() {
    check_infinity_counted(dt_max::<f64>);
    check_infinity_counted(dt_max_at_f32);
    assert_eq!(dt_max(&[f64::MAX, f64::INFINITY]), largest(f64::MAX, 1));
    assert_eq!(dt_max(&[f64::MIN, f64::NEG_INFINITY]), largest(f64::MIN, 1));
}

negative_control!(
    dt_max_skipped_infinity_is_counted,
    "the fold of every step that is a number, which reports an infinite step as the largest",
    expected = "an infinite step must be skipped and counted",
    check_infinity_counted(|steps| model(steps, |dt| !dt.is_nan(), true))
);

/// Checks that a fold of non-finite steps alone reports the sentinel with their count.
fn check_sentinel_count(fold: impl Fn(&[f64]) -> DtMaxReport<f64>) {
    let (nan, pos, neg) = (f64::NAN, f64::INFINITY, f64::NEG_INFINITY);
    let m = "the sentinel must carry the count of skipped steps";
    let sentinel = |skipped| DtMaxReport::NoUsableStep { skipped };
    assert_eq!(fold(&[]), sentinel(0), "{m}");
    assert_eq!(fold(&[pos]), sentinel(1), "{m}");
    assert_eq!(fold(&[neg]), sentinel(1), "{m}");
    assert_eq!(fold(&[nan, pos, neg]), sentinel(3), "{m}");
    assert_eq!(fold(&[neg, neg, pos, nan, nan]), sentinel(5), "{m}");
    assert_ne!(fold(&[nan]), fold(&[]), "{m}");
}

#[test]
fn no_usable_step_sentinel_counts_skipped() {
    check_sentinel_count(dt_max::<f64>);
    check_sentinel_count(dt_max_at_f32);
    // At f32 every `2.209e128` step is `+∞`: the all-overflow trajectory is the sentinel, with its count, where at
    // f64 the same steps report the overflow itself.
    assert_eq!(
        dt_max_at_f32(&[OVERFLOW, OVERFLOW, OVERFLOW]),
        DtMaxReport::NoUsableStep { skipped: 3 }
    );
    assert_eq!(
        dt_max(&[OVERFLOW, OVERFLOW, OVERFLOW]),
        largest(OVERFLOW, 0)
    );
}

negative_control!(
    no_usable_step_sentinel_counts_skipped,
    "a sentinel that says nothing of the steps skipped",
    expected = "the sentinel must carry the count of skipped steps",
    check_sentinel_count(|steps| model(steps, f64::is_finite, false))
);

/// Checks a count of skipped steps: `count(already, more)` is the count reported after `already` skipped steps and
/// then `more` non-finite steps observed.
fn check_saturation(count: impl Fn(u32, u32) -> u32) {
    let m = "the count of skipped steps must saturate";
    assert_eq!(count(0, 0), 0, "{m}");
    assert_eq!(count(7, 2), 9, "{m}");
    assert_eq!(count(u32::MAX - 2, 1), u32::MAX - 1, "{m}");
    assert_eq!(count(u32::MAX - 1, 1), u32::MAX, "{m}");
    assert_eq!(count(u32::MAX - 1, 2), u32::MAX, "{m}");
    assert_eq!(count(u32::MAX, 3), u32::MAX, "{m}");
}

/// The kernel's count: `already` steps skipped, then `more` steps observed, NaN and `±∞` in turn, around a finite one.
fn skipped_count<R: IeeeBits + From<f32>>(already: u32, more: u32) -> u32 {
    let non_finite = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    let fold = (0..more).fold(
        DtMax::<R>::new()
            .observe(<R as From<f32>>::from(1.0))
            .skipping(already),
        |d, i| d.observe(<R as From<f32>>::from(non_finite[i as usize % 3])),
    );
    match fold.report() {
        DtMaxReport::Largest { skipped, .. } => skipped,
        DtMaxReport::NoUsableStep { .. } => panic!("a finite step was observed"),
    }
}

#[test]
fn dt_max_skipped_count_saturates() {
    check_saturation(skipped_count::<f64>);
    check_saturation(skipped_count::<f32>);
    // `skipping` saturates by itself too, and leaves the largest step alone.
    assert_eq!(
        DtMax::new()
            .observe(2.0f64)
            .skipping(u32::MAX)
            .skipping(u32::MAX)
            .report(),
        largest(2.0, u32::MAX)
    );
    assert_eq!(
        DtMax::<f64>::new().skipping(4).report(),
        DtMaxReport::NoUsableStep { skipped: 4 }
    );
}

negative_control!(
    dt_max_skipped_count_saturates,
    "a count that wraps past its largest value",
    expected = "the count of skipped steps must saturate",
    check_saturation(u32::wrapping_add)
);
