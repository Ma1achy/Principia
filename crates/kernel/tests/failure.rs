//! The failure path (`kernel::driver::failure`; payload §1's storage rules, §2's `sim_failed`; philosophy §4.3).
//!
//! - `no_nan_in_storage_*`: no stored float of any sample is a NaN bit pattern, read by the scanner
//!   (`failure_support`), over synthetic samples built poisoned, samples forced to fail through the march, and real
//!   payloads; and a sample with `n < 2` reads its diffusion slope as the read side defines it.
//! - `failure_is_measurement_*`: a march forced to fail is stored with `sim_failed`, its cause by what failed, its
//!   counts up to the failure, and every f32 field the value payload §1's table defines; nothing is dropped, masked or
//!   filtered by drift.
//!
//! Every criterion here is the rule written before the run: an exact value, or a bitwise match with a reference run.

mod failure_support;

use std::cell::Cell;

#[cfg(feature = "controls")]
use failure_support::Scan;
use failure_support::{count, nans};
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::{Advance, AdvanceParams, AdvanceReal, Advanced, Cadence};
use kernel::driver::benettin::{integrate_ftle, BenettinParams};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::failure::{
    decode_failed, failed_shadow, sim_failed, DETAIL_DERIVED, DETAIL_INF, DETAIL_NAN,
};
use kernel::driver::wrapper::{integrate_detecting, integrate_with, monitored, WrapperUniforms};
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{CapabilityProfile, Geom, State, Stepper};
use kernel::payload::{
    cf_coll_floor, diffusion_slope, pa_d_min_is_unset, sd_detail, sd_dmin_pair, sd_state,
    tm_t_dmin_step, tm_t_end_step, SimStateBase, SimStateBaseOf, SimStateFTLE, PA_D_MIN_UNSET,
    SD_DMIN_PAIR_SENTINEL, STATE_DECODE_FAILED, STATE_SIM_FAILED,
};
use kernel::regularisation::az::{self, AzCount, AzP, AzParams, AzQ, AzReal, AzStepper, AzSystem};
use kernel::regularisation::heggie::{
    self as hg, HgConfig, HgP, HgParams, HgQ, HgStepper, HgSystem, DEFAULT_TIME,
};
use kernel::regularisation::logh::{self as lh, LhP, LhParams, LhQ, LhStepper, LhSystem};
use kernel::regularisation::pairs::{DefaultOccupant, LogHMode};
use validation::drift::sim_key;
use validation::negative_control;

// ── Fixtures ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Three bodies on a triangle with small momenta summing to zero, centre of mass at the origin: a finite, ordinary run.
fn triple() -> Ic<f32> {
    let m = [0.25f32, 0.35, 0.4];
    let r0 = [[-0.6f32, 0.1], [0.5, -0.35], [0.15, 0.6]];
    let p0 = [[0.05f32, 0.3], [-0.25, -0.1], [0.2, -0.2]];
    let total: f32 = m.iter().sum();
    let c = [0, 1].map(|j| (0..3).map(|i| m[i] * r0[i][j]).sum::<f32>() / total);
    Ic {
        m,
        r: r0.map(|x| [x[0] - c[0], x[1] - c[1]]),
        p: p0,
    }
}

/// [`triple`] with momenta whose squares overflow f32: `E₀` is `+∞` while every component of the state is finite.
fn overflowing() -> Ic<f32> {
    Ic {
        p: [[3e19, 0.0], [-3e19, 0.0], [0.0, 0.0]],
        ..triple()
    }
}

/// [`triple`] with bodies 0 and 1 at one point: `E₀` is `−∞` (a zero separation) on a finite state.
fn coincident() -> Ic<f32> {
    let mut ic = triple();
    ic.r[1] = ic.r[0];
    ic
}

/// A sim key of `steps` macro-steps of `1e-3` at the default substep law.
fn key(steps: u32) -> WrapperUniforms<f32> {
    WrapperUniforms {
        horizon_steps: steps,
        ..sim_key::<f32>(0.5)
    }
}

/// The run of `A` from `ic` under `key` through the wrapper and its cadence, the monitored fields written, as
/// [`integrate_detecting`] runs it at `r_coll = 0`.
fn run<A: Advance>(ic: &Ic<f32>, key: &WrapperUniforms<f32>) -> SimStateBase {
    integrate_detecting::<A, f32>(ic, key, CollisionRadius::none())
}

/// Heggie's values as the engine's field runs them (`engine::field::heggie_params`), a configuration of the
/// default occupant, Heggie × RK4.
struct Production;

impl HgConfig for Production {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.125, 0.125),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(4e4, 4e4),
            n_min: 100_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: DEFAULT_TIME,
        }
    }
}

/// Heggie × RK4 at [`Production`]'s values.
type HeggieRk4 = DefaultOccupant<Production>;

// ── The scanner sees every float (its own control, pitfalls §9) ──────────────────────────────────────────────────────

/// A scalar member of a sample, reached for writing.
type Field = fn(&mut SimStateBase) -> &mut f32;

/// The base tier's stored floats, named by hand here, apart from the scanner's own list: each set to `v` in turn on a
/// sample otherwise clean, with the name the scanner should give it.
fn base_plants(v: f32) -> Vec<(String, SimStateBase)> {
    let clean = SimStateBase::default();
    let mut out = Vec::new();
    for i in 0..3 {
        for j in 0..2 {
            let mut s = clean;
            s.r[i][j] = v;
            out.push((format!("r[{i}][{j}]"), s));
            let mut s = clean;
            s.p[i][j] = v;
            out.push((format!("p[{i}][{j}]"), s));
        }
    }
    let scalars: [(&str, Field); 10] = [
        ("S", |s| &mut s.S),
        ("theta", |s| &mut s.theta),
        ("mean_y", |s| &mut s.mean_y),
        ("C_ty", |s| &mut s.C_ty),
        ("E_0", |s| &mut s.E_0),
        ("Lz_0", |s| &mut s.Lz_0),
        ("closure_min", |s| &mut s.closure_min),
        ("_nhat_lag_0", |s| &mut s._nhat_lag_0),
        ("_nhat_lag_1", |s| &mut s._nhat_lag_1),
        ("_nhat_lag_2", |s| &mut s._nhat_lag_2),
    ];
    for (name, field) in scalars {
        let mut s = clean;
        *field(&mut s) = v;
        out.push((name.to_string(), s));
    }
    out
}

/// A binary16 NaN (quiet, payload 1) and the one half it can be planted in, `d_min`'s.
const F16_NAN: u32 = 0x7E01;

fn half_plants() -> Vec<(String, SimStateBase)> {
    let clean = SimStateBase::default();
    vec![(
        "packed_a.d_min".into(),
        SimStateBase {
            packed_a: F16_NAN << 16,
            ..clean
        },
    )]
}

/// The drift latches' `log16` halves (payload §2) hold no NaN at all: every one of the 2¹⁶ codes reads
/// back as a number, so there is nothing for the scanner to plant there.
fn check_log16_has_no_nan() {
    for h in 0..=u16::MAX {
        assert!(
            !kernel::payload::log16_to_f32(h).is_nan(),
            "the log16 code {h:#06x} reads back NaN"
        );
    }
}

/// The scanner finds a NaN planted in every float, by its name, and only there; it finds none in `±∞`, which is no NaN.
fn check_scanner(plants: Vec<(String, SimStateBase)>, nan: bool) {
    for (name, s) in plants {
        let found: Vec<String> = nans(&[SimStateBase::default(), s])
            .into_iter()
            .map(|n| format!("{}:{}", n.index, n.field))
            .collect();
        let want = if nan {
            vec![format!("1:{name}")]
        } else {
            vec![]
        };
        assert_eq!(found, want, "the scanner on a value planted in {name}");
    }
}

#[test]
fn no_nan_in_storage_scanner_sees_every_float() {
    // 6 + 6 phase-state components (three 2-vectors each), 10 scalars, 5 16-bit halves (`d_min`, `packed_b`'s two and
    // `dE_reg`'s two); the FTLE tier adds the shadow's 12.
    assert_eq!(
        count(&SimStateBase::default()),
        27,
        "the base tier's floats"
    );
    assert_eq!(
        count(&SimStateFTLE::default()),
        39,
        "the FTLE tier's floats"
    );
    assert_eq!(count(&SimStateBaseOf::<f64>::default()), 27, "at f64");
    check_log16_has_no_nan();
    for nan in [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFFFF_FFFF),
    ] {
        check_scanner(base_plants(nan), true);
    }
    check_scanner(half_plants(), true);
    check_scanner(base_plants(f32::INFINITY), false);
    check_scanner(base_plants(f32::NEG_INFINITY), false);
    // The FTLE tier's shadow, planted in its first and last components.
    for (name, plant) in [
        (
            "r_sh[0][0]",
            (|s: &mut SimStateFTLE| s.r_sh[0][0] = f32::NAN) as fn(&mut SimStateFTLE),
        ),
        ("p_sh[2][1]", |s: &mut SimStateFTLE| s.p_sh[2][1] = f32::NAN),
    ] {
        let mut s = SimStateFTLE::default();
        plant(&mut s);
        let found: Vec<String> = nans(&[s]).into_iter().map(|n| n.field).collect();
        assert_eq!(found, vec![name.to_string()], "the FTLE tier's {name}");
    }
}

/// The control: a scanner that skips the phase state would pass a NaN planted there.
#[cfg(feature = "controls")]
fn skipping_scanner_finds(s: &SimStateBase) -> usize {
    let mut found = 0;
    s.floats(&mut |name, nan| {
        if nan && !name.starts_with('r') && !name.starts_with('p') {
            found += 1;
        }
    });
    found
}

negative_control!(
    no_nan_in_storage_scanner_sees_every_float,
    "a scanner that skips r and p, fed a NaN in r[0][0]",
    expected = "the scanner missed the NaN in r[0][0]",
    {
        let mut s = SimStateBase::default();
        s.r[0][0] = f32::NAN;
        assert_eq!(
            skipping_scanner_finds(&s),
            1,
            "the scanner missed the NaN in r[0][0]"
        );
    }
);

// ── The failed-state values (payload §1's table) ─────────────────────────────────────────────────────────────────────

/// The budget word a poisoned sample carries: the exhausted bit, 5 floor hits and 3 cap hits, the counts a failed
/// run keeps (payload §1's failed-state table).
const POISONED_BUDGET_BITS: u32 = 1 | (5 << 1) | (3 << 16);

/// A sample with every f32 field `v`, the packed halves NaN, and the integer words set, as a march gone wrong might
/// leave it.
fn poisoned(v: f32) -> SimStateBase {
    SimStateBase {
        r: [[v; 2]; 3],
        p: [[v; 2]; 3],
        S: v,
        theta: v,
        mean_y: v,
        C_ty: v,
        E_0: v,
        Lz_0: v,
        packed_a: (F16_NAN << 16) | 0x0021,
        packed_b: F16_NAN | (F16_NAN << 16),
        times: 7 | (5 << 16),
        total_substeps: 31,
        budget_bits: POISONED_BUDGET_BITS,
        dE_reg: 0x3f9d_bb93,
        closure_min: v,
        closure_step: 4,
        _reserved: 0xFFFF,
        coll_floor_bits: 9,
        _nhat_lag_0: v,
        _nhat_lag_1: v,
        _nhat_lag_2: v,
        ..SimStateBase::default()
    }
}

/// Checks `s` holds a failed sample's shared words: `state`, `detail`, `d_min` unset, `dmin_pair` 3, both drift
/// halves 0.0, `t_dmin_step` 0 and `coll_floor` 0; and no NaN anywhere.
fn check_failed_words(s: &SimStateBase, state: u32, detail: u32, what: &str) {
    assert_eq!(
        (sd_state(s.packed_a), sd_detail(s.packed_a)),
        (state, detail),
        "{what}: the label"
    );
    assert!(pa_d_min_is_unset(s.packed_a), "{what}: d_min is unset");
    assert_eq!(s.packed_a >> 16, PA_D_MIN_UNSET, "{what}: d_min's bits");
    assert_eq!(
        sd_dmin_pair(s.packed_a),
        SD_DMIN_PAIR_SENTINEL,
        "{what}: dmin_pair"
    );
    assert_eq!(s.packed_b, 0, "{what}: both drift halves 0.0");
    assert_eq!(tm_t_dmin_step(s.times), 0, "{what}: t_dmin_step");
    assert_eq!(cf_coll_floor(s.coll_floor_bits), 0, "{what}: coll_floor");
    assert_eq!(nans(&[*s]), vec![], "{what}: a NaN stored");
}

#[test]
fn no_nan_in_storage_failed_state_values() {
    check_failed_state_values(sim_failed);
}

// The control: a writer that keeps the poisoned `E_0` stores the NaN that failed the sample.
negative_control!(
    no_nan_in_storage_failed_state_values,
    "a sim_failed writer that keeps E_0 as given",
    expected = "a NaN stored",
    check_failed_state_values(|s, d| SimStateBase {
        E_0: s.E_0,
        ..sim_failed(s, d)
    })
);

negative_control!(
    no_nan_in_storage_failed_state_values_regularised_drift,
    "a sim_failed writer that keeps dE_reg as given",
    expected = "the regularised drift, both halves 0.0",
    check_failed_state_values(|s, d| SimStateBase {
        dE_reg: s.dE_reg,
        ..sim_failed(s, d)
    })
);

negative_control!(
    no_nan_in_storage_failed_state_values_budget_word,
    "a sim_failed writer that zeroes budget_bits",
    expected = "the budget word is kept",
    check_failed_state_values(|s, d| SimStateBase {
        budget_bits: 0,
        ..sim_failed(s, d)
    })
);

/// `write`'s failed samples against payload §1's table: from samples poisoned NaN and `±∞`, from a finite sample, and
/// `decode_failed`'s, at f32 and f64.
fn check_failed_state_values(write: fn(SimStateBase, u32) -> SimStateBase) {
    for v in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for detail in [DETAIL_NAN, DETAIL_INF, DETAIL_DERIVED] {
            let s = write(poisoned(v), detail);
            let what = format!("write({v}, {detail})");
            check_failed_words(&s, STATE_SIM_FAILED, detail, &what);
            assert_eq!((s.r, s.p), ([[0.0; 2]; 3], [[0.0; 2]; 3]), "{what}: r, p");
            assert_eq!(s.S.to_bits(), f32::MAX.to_bits(), "{what}: S");
            for (name, x) in [
                ("theta", s.theta),
                ("mean_y", s.mean_y),
                ("C_ty", s.C_ty),
                ("E_0", s.E_0),
                ("Lz_0", s.Lz_0),
                ("_nhat_lag_0", s._nhat_lag_0),
                ("_nhat_lag_1", s._nhat_lag_1),
                ("_nhat_lag_2", s._nhat_lag_2),
            ] {
                assert_eq!(x.to_bits(), 0, "{what}: {name}");
            }
            // A NaN `closure_min` is unset, `+∞`; a `±∞` one is kept, `−∞` included, as it was given.
            let closure = if v.is_nan() { f32::INFINITY } else { v };
            assert_eq!(
                s.closure_min.to_bits(),
                closure.to_bits(),
                "{what}: closure_min"
            );
            assert_eq!(
                (tm_t_end_step(s.times), s.total_substeps, s.closure_step),
                (7, 31, 4),
                "{what}: the counts up to the failure are kept"
            );
            assert_eq!(
                s.budget_bits, POISONED_BUDGET_BITS,
                "{what}: the budget word is kept"
            );
            assert_eq!(
                s.dE_reg, 0,
                "{what}: the regularised drift, both halves 0.0"
            );
            assert_eq!(write(s, detail), s, "{what}: written twice");
        }
    }
    // A finite sample's measured values are kept.
    let finite = SimStateBase {
        r: [[0.5; 2]; 3],
        p: [[-0.25; 2]; 3],
        theta: 3.0,
        mean_y: 0.125,
        C_ty: -2.0,
        E_0: -1.5,
        Lz_0: 0.75,
        closure_min: 0.0625,
        _nhat_lag_0: 0.6,
        _nhat_lag_1: 0.0,
        _nhat_lag_2: 0.8,
        ..SimStateBase::default()
    };
    let s = write(finite, DETAIL_DERIVED);
    assert_eq!(
        (
            s.r,
            s.p,
            s.theta,
            s.mean_y,
            s.C_ty,
            s.E_0,
            s.Lz_0,
            s.closure_min
        ),
        (finite.r, finite.p, 3.0, 0.125, -2.0, -1.5, 0.75, 0.0625),
        "a finite sample's measured values"
    );
    assert_eq!(
        [s._nhat_lag_0, s._nhat_lag_1, s._nhat_lag_2],
        [0.6, 0.0, 0.8],
        "the lagged register"
    );
    // One non-finite component drops the whole state, never a vector with a component zeroed.
    let mut one = finite;
    one.p[2][1] = f32::INFINITY;
    assert_eq!(
        (write(one, DETAIL_INF).r, write(one, DETAIL_INF).p),
        ([[0.0; 2]; 3], [[0.0; 2]; 3]),
        "a state with one ∞"
    );
    // decode_failed: no march, every count 0, every float its failed-state value.
    for detail in 0..4 {
        let s = decode_failed::<f32>(detail);
        let what = format!("decode_failed({detail})");
        check_failed_words(&s, STATE_DECODE_FAILED, detail, &what);
        let want = SimStateBase {
            S: f32::MAX,
            closure_min: f32::INFINITY,
            _reserved: 0xFFFF,
            packed_a: s.packed_a,
            packed_b: 0,
            ..SimStateBase::default()
        };
        assert_eq!(s, want, "{what}: every other word");
        let f = failed_shadow(s);
        assert_eq!(
            (f.r_sh, f.p_sh, f.S.to_bits()),
            (s.r, s.p, f32::MAX.to_bits()),
            "{what}: its shadow"
        );
        assert_eq!(nans(&[f]), vec![], "{what}: the FTLE tier");
    }
    // At f64.
    let s = sim_failed(
        SimStateBaseOf::<f64> {
            E_0: f64::NAN,
            ..SimStateBaseOf::default()
        },
        2,
    );
    assert_eq!((s.E_0.to_bits(), s.S), (0, f64::MAX), "at f64");
    assert_eq!(nans(&[s]), vec![], "at f64: a NaN stored");
}

// ── A stepper forced to fail inside its march ───────────────────────────────────────────────────────────────────────

/// What [`Poisoned`] does to the step it poisons.
#[derive(Clone, Copy, Debug)]
enum Poison {
    /// `r[1][0]` NaN.
    Nan,
    /// `p[2][1]` `+∞`.
    PosInf,
    /// `r[0][1]` `−∞`.
    NegInf,
    /// Bodies 0 and 1 at one point: a finite state whose energy is `−∞`.
    Coincide,
}

thread_local! {
    /// The poison and where: macro-step, and whether on its first substep or its last.
    static PLAN: Cell<Option<(Poison, u32, bool)>> = const { Cell::new(None) };
}

/// KDK's march with one step poisoned, as [`PLAN`] says: a stepper's `ADVANCE`, its state after the poisoned step
/// handed to the cadence like any other.
struct Poisoned;

impl Advance for Poisoned {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let steps = (t_target - t_now) * params.n_sub;
        let dt = params.dt_macro / R::count(params.n_sub);
        let (mut s, mut g, mut taken) = (*state, *geom, 0);
        while taken < steps && !cadence.done() {
            (s, g) = Kdk::step(&s, dt, &g);
            if let Some((what, at, last)) = PLAN.get() {
                let here = if last { taken + 1 == steps } else { taken == 0 };
                if t_now == at && here {
                    match what {
                        Poison::Nan => s.r[1][0] = R::at_width(f32::NAN, f64::NAN),
                        Poison::PosInf => s.p[2][1] = R::at_width(f32::INFINITY, f64::INFINITY),
                        Poison::NegInf => {
                            s.r[0][1] = R::at_width(f32::NEG_INFINITY, f64::NEG_INFINITY)
                        }
                        Poison::Coincide => s.r[1] = s.r[0],
                    }
                    g = Geom::new(&s.m, &s.r);
                }
            }
            cadence.after_step(&mut s, &mut g);
            taken += 1;
        }
        Advanced {
            state: s,
            geom: g,
            substeps: taken,
        }
    }
}

/// The run of [`Poisoned`] from [`triple`] over 40 macro-steps, `what` at macro-step `at` (its first substep, or its
/// last where `last`).
fn poisoned_run(what: Poison, at: u32, last: bool) -> SimStateBase {
    PLAN.set(Some((what, at, last)));
    let out = run::<Poisoned>(&triple(), &key(40));
    PLAN.set(None);
    out
}

/// The run that failed at macro-step `at` against KDK's own run of `at` macro-steps, the march it followed until then:
/// `sim_failed` with `detail`; `t_end_step` `at`, the failed macro-step not completed; the Welford co-moments, `θ̃` and
/// the closure registers those of the `at` completed macro-steps, bit for bit; `E_0`, `Lz_0` the IC's; no NaN.
fn check_failed_at(out: &SimStateBase, at: u32, detail: u32, what: &str) -> SimStateBase {
    let before = run::<Kdk>(&triple(), &key(at));
    check_failed_words(out, STATE_SIM_FAILED, detail, what);
    assert_eq!(tm_t_end_step(out.times), at, "{what}: t_end_step");
    assert_eq!(
        (
            out.mean_y.to_bits(),
            out.C_ty.to_bits(),
            out.theta.to_bits(),
            out.closure_min.to_bits()
        ),
        (
            before.mean_y.to_bits(),
            before.C_ty.to_bits(),
            before.theta.to_bits(),
            before.closure_min.to_bits()
        ),
        "{what}: the measured fields of the completed macro-steps"
    );
    assert_eq!(
        (out.E_0.to_bits(), out.Lz_0.to_bits()),
        (before.E_0.to_bits(), before.Lz_0.to_bits()),
        "{what}: E_0, Lz_0"
    );
    assert_eq!(out.S.to_bits(), f32::MAX.to_bits(), "{what}: S");
    before
}

#[test]
fn failure_is_measurement_stepper_cause_and_counts() {
    for (what, detail) in [
        (Poison::Nan, DETAIL_NAN),
        (Poison::PosInf, DETAIL_INF),
        (Poison::NegInf, DETAIL_INF),
        (Poison::Coincide, DETAIL_DERIVED),
    ] {
        let name = format!("{what:?} on macro-step 10's first substep");
        let out = poisoned_run(what, 10, false);
        let before = check_failed_at(&out, 10, detail, &name);
        // The last finite state is the end of macro-step 9, the reference run's last, bit for bit; its substeps and the
        // failed one are counted.
        assert_eq!((out.r, out.p), (before.r, before.p), "{name}: r, p");
        assert_eq!(
            out.total_substeps,
            before.total_substeps + 1,
            "{name}: total_substeps"
        );
        let name = format!("{what:?} on macro-step 10's last substep");
        let out = poisoned_run(what, 10, true);
        check_failed_at(&out, 10, detail, &name);
    }
    // A failure is a stored, counted outcome among the others, never dropped: a batch of runs keeps every sample.
    let batch = [
        poisoned_run(Poison::Nan, 3, false),
        run::<Kdk>(&triple(), &key(40)),
        poisoned_run(Poison::PosInf, 0, false),
        run::<Kdk>(&overflowing(), &key(40)),
    ];
    let states: Vec<u32> = batch.iter().map(|s| sd_state(s.packed_a)).collect();
    assert_eq!(
        states,
        vec![STATE_SIM_FAILED, 1, STATE_SIM_FAILED, STATE_SIM_FAILED],
        "every sample stored with its outcome"
    );
    assert_eq!(nans(&batch), vec![], "the batch: a NaN stored");
}

// The control: the cause read after the COM projection. A `+∞` in one component meets `∞ − ∞` there, so the
// projected state holds a NaN and reads 0, not 1: detection on the projected state alone would misread every `∞`.
negative_control!(
    failure_is_measurement_stepper_cause_and_counts,
    "the cause read on the projected state, which turns an ∞ into a NaN",
    expected = "an ∞ read after the projection",
    {
        let ic = triple();
        let projection = kernel::driver::project::ComProjection::new(&ic.m);
        let mut p = ic.p;
        p[2][1] = f32::INFINITY;
        let (r, p) = projection.project(&ic.r, &p);
        assert_eq!(
            kernel::driver::failure::state_detail(&r, &p),
            DETAIL_INF,
            "an ∞ read after the projection"
        );
    }
);

/// A cadence that detects nothing: the controls' march, which runs on past a failure.
#[cfg(feature = "controls")]
struct Blind;

#[cfg(feature = "controls")]
impl Cadence<f32> for Blind {
    fn after_step(&mut self, _state: &mut State<f32>, _geom: &mut Geom<f32>) {}
    fn done(&self) -> bool {
        false
    }
}

// ── A march that fails before its first step, and the FTLE tier ─────────────────────────────────────────────────────

// The control: a cadence that detects nothing marches the overflowing IC on, unmarked.
negative_control!(
    failure_is_measurement_at_step_zero,
    "the overflowing IC under a cadence that detects nothing",
    expected = "the label",
    check_failed_words(
        &integrate_with::<Kdk, f32, _>(&overflowing(), &key(40), &mut Blind),
        STATE_SIM_FAILED,
        DETAIL_DERIVED,
        "a blind march"
    )
);

#[test]
fn failure_is_measurement_at_step_zero() {
    for (name, ic) in [
        ("overflowing momenta", overflowing()),
        ("coincident bodies", coincident()),
    ] {
        for (occupant, out) in [
            ("KDK", run::<Kdk>(&ic, &key(40))),
            ("Heggie × RK4", run::<HeggieRk4>(&ic, &key(40))),
        ] {
            let what = format!("{name} under {occupant}");
            check_failed_words(&out, STATE_SIM_FAILED, DETAIL_DERIVED, &what);
            assert_eq!(
                (tm_t_end_step(out.times), out.total_substeps),
                (0, 0),
                "{what}: no step taken"
            );
            assert_eq!((out.r, out.p), (ic.r, ic.p), "{what}: the IC stored");
            assert!(
                out.E_0 == 0.0 && out.Lz_0.is_finite(),
                "{what}: E_0 {}",
                out.E_0
            );
        }
        let ftle = integrate_ftle::<Kdk, f32>(
            &ic,
            &key(40),
            CollisionRadius::none(),
            BenettinParams {
                delta_0: 1e-4,
                n_renorm: 4,
            },
            false,
        );
        assert_eq!(nans(&[ftle]), vec![], "{name}: the FTLE tier");
        assert_eq!(
            (ftle.r_sh, ftle.p_sh, ftle.S.to_bits()),
            (ftle.r, ftle.p, f32::MAX.to_bits()),
            "{name}: the failed shadow"
        );
    }
}

// ── A regularised march's step-7 exit, by cause ──────────────────────────────────────────────────────────────────────

thread_local! {
    /// The regularised step to poison, counting from 1, and with what: NaN where true, `+∞` where false.
    static REG: Cell<(u32, bool)> = const { Cell::new((0, true)) };
    static CALLS: Cell<u32> = const { Cell::new(0) };
}

/// Whether this regularised step is the one to poison, and the value to poison it with.
fn reg_poison<R: AzReal>() -> Option<R> {
    let n = CALLS.get() + 1;
    CALLS.set(n);
    let (at, nan) = REG.get();
    (n == at).then(|| {
        if nan {
            R::at_width(f32::NAN, f64::NAN)
        } else {
            R::at_width(f32::INFINITY, f64::INFINITY)
        }
    })
}

/// RK4's regularised step, one component poisoned as [`REG`] says.
struct RegPoisoned;

impl HgStepper for RegPoisoned {
    fn pair_profile() -> CapabilityProfile {
        <Rk4 as HgStepper>::pair_profile()
    }
    fn regularised_step<R: AzReal>(
        s: &HgSystem<R>,
        q: &HgQ<R>,
        p: &HgP<R>,
        h: R,
    ) -> (HgQ<R>, HgP<R>) {
        let (mut q, p) = <Rk4 as HgStepper>::regularised_step(s, q, p, h);
        if let Some(v) = reg_poison::<R>() {
            q.q[1][0] = v;
        }
        (q, p)
    }
}

impl AzStepper for RegPoisoned {
    fn pair_profile() -> CapabilityProfile {
        <Rk4 as AzStepper>::pair_profile()
    }
    fn regularised_step<R: AzReal>(
        s: &AzSystem<R>,
        q: &AzQ<R>,
        p: &AzP<R>,
        h: R,
    ) -> (AzQ<R>, AzP<R>) {
        let (q, mut p) = <Rk4 as AzStepper>::regularised_step(s, q, p, h);
        if let Some(v) = reg_poison::<R>() {
            p.p2[1] = v;
        }
        (q, p)
    }
}

impl LhStepper for RegPoisoned {
    fn pair_profile(mode: LogHMode) -> CapabilityProfile {
        <Rk4 as LhStepper>::pair_profile(mode)
    }
    fn regularised_step<R: AzReal>(
        s: &LhSystem<R>,
        q: &LhQ<R>,
        p: &LhP<R>,
        h: R,
    ) -> (LhQ<R>, LhP<R>) {
        let (mut q, p) = <Rk4 as LhStepper>::regularised_step(s, q, p, h);
        if let Some(v) = reg_poison::<R>() {
            q.r[1][0] = v;
        }
        (q, p)
    }
}

/// Aarseth–Zare's values as its tests run them (`tests/az.rs`).
fn az_params() -> AzParams<f32> {
    AzParams {
        eta: 0.01,
        delta_sync: 0.125,
        tiny: 1e-37,
        predictive_f: 0.02,
        step_budget_rate: 1e7,
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
    }
}

/// logH's values as its seam tests run them (`tests/logh_seam.rs`), in the logH mode.
fn lh_params() -> LhParams<f32> {
    LhParams {
        eta: 0.01,
        delta_sync: 0.125,
        tiny: 1e-37,
        predictive_f: 0.02,
        step_budget_rate: 4e4,
        n_min: 100_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        mode: LogHMode::LogH,
    }
}

/// A cadence with no work of its own that keeps the causes the occupant tells it ([`Cadence::non_finite`]), every one,
/// so a test reads what the march itself reported and nothing a per-step detection decided first.
#[derive(Default)]
struct Told(Vec<u32>);

impl Cadence<f32> for Told {
    fn after_step(&mut self, _state: &mut State<f32>, _geom: &mut Geom<f32>) {}
    fn done(&self) -> bool {
        false
    }
    fn non_finite(&mut self, detail: u32) {
        self.0.push(detail);
    }
}

/// The march `march` from [`triple`] with step 5 poisoned NaN (`nan`) or `+∞`: the one cause it reported.
fn reg_cause(nan: bool, march: fn(&State<f32>, &mut Told)) -> u32 {
    let ic = triple();
    let s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    CALLS.set(0);
    REG.set((5, nan));
    let mut told = Told::default();
    march(&s, &mut told);
    REG.set((0, true));
    assert_eq!(told.0.len(), 1, "the poisoned march reported {:?}", told.0);
    told.0[0]
}

fn heggie_march(s: &State<f32>, c: &mut Told) {
    let out =
        hg::march::<RegPoisoned, f32, _, _>(s, 4, &Production::params(), c, &mut az::ExplicitTest);
    assert_eq!(
        out.count.end,
        az::End::NonFinite,
        "Heggie's march ends at its finiteness test"
    );
}

fn az_march(s: &State<f32>, c: &mut Told) {
    let out = az::march::<RegPoisoned, f32, _, _>(
        s,
        0,
        4,
        AzCount::new(),
        &az_params(),
        c,
        &mut az::ExplicitTest,
    );
    assert_eq!(
        out.count.end,
        az::End::NonFinite,
        "Aarseth–Zare's march ends at its finiteness test"
    );
}

fn logh_march(s: &State<f32>, c: &mut Told) {
    let out = lh::march::<RegPoisoned, f32, _, _>(s, 4, &lh_params(), c, &mut az::ExplicitTest);
    assert_eq!(
        out.count.end,
        az::End::NonFinite,
        "logH's march ends at its finiteness test"
    );
}

#[test]
fn failure_is_measurement_regularised_cause() {
    for (name, march) in [
        ("Heggie", heggie_march as fn(&State<f32>, &mut Told)),
        ("Aarseth–Zare", az_march),
        ("logH", logh_march),
    ] {
        assert_eq!(
            reg_cause(true, march),
            DETAIL_NAN,
            "{name}: a NaN in the rejected state"
        );
        assert_eq!(
            reg_cause(false, march),
            DETAIL_INF,
            "{name}: an ∞ in the rejected state"
        );
    }
    // A non-finite step size: Heggie's entry rate at coincident bodies, and Aarseth–Zare's infinite entry size at
    // three bodies within `TINY` of each other (`tests/az.rs`'s doubly degenerate fixture).
    let c = coincident();
    let s = State {
        m: c.m,
        r: c.r,
        p: c.p,
    };
    let mut told = Told::default();
    let out = hg::march::<Rk4, f32, _, _>(
        &s,
        1,
        &Production::params(),
        &mut told,
        &mut az::ExplicitTest,
    );
    assert_eq!(
        out.count.end,
        az::End::NonFinite,
        "Heggie at coincident bodies"
    );
    assert_eq!(
        told.0,
        vec![DETAIL_DERIVED],
        "Heggie's non-finite entry rate"
    );
    let d = State {
        m: [1.0 / 3.0; 3],
        r: [[0.0, 0.0], [1e-38, 0.0], [0.0, 2e-38]],
        p: [[0.0; 2]; 3],
    };
    let mut p = az_params();
    p.n_min = 64;
    p.step_budget_rate = 0.0;
    let mut told = Told::default();
    let out = az::march::<Rk4, f32, _, _>(
        &d,
        0,
        1,
        AzCount::new(),
        &p,
        &mut told,
        &mut az::ExplicitTest,
    );
    assert_eq!(
        out.count.end,
        az::End::NonFinite,
        "Aarseth–Zare doubly degenerate"
    );
    assert_eq!(
        told.0,
        vec![DETAIL_DERIVED],
        "Aarseth–Zare's infinite step size"
    );
}

// The control: the interim rule, the cause read on the latched state the march stored, reads 2 for a NaN.
negative_control!(
    failure_is_measurement_regularised_cause,
    "the step-7 cause read on the latched, finite state",
    expected = "the latched state's cause",
    {
        let ic = triple();
        let s = State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        };
        CALLS.set(0);
        REG.set((5, true));
        let out = hg::march::<RegPoisoned, f32, _, _>(
            &s,
            4,
            &Production::params(),
            &mut Told::default(),
            &mut az::ExplicitTest,
        );
        REG.set((0, true));
        assert_eq!(
            kernel::driver::failure::state_detail(&out.state.r, &out.state.p),
            DETAIL_NAN,
            "the latched state's cause"
        );
    }
);

// ── Real payloads, and the diffusion below two samples ──────────────────────────────────────────────────────────────

// The control: the poisoned march under a cadence that detects nothing runs on past the failure, so the scan of its
// payload finds the NaN.
negative_control!(
    no_nan_in_storage_real_payloads,
    "the poisoned march under a cadence that detects no failure",
    expected = "a NaN stored",
    {
        PLAN.set(Some((Poison::Nan, 10, false)));
        let out = integrate_with::<Poisoned, f32, _>(&triple(), &key(40), &mut Blind);
        PLAN.set(None);
        assert_eq!(nans(&[out]), vec![], "a NaN stored");
    }
);

#[test]
fn no_nan_in_storage_real_payloads() {
    // A 6 × 6 grid of momenta around [`triple`]'s under KDK and Heggie × RK4, and the FTLE tier under KDK.
    let mut base = Vec::new();
    let mut ftle = Vec::new();
    for a in 0..6 {
        for b in 0..6 {
            let mut ic = triple();
            ic.p[0][0] += 0.1 * a as f32;
            ic.p[1][1] -= 0.1 * b as f32;
            ic.p[2] = [-(ic.p[0][0] + ic.p[1][0]), -(ic.p[0][1] + ic.p[1][1])];
            base.push(run::<Kdk>(&ic, &key(200)));
            base.push(run::<HeggieRk4>(&ic, &key(200)));
            ftle.push(integrate_ftle::<Kdk, f32>(
                &ic,
                &key(200),
                CollisionRadius::none(),
                BenettinParams {
                    delta_0: 1e-4,
                    n_renorm: 4,
                },
                false,
            ));
        }
    }
    assert_eq!(nans(&base), vec![], "the base tier");
    assert_eq!(nans(&ftle), vec![], "the FTLE tier");
}

#[test]
fn no_nan_in_storage_diffusion_below_two() {
    check_diffusion(diffusion_slope);
}

// The control: the task's −1.0 marker in place of the read side's NaN.
negative_control!(
    no_nan_in_storage_diffusion_below_two,
    "a slope that reads −1.0 below two samples",
    expected = "not NaN",
    check_diffusion(|c, n, h| if n < 2 {
        -1.0
    } else {
        diffusion_slope(c, n, h)
    })
);

/// The diffusion slope `slope` reads of real samples with `n` = 0, 1 and 2.
fn check_diffusion(slope: fn(f32, u32, f32) -> f32) {
    // The plan is unclear here: the task asks a sample with n < 2 to read diffusion −1.0, while payload §4 and the
    // generated read side read NaN there (no −1 marker); chose the read side's NaN, because the read side is
    // generated from the ledger and the code is the source of truth. Storage holds the co-moments, finite, never NaN.
    let h = key(1).dt_macro;
    for steps in [0u32, 1] {
        let out = run::<Kdk>(&triple(), &key(steps));
        assert_eq!(tm_t_end_step(out.times), steps, "n = {steps}");
        assert!(
            out.mean_y.is_finite() && out.C_ty.is_finite(),
            "n = {steps}: the stored co-moments"
        );
        let slope = slope(out.C_ty, tm_t_end_step(out.times), h);
        assert!(
            slope.is_nan(),
            "n = {steps}: the slope reads {slope}, not NaN"
        );
    }
    let out = run::<Kdk>(&triple(), &key(2));
    let two = slope(out.C_ty, tm_t_end_step(out.times), h);
    assert!(two.is_finite(), "n = 2: the slope reads {two}");
}

/// The failed sample written through the cadence's write, as a resumed dispatch finishes one: [`monitored`] keeps it.
#[test]
fn failure_is_measurement_written_once() {
    check_written(monitored);
}

// The control: a write that relabels the failed sample by the horizon it reached.
negative_control!(
    failure_is_measurement_written_once,
    "a write that relabels a failed sample bounded",
    expected = "the cadence's write: the label",
    check_written(|o, c| {
        let w = monitored(o, c);
        SimStateBase {
            packed_a: kernel::payload::set_state(w.packed_a, 1),
            ..w
        }
    })
);

/// `write`, the cadence's write of a finished run, keeps a failed sample failed, with its words, and twice is once.
fn check_written(write: fn(SimStateBase, &WrapperCadence<f32>) -> SimStateBase) {
    PLAN.set(Some((Poison::PosInf, 4, false)));
    let ic = triple();
    let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let out = integrate_with::<Poisoned, f32, _>(&ic, &key(40), &mut cadence);
    PLAN.set(None);
    assert_eq!(
        sd_state(out.packed_a),
        STATE_SIM_FAILED,
        "the march's own write"
    );
    let written = write(out, &cadence);
    check_failed_words(
        &written,
        STATE_SIM_FAILED,
        DETAIL_INF,
        "the cadence's write",
    );
    assert_eq!(write(written, &cadence), written, "written twice");
}

/// A measurement behind a changed expectation (`tests/logh_seam.rs`'s `logh_seam_non_finite_fails`): logH's TTL mode at
/// two coincident bodies of Burrau's triangle. Its entry size is finite and its first step's size is finite, and that
/// step's regularised state holds a NaN, so the step-7 cause is 0 by the ruled rule; the logH mode's entry size is not
/// finite, so it exits at its entry with 2. Criterion: under TTL, `h` finite and a NaN in the stepped state, at f32 and
/// f64; under logH, the entry rate not finite.
fn check_logh_coincident<R: AzReal + std::fmt::Debug>(coincide: bool) {
    let m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
    let r = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f64; 2]| x.map(|c| c / 5.0));
    let p = [[0.03, -0.01], [-0.02, 0.04], [-0.01, -0.03]];
    let of = |x: f64| R::at_width(x as f32, x);
    let mut s = State {
        m: m.map(of),
        r: r.map(|x| x.map(of)),
        p: p.map(|x| x.map(of)),
    };
    if coincide {
        s.r[1] = s.r[0];
    }
    for mode in [LogHMode::Ttl, LogHMode::LogH] {
        let params = LhParams {
            eta: of(0.01),
            delta_sync: of(0.125),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: of(0.02),
            step_budget_rate: of(1e7),
            n_min: 1_000_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            mode,
        };
        let reg = lh::register(&s, params.tiny, mode);
        let rate = reg.system.dt_ds(&reg.p);
        let entry = params.eta * params.delta_sync / rate;
        let h = lh::step_size(&reg.system, &reg.q, &reg.p, entry, &params).h;
        let (q1, p1) = <Rk4 as LhStepper>::regularised_step(&reg.system, &reg.q, &reg.p, h);
        let stepped = [q1.r[0], q1.r[1], q1.r[2], p1.p[0], p1.p[1], p1.p[2]];
        let nan =
            stepped.iter().flatten().any(|x| x.nan_bits()) || q1.t.nan_bits() || p1.w.nan_bits();
        println!("logH {mode:?} at coincident bodies: entry rate {rate:?}, h {h:?}, NaN in the stepped state {nan}");
        if mode == LogHMode::Ttl {
            assert!(!h.non_finite_bits() && nan, "{mode:?}: h {h:?}, NaN {nan}");
        } else {
            assert!(
                rate.non_finite_bits() || rate <= R::default(),
                "{mode:?}: entry rate {rate:?}"
            );
        }
    }
}

#[test]
fn failure_is_measurement_logh_coincident_cause() {
    check_logh_coincident::<f32>(true);
    check_logh_coincident::<f64>(true);
}

// The control: Burrau's triangle with its bodies apart, whose first step is finite.
negative_control!(
    failure_is_measurement_logh_coincident_cause,
    "the measurement on bodies apart",
    expected = "NaN false",
    check_logh_coincident::<f32>(false)
);
