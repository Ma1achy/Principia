//! The two energies under a regularised occupant, and the drift latches' `log16` form (integrator_contract Part 1,
//! "Under a regularised occupant, two energies, named"; R-566, R-605, R-632; payload §1, §2, §5):
//! - `energy_named_*` (REQ-VAL-195): under Heggie × RK4 the stored `dE_reg` is the occupant's own regularised drift, as
//!   it reported it, and the suspect predicate reads it, never the Cartesian `dE_max`, which differs from it after a
//!   close passage; with regularisation off (RK4 alone) the two energies are one and `dE_reg` holds the Cartesian drift.
//! - `dE_reg_encoding_*` (REQ-PAY-111): on dd_integrator §5's tests 2 and 7 (REQ-INT-106's fixtures) a round-off
//!   regularised drift at f64 reads back non-zero and within the form's stated resolution, a relative 2⁻⁹; final and
//!   max read back ordered, and distinct where they differ by more than the resolution.
//! - `packed_b_encoding_*` (REQ-PAY-112): `dE_max` and `dLz_max` pack through `dE_reg`'s form, a drift at the form's
//!   floor `2⁻⁹⁶` reads back non-zero within its resolution, a failed state reads 0.0, and the march's branch reads the
//!   live f32 latch, never the unpacked one.
//!
//! Every pass criterion is written here before the runs it reads. The step-control values are the CPU field's
//! (`engine::field`: `η = 0.01`, `Δ_sync = 1/8`, `f = 0.02`), with a budget that does not bind: test inputs.

// The acceptance filters name the payload's fields as the ledger spells them, `dE_reg`.
#![allow(non_snake_case)]

use kernel::driver::advance::{Advance, AdvanceParams, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::carried::Carried;
use kernel::driver::latch::Latches;
use kernel::driver::substep::Quantise;
use kernel::driver::wrapper::{integrate_with, monitored, WrapperReal};
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    dr_dE_reg_final, dr_dE_reg_max, f32_to_log16, log16_to_f32, pack_dE_reg, pack_packed_b,
    pb_dE_max, pb_dLz_max, set_state, suspect_energy, DminCounters, PayloadReal, SimStateBaseOf,
    LOG16_OCTAVE_LO, STATE_BOUNDED, STATE_SIM_FAILED,
};
use kernel::regularisation::az::AzReal;
use kernel::regularisation::heggie::{Heggie, HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::Composed;
use validation::drift::{close_encounter, figure_eight, ic_at, sim_key, AsR, Fixture};
use validation::negative_control;

/// The form's stated resolution: round to nearest on an 8-bit fraction, a relative error of at most 2⁻⁹ (payload §2,
/// "The `log16` form").
const RESOLUTION: f64 = 1.0 / 512.0;

/// The CPU field's Heggie values with a budget that never binds on these runs.
struct Field;

impl HgConfig for Field {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.125, 0.125),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(1e7, 1e7),
            n_min: 100_000_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: DEFAULT_TIME,
        }
    }
}

type HgRk4 = Composed<Rk4, Heggie<Field>>;

/// The wrapper's cadence, every call passed on, with the regularised drifts the occupant reported recorded at the
/// working precision: the last, and the greatest magnitude.
struct Recording<R: WrapperReal> {
    inner: WrapperCadence<R>,
    last: f64,
    max: f64,
    reports: u64,
    /// Whether to pass the drift reports on: false is the control, a cadence that drops them.
    forward: bool,
}

impl<R: WrapperReal + Into<f64>> Cadence<R> for Recording<R> {
    fn after_step(&mut self, s: &mut State<R>, g: &mut Geom<R>) {
        self.inner.after_step(s, g);
    }
    fn done(&self) -> bool {
        self.inner.done()
    }
    fn sample(&mut self, s: &State<R>, i: u32, q: R) {
        self.inner.sample(s, i, q);
    }
    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }
    fn clock(&mut self, c: u32) {
        self.inner.clock(c);
    }
    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }
    fn regularised_drift(&mut self, drift: R) {
        let d: f64 = drift.into();
        self.last = d;
        self.max = self.max.max(d.abs());
        self.reports += 1;
        if self.forward {
            self.inner.regularised_drift(drift);
        }
    }
}

/// One run of `A` at `R` on `fixture`, its stored state and its recording.
fn run<A: Advance, R: WrapperReal + Into<f64>>(
    fixture: &Fixture,
    forward: bool,
) -> (SimStateBaseOf<R>, Recording<R>)
where
    f64: AsR<R>,
{
    let ic = ic_at::<R>(&fixture.ic);
    let key = sim_key::<R>(fixture.horizon);
    let mut rec = Recording {
        inner: WrapperCadence::new(&ic.m, &ic.r, &ic.p),
        last: 0.0,
        max: 0.0,
        reports: 0,
        forward,
    };
    let out = integrate_with::<A, R, _>(&ic, &key, &mut rec);
    (monitored(out, &rec.inner), rec)
}

/// `x` at `R`, rounded to f32 as the write rounds it, in the `log16` form, decoded: what `dE_reg` holds for a value.
fn stored_as(x: f64) -> f32 {
    log16_to_f32(f32_to_log16(x as f32))
}

// ── energy_named (REQ-VAL-195) ─────────────────────────────────────────────────────────────────────────────────

/// Asserts, of Heggie × RK4 at f64 on test 7's close encounters (run with the drift reports passed on or not, as
/// `forward`): `dE_reg`'s halves are the regularised drift the occupant reported, final and max, bitwise as stored;
/// they differ from the Cartesian latch, the regularised drift staying smaller; the suspect predicate reads `dE_reg`.
fn check_named(forward: bool) {
    let (out, rec) = run::<HgRk4, f64>(&close_encounter(), forward);
    let m = rec.inner.monitor_state();
    let (cart_final, cart_max) = (m.de_final(), m.de_max());
    let (reg_final, reg_max) = (dr_dE_reg_final(out.dE_reg), dr_dE_reg_max(out.dE_reg));
    println!(
        "test 7, Heggie × RK4, f64: {} reports; regularised final {:.3e} max {:.3e}, stored {reg_final:.3e} \
         {reg_max:.3e}; Cartesian final {cart_final:.3e} max {cart_max:.3e}, dE_max stored {:.3e}",
        rec.reports,
        rec.last,
        rec.max,
        pb_dE_max(out.packed_b)
    );
    assert!(
        rec.reports > 0,
        "the occupant reported no regularised drift"
    );
    assert_eq!(
        (reg_final.to_bits(), reg_max.to_bits()),
        (stored_as(rec.last).to_bits(), stored_as(rec.max).to_bits()),
        "dE_reg is not the regularised drift the occupant reported"
    );
    assert!(
        f64::from(reg_max) * 4.0 < cart_max,
        "the stored regularised maximum {reg_max:e} is not below the Cartesian {cart_max:e}: the Cartesian was stored"
    );
    let read = kernel::payload::SimState {
        dE_reg_max: reg_max,
        dE_max: pb_dE_max(out.packed_b),
        E_0: 0.0,
        ..Default::default()
    };
    // At `E_0 = 0` the relative drift is `latch / ε_E`, so an `ε_E` at the two latches' geometric mean over `τ_E` lights
    // a Cartesian reader and not the regularised one.
    let eps = (f64::from(reg_max) * pb_dE_max(out.packed_b) as f64).sqrt() as f32
        / kernel::payload::SUSPECT_ENERGY_THRESHOLD;
    assert!(
        suspect_energy(read.dE_max, read.E_0, eps),
        "the discriminating ε_E does not light the Cartesian latch"
    );
    let lit = kernel::payload::drift_suspect(&read, eps, false).energy;
    assert_eq!(
        lit,
        suspect_energy(read.dE_reg_max, read.E_0, eps),
        "the suspect predicate does not read dE_reg_max"
    );
    assert!(!lit, "the suspect predicate lit on the Cartesian drift");
}

#[test]
fn energy_named_regularised_drift_is_stored_and_read() {
    check_named(true);
}

negative_control!(
    energy_named_regularised_drift_is_stored_and_read,
    "a cadence that drops the occupant's drift reports leaves dE_reg the Cartesian drift",
    expected = "dE_reg is not the regularised drift the occupant reported",
    check_named(false)
);

/// Asserts that with regularisation off (RK4 alone, at `R`, on test 7) no regularised drift is reported and `dE_reg`
/// holds the Cartesian drift, final and max, the two energies being one; `stored` is the write under test.
fn check_one_energy<R: WrapperReal + Into<f64> + Quantise>(stored: fn(f64, f64) -> u32)
where
    f64: AsR<R>,
{
    let (out, rec) = run::<Rk4, R>(&close_encounter(), true);
    let m = rec.inner.monitor_state();
    let (last, max) = (m.de_final().quantise(), m.de_max().quantise());
    assert_eq!(rec.reports, 0, "a stepper reported a regularised drift");
    assert_eq!(
        out.dE_reg,
        stored(f64::from(last), f64::from(max)),
        "with regularisation off dE_reg is not the Cartesian drift"
    );
}

#[test]
fn energy_named_one_energy_with_regularisation_off() {
    let write = |l: f64, m: f64| pack_dE_reg(l as f32, m as f32);
    check_one_energy::<f64>(write);
    check_one_energy::<f32>(write);
}

negative_control!(
    energy_named_one_energy_with_regularisation_off,
    "dE_reg left 0 under a stepper is not the one energy",
    expected = "with regularisation off dE_reg is not the Cartesian drift",
    check_one_energy::<f64>(|_, _| 0)
);

// ── dE_reg_encoding (REQ-PAY-111) ──────────────────────────────────────────────────────────────────────────────

/// An encoding of the two halves under test: `(final, max)` in, the two decoded values out.
type Halves = fn(f64, f64) -> (f64, f64);

/// The ledger's form: each half through `dE_reg`'s packer and accessors.
fn log16_halves(last: f64, max: f64) -> (f64, f64) {
    let w = pack_dE_reg(last as f32, max as f32);
    (f64::from(dr_dE_reg_final(w)), f64::from(dr_dE_reg_max(w)))
}

/// Asserts, of Heggie × RK4 at f64 on tests 2 and 7, that `halves` holds the regularised drift: the maximum non-zero
/// and within [`RESOLUTION`], the final within it too, `|final| ≤ max` as read back, and distinct where the live ones
/// differ by more than twice the resolution.
fn check_encoding(halves: Halves) {
    for fixture in [figure_eight(), close_encounter()] {
        let (_, rec) = run::<HgRk4, f64>(&fixture, true);
        let (last, max) = halves(rec.last, rec.max);
        println!(
            "{}: live final {:.4e} max {:.4e}; read back {last:.4e} {max:.4e}",
            fixture.name, rec.last, rec.max
        );
        assert!(
            rec.max > 0.0 && rec.max < 1e-9,
            "{}: the live drift {:e} is not a round-off drift, outside this check's domain",
            fixture.name,
            rec.max
        );
        assert!(
            max != 0.0,
            "{}: a round-off drift reads back zero",
            fixture.name
        );
        assert!(
            last.abs() <= max,
            "{}: final and max read back out of order",
            fixture.name
        );
        if rec.last.abs() < rec.max * (1.0 - 2.0 * RESOLUTION) {
            assert!(
                last.abs() < max,
                "{}: final and max collapse where they differ",
                fixture.name
            );
        }
        let rel = |got: f64, want: f64| ((got - want) / want).abs();
        assert!(
            rel(max, rec.max) <= RESOLUTION && rel(last, rec.last) <= RESOLUTION,
            "{}: the halves read back {last:e}, {max:e}, not within 2⁻⁹ of {:e}, {:e}",
            fixture.name,
            rec.last,
            rec.max
        );
    }
}

#[test]
fn dE_reg_encoding_round_off_held_and_ordered() {
    check_encoding(log16_halves);
}

negative_control!(
    dE_reg_encoding_round_off_held_and_ordered,
    "linear binary16 halves flush an f64 round-off drift to zero",
    expected = "a round-off drift reads back zero",
    check_encoding(|l, m| {
        use kernel::payload::{f16_bits_to_f32, f32_to_f16_bits};
        let h = |x: f64| f64::from(f16_bits_to_f32(f32_to_f16_bits(x as f32)));
        (h(l), h(m))
    })
);

/// A second control: the maximum written into both halves, so final and max collapse.
#[cfg(feature = "controls")]
mod collapse {
    use super::*;
    negative_control!(
        dE_reg_encoding_round_off_held_and_ordered,
        "the maximum written into both halves collapses final and max",
        expected = "final and max collapse where they differ",
        check_encoding(|_, m| log16_halves(m, m))
    );
}

// ── packed_b_encoding (REQ-PAY-112) ────────────────────────────────────────────────────────────────────────────

/// A `packed_b` packer under test.
type PackB = fn(f32, f32) -> u32;

/// Asserts `pack` puts `dE_max` and `dLz_max` in `dE_reg`'s form: over values from the floor `2⁻⁹⁶` to `10⁹`, each half's
/// bits are the `log16` code `dE_reg` stores for the value, each reads back non-zero within [`RESOLUTION`], and a drift
/// at the floor reads back as the floor itself.
fn check_packed_b(pack: PackB) {
    let floor = 2f32.powi(LOG16_OCTAVE_LO);
    let mut x = floor;
    while x < 1e9 {
        let b = pack(x, 3.0 * x);
        assert_eq!(
            (b as u16, (b >> 16) as u16),
            (f32_to_log16(x), f32_to_log16(3.0 * x)),
            "packed_b's halves at {x:e} are not dE_reg's log16 form"
        );
        for (got, want) in [(pb_dE_max(b), x), (pb_dLz_max(b), 3.0 * x)] {
            let rel = ((f64::from(got) - f64::from(want)) / f64::from(want)).abs();
            assert!(
                got != 0.0 && rel <= RESOLUTION,
                "{want:e} reads back {got:e}, not non-zero within 2⁻⁹"
            );
        }
        x *= 7.3;
    }
    assert_eq!(
        pb_dE_max(pack(floor, 0.0)),
        floor,
        "the floor does not read back as itself"
    );
}

#[test]
fn packed_b_encoding_is_dE_reg_s_form() {
    check_packed_b(pack_packed_b);
}

negative_control!(
    packed_b_encoding_is_dE_reg_s_form,
    "packed_b packed as linear binary16 while dE_reg's halves are log16",
    expected = "are not dE_reg's log16 form",
    check_packed_b(|a, b| kernel::payload::pack2x16float([a, b]))
);

/// Asserts a failed state's drift halves read 0.0 and a valid state's march branch reads the live f32 latch (`live`
/// the reader under test): a drift of `1.0e-30`, which the form rounds, reads exactly itself.
fn check_failed_and_live(live: fn(&Latches) -> f32) {
    let mut l = Latches::unset();
    l.drift(1.0e-30, -2.5e-12);
    let (_, failed) = l.persist(set_state(0, STATE_SIM_FAILED), &DminCounters::new());
    assert_eq!(
        (pb_dE_max(failed), pb_dLz_max(failed)),
        (0.0, 0.0),
        "a failed state's drift halves do not read 0.0"
    );
    let (_, valid) = l.persist(set_state(0, STATE_BOUNDED), &DminCounters::new());
    assert!(
        pb_dE_max(valid) != 1.0e-30,
        "the fixture's drift is exact in the form"
    );
    assert_eq!(
        live(&l),
        1.0e-30,
        "the march's branch does not read the live f32 latch"
    );
}

#[test]
fn packed_b_encoding_failed_zero_and_live_branch() {
    check_failed_and_live(Latches::de_max);
}

negative_control!(
    packed_b_encoding_failed_zero_and_live_branch,
    "a branch fed the unpacked latch reads the form's rounding",
    expected = "does not read the live f32 latch",
    check_failed_and_live(|l| {
        let (_, b) = l.persist(set_state(0, STATE_BOUNDED), &DminCounters::new());
        pb_dE_max(b)
    })
);

/// `R`'s payload type is used by the read side at f32 only: a compile-time witness that `dE_reg` exists at both widths.
#[allow(dead_code)]
fn widths<R: PayloadReal>(s: &SimStateBaseOf<R>) -> (u32, u32) {
    (s.dE_reg, s.budget_bits)
}

// ── The energy-drift readout's "stopped at event" (REQ-TOOL-219, the data and the string) ──────────────────────────

/// Asserts `readout` says "stopped at event" of an escape and a collision and nothing of a bounded, running or failed
/// run, and that the diagnostics' filter keeps the Cartesian drift of exactly the runs that did not stop at an event and
/// did not fail.
fn check_readout(readout: fn(u32) -> Option<&'static str>) {
    use kernel::payload::drift::cartesian_drift_readable;
    use kernel::payload::{
        STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE, STATE_RUNNING, STATE_SIM_FAILED,
    };
    for (state, words, readable) in [
        (STATE_ESCAPE, Some("stopped at event"), false),
        (STATE_COLLISION, Some("stopped at event"), false),
        (STATE_BOUNDED, None, true),
        (STATE_RUNNING, None, true),
        (STATE_SIM_FAILED, None, false),
        (STATE_DECODE_FAILED, None, false),
    ] {
        let a = set_state(0, state);
        assert_eq!(
            readout(a),
            words,
            "state {state}: the readout's words are wrong"
        );
        assert_eq!(
            cartesian_drift_readable(a),
            readable,
            "state {state}: the diagnostics' event filter is wrong"
        );
    }
}

#[test]
fn energy_named_drift_readout_stopped_at_event() {
    check_readout(|a| kernel::payload::drift::DriftReadout::of(a).words());
}

negative_control!(
    energy_named_drift_readout_stopped_at_event,
    "a readout without the words",
    expected = "the readout's words are wrong",
    check_readout(|_| None)
);

// ── Every regularised occupant's drift (physics review 6003366749 of PR #32, note 5) ──────────────────────────────

/// Aarseth–Zare's and logH's values, the CPU field's step control with a budget that never binds.
struct Every;

impl kernel::regularisation::az::AzConfig for Every {
    fn params<R: AzReal>() -> kernel::regularisation::az::AzParams<R> {
        let v = <Field as HgConfig>::params::<R>();
        kernel::regularisation::az::AzParams {
            eta: v.eta,
            delta_sync: v.delta_sync,
            tiny: v.tiny,
            predictive_f: v.predictive_f,
            step_budget_rate: v.step_budget_rate,
            n_min: v.n_min,
            clamp: v.clamp,
            secant_tries: v.secant_tries,
            bisect_tries: v.bisect_tries,
        }
    }
}

impl kernel::regularisation::logh::LhConfig for Every {
    fn params<R: AzReal>() -> kernel::regularisation::logh::LhParams<R> {
        let v = <Field as HgConfig>::params::<R>();
        kernel::regularisation::logh::LhParams {
            eta: v.eta,
            delta_sync: v.delta_sync,
            tiny: v.tiny,
            predictive_f: v.predictive_f,
            step_budget_rate: v.step_budget_rate,
            n_min: v.n_min,
            clamp: v.clamp,
            secant_tries: v.secant_tries,
            bisect_tries: v.bisect_tries,
            mode: kernel::regularisation::pairs::LogHMode::LogH,
        }
    }
}

type AzRk4 = Composed<Rk4, kernel::regularisation::az::AarsethZare<Every>>;
type LhRk4 = Composed<Rk4, kernel::regularisation::logh::LogH<Every>>;

/// What a reported drift reads under a model of a wrong form: `(drift, E_0) → drift`.
type Form = fn(f64, f64) -> f64;

/// Asserts that `A`'s reported regularised drift on test 7 at f64, through `form`, starts at round-off (its first kept
/// step within 1e-12 of `|E_0|`) and stays small (its maximum within 1e-6 of `|E_0|`): a drift read against the wrong
/// reference, or with the wrong sign, is of the order of `E_0` from the first step.
fn check_small<A: Advance>(name: &str, form: Form) {
    let ic = ic_at::<f64>(&close_encounter().ic);
    let key = sim_key::<f64>(close_encounter().horizon);
    let mut rec = First {
        inner: WrapperCadence::new(&ic.m, &ic.r, &ic.p),
        first: None,
        max: 0.0,
    };
    let _ = integrate_with::<A, f64, _>(&ic, &key, &mut rec);
    let e_0 = rec.inner.monitor_state().e_0().abs();
    let first = form(rec.first.expect("a drift reported"), e_0);
    let max = form(rec.max, e_0);
    println!("{name}: first drift {first:.3e}, max {max:.3e}, |E_0| {e_0:.4}");
    assert!(
        first.abs() < 1e-12 * e_0 && max.abs() < 1e-6 * e_0,
        "{name}: the reported drift {first:e} (max {max:e}) is not a drift of the energy against its reference"
    );
}

/// The wrapper's cadence with the first reported drift and the greatest magnitude recorded.
struct First {
    inner: WrapperCadence<f64>,
    first: Option<f64>,
    max: f64,
}

impl Cadence<f64> for First {
    fn after_step(&mut self, s: &mut State<f64>, g: &mut Geom<f64>) {
        self.inner.after_step(s, g);
    }
    fn done(&self) -> bool {
        self.inner.done()
    }
    fn regularised_drift(&mut self, drift: f64) {
        self.first.get_or_insert(drift);
        self.max = self.max.max(drift.abs());
    }
}

#[test]
fn energy_named_every_occupant_reports_a_small_drift() {
    check_small::<HgRk4>("Heggie × RK4", |d, _| d);
    check_small::<AzRk4>("Aarseth–Zare × RK4", |d, _| d);
    check_small::<LhRk4>("logH × RK4", |d, _| d);
}

negative_control!(
    energy_named_every_occupant_reports_a_small_drift,
    "logH's drift with B's sign flipped, E − B = (E + B) − 2B and B = −E_0, reads 2|E_0| off",
    expected = "is not a drift of the energy against its reference",
    check_small::<LhRk4>("logH × RK4, sign flipped", |d, e_0| d + 2.0 * e_0)
);

// ── Aarseth–Zare's drift reference across seam calls (code review 6003876254 note 5; physics 6003854656 note 10) ──

/// Every regularised drift a march reported, by its bits, in order.
#[derive(Default)]
struct Drifts(Vec<u64>);

impl Cadence<f64> for Drifts {
    fn after_step(&mut self, _: &mut State<f64>, _: &mut Geom<f64>) {}
    fn done(&self) -> bool {
        false
    }
    fn regularised_drift(&mut self, drift: f64) {
        self.0.push(drift.to_bits());
    }
}

/// The macro-steps of the seam march: forty sync intervals.
const SEAM_CALLS: u32 = 40;

/// Aarseth–Zare × RK4 on test 7 at f64, `dt_macro = Δ_sync = 1/8`, from `t = 0` to [`SEAM_CALLS`] macro-steps: in one
/// `ADVANCE` call when `split` is false, else in one call per macro-step with the carried state passed on, `seam`
/// applied to it before each call. Returns every drift reported.
fn az_drifts(split: bool, seam: fn(&mut Carried<f64>)) -> Vec<u64> {
    let ic = ic_at::<f64>(&close_encounter().ic);
    let params = AdvanceParams {
        dt_macro: 0.125,
        n_sub: 4,
    };
    let mut state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let mut carried = Carried::unregistered();
    let mut drifts = Drifts::default();
    let spans: Vec<(u32, u32)> = if split {
        (0..SEAM_CALLS).map(|i| (i, i + 1)).collect()
    } else {
        vec![(0, SEAM_CALLS)]
    };
    for (from, to) in spans {
        seam(&mut carried);
        let (out, reached) = AzRk4::advance_carrying(
            &state,
            &Geom::new(&state.m, &state.r),
            from,
            to,
            &params,
            &mut drifts,
            &mut carried,
        );
        assert!(
            !reached.ended(),
            "the march ended at call {from}: {reached:?}"
        );
        state = out.state;
    }
    drifts.0
}

/// Asserts the split march reports the uncut march's drifts bit for bit, the greatest magnitude included: the drift is
/// read against the first registration's `E` across calls, never one re-taken at a call's start.
fn check_az_reference(seam: fn(&mut Carried<f64>)) {
    let uncut = az_drifts(false, |_| {});
    let split = az_drifts(true, seam);
    let max = |d: &[u64]| {
        d.iter()
            .map(|&b| f64::from_bits(b).abs())
            .fold(0.0, f64::max)
    };
    println!(
        "Aarseth–Zare × RK4, test 7, f64: {} drifts uncut, {} split; max |drift| uncut {:.6e}, split {:.6e}",
        uncut.len(),
        split.len(),
        max(&uncut),
        max(&split)
    );
    assert!(
        uncut.len() > 2 * SEAM_CALLS as usize,
        "too few drifts reported to compare"
    );
    assert!(
        max(&split).to_bits() == max(&uncut).to_bits() && split == uncut,
        "the split march's drifts are not the uncut march's: the reference was not carried across calls"
    );
}

#[test]
fn energy_named_az_reference_carried_across_calls() {
    check_az_reference(|_| {});
}

negative_control!(
    energy_named_az_reference_carried_across_calls,
    "a seam that drops the registration before each call re-takes the reference there",
    expected = "the reference was not carried across calls",
    check_az_reference(|c| c.registered = 0)
);
