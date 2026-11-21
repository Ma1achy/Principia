//! The payload's producer (`kernel::driver::producer`; dd_integrator §4, seam 5: "every field of the generation-root
//! ledger written, per its metadata"; debug_tooling_plan §D, §E, §G; TASK-M3-22), on real CPU marches:
//! - `producer_joins_the_tiers`: [`produce`] is the join of the paths that each wrote part of the payload: every field
//!   but `last_symbol` is `integrate_ftle`'s, bit for bit, and `last_symbol` and the word are `integrate_word`'s; a
//!   non-finite IC is `decode_failed`, its shadow the failed one and its word empty; beside an occupant that owns its
//!   time mapping (Heggie × RK4) no shadow is marched and the failed one is stored.
//! - `writers_single_in_source`: no kernel source but a packed field's declared writer's, or a named exception, calls
//!   that field's generated setters.
//! - `writers_name_every_ledger_field`, `writers_are_single`, `writers_exist_in_source`: the ledger walk over
//!   [`WRITERS`]: every stored ledger field, the `SimState`'s, the word's and the `ICDescriptor`'s, names one writer
//!   and only one, every row names a ledger field, and every writer is a function declared where the row says.
//! - `every_field_written_on_real_marches`: on the fixtures' real marches and a decoded survey's, at f32 and f64, every
//!   stored field leaves its zero value on at least one sample: a field nobody writes stays at the zero its struct
//!   starts from.
//! - `every_field_in_its_declared_range`: every stored value of every sample is within its field's declared range or is
//!   its sentinel (sentinel conventions honoured; dd seam 5's "sane"). The `ICDescriptor`'s fields are read from the
//!   decoded survey only: the hand-built fixtures' `(m, r, p)` are no decoder's output, and their descriptors fall
//!   outside the canonical gauge its ranges are stated in.
//! - The scalar views' assertions (debug_tooling_plan §D; REQ-TOOL-036), on the produced payloads read through the
//!   generated read side: `scalar_view_t_end_step_latched` (the completed-step count, latched at termination);
//!   `scalar_view_d_min_is_trajectory_minimum` (`d_min > 0`, the minimum over every state the march took);
//!   `scalar_view_latches_bound_final_drift` (`dE_max ≥ |energy_drift|`, `dLz_max ≥ |Lz_drift|`);
//!   `scalar_view_energy_drift_shape` (bounded under a symplectic stepper, secular under Euler);
//!   `scalar_view_drift_fields_signed` (signed drift fields on diverging scales, both signs read).
//!   `ftle ≈ 0` on the Kepler-embedded orbit and large on Burrau is `benettin.rs`'s `benettin_ftle`, read through the
//!   same generated read side.
//! - `e0_agreement` (debug_tooling_plan §G; REQ-VAL-117, REQ-VAL-027): `|E_0 − (K_0 + V_0)|` on the synthetic fixtures
//!   and a decoded survey, at f32 and f64, within a bound derived below; every non-finite `E_0`, `K_0` or `V_0`, tested
//!   on its bits first, is excluded and counted (R-436); the `ICDescriptor` stores no `E₀`.
//! - `shape_views_terminal_frozen` (debug_tooling_plan §E; REQ-TOOL-037): a sample that latched a terminal stores the
//!   same payload, every word, whatever horizon it is produced at, so the live shape and accumulator views over it are
//!   frozen. The other two shape-view properties are measured elsewhere: `‖n‖ = 1` is a stateless function of the
//!   stored positions, tested over random states by `shape_live.rs` and through the fragment by
//!   `render/tests/debug_views.rs`; `θ̃` with no 2π seams on a real circulating orbit is `readout.rs`'s
//!   `theta_unwrap_real_orbit`.
//!
//! Every pass criterion is written here before the runs it reads, with its derivation. The fixtures are test inputs.
//!
//! **Domain** (pitfalls §10): the producer's composition over a handful of real trajectories chosen to reach every
//! field: Burrau's problem, the figure-eight, a tight hierarchy, a head-on collision, a three-way escape, a
//! near-singular passage, Heggie × RK4 under a tight budget and under a raised separation floor, and a non-finite IC.

use std::collections::BTreeMap;
use std::fmt::Debug;

use kernel::chart::invariant::WarpReal;
use kernel::decode::descriptor::{describe, energy};
use kernel::decode::{self, Ic};
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::benettin::{integrate_ftle, s_failed, BenettinParams};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::failure::failed_shadow;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::producer::{produce, Writer, WRITERS};
use kernel::driver::substep::Quantise;
use kernel::driver::wrapper::{
    integrate_detecting, integrate_with, integrate_word, monitored, WrapperReal, WrapperUniforms,
};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::occupant::euler::Euler;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    extract, f16_bits_to_f32, log16_to_f32, pa_d_min, sd_last_symbol, sd_state,
    sim_state_from_ftle, tm_t_end_step, FreeGroupWord, ICDescriptorOf, ReadParams, SimState,
    SimStateBaseOf, SimStateFTLE, SimStateFTLEOf, STATE_BOUNDED, STATE_COLLISION,
    STATE_DECODE_FAILED,
};
use kernel::real::IeeeBits;
use kernel::regularisation::az::AzReal;
use kernel::regularisation::heggie::{Heggie, HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::Composed;
use ledger::schema::{Bound, Entry, FieldType, Location, Provenance, Range, Scale};
use validation::drift::{self, sim_key, AsR};
use validation::measure::benettin as orbits;
use validation::negative_control;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the producer runs at.
trait P: WrapperReal + LinkReal + IeeeBits + AzReal + Quantise + WarpReal + Into<f64> + Debug where
    f64: AsR<Self>,
{
    const LABEL: &'static str;
    /// The unit roundoff, `2^−24` or `2^−53`.
    const U: f64;
}

impl P for f32 {
    const LABEL: &'static str = "f32";
    const U: f64 = f32::EPSILON as f64 / 2.0;
}

impl P for f64 {
    const LABEL: &'static str = "f64";
    const U: f64 = f64::EPSILON / 2.0;
}

/// The f32 unit roundoff, the read side's and the stored payload's.
const U32: f64 = f32::EPSILON as f64 / 2.0;

// ── Fixtures ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Which occupant a fixture marches under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Occupant {
    Yoshida4,
    Euler,
    /// Heggie × RK4 under a budget of [`TIGHT_STEPS`].
    HeggieTight,
    /// Heggie × RK4 with its separation floor `TINY` raised to [`FLOOR_TINY`], so the floor binds, under the budget of
    /// [`TIGHT_STEPS`].
    HeggieFloored,
}

/// A fixture: its IC at f64 (non-finite for the decode-failure fixture), its horizon, its collision threshold and its
/// occupant.
#[derive(Clone, Copy, Debug)]
struct Fixture {
    name: &'static str,
    ic: Ic<f64>,
    horizon: f64,
    r_coll: f64,
    occupant: Occupant,
}

/// The state of masses `m`, positions `r` and velocities `v`, moved to its centre of mass and momentum (as
/// `terminal.rs` builds its fixtures).
fn centred(m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> Ic<f64> {
    let total: f64 = m.iter().sum();
    let mean = |x: &[[f64; 2]; 3], c: usize| (0..3).map(|b| m[b] * x[b][c]).sum::<f64>() / total;
    let (rc, vc) = ([mean(&r, 0), mean(&r, 1)], [mean(&v, 0), mean(&v, 1)]);
    Ic {
        m,
        r: core::array::from_fn(|b| [r[b][0] - rc[0], r[b][1] - rc[1]]),
        p: core::array::from_fn(|b| [m[b] * (v[b][0] - vc[0]), m[b] * (v[b][1] - vc[1])]),
    }
}

/// A binary of unit masses and separation `a` on a circular orbit, with a third unit mass at distance `d` moving at
/// `third`.
fn binary_third(a: f64, d: f64, third: [f64; 2]) -> Ic<f64> {
    let vb = (2.0 / a).sqrt();
    centred(
        [1.0; 3],
        [[-a / 2.0, 0.0], [a / 2.0, 0.0], [d, 0.0]],
        [[0.0, -vb / 2.0], [0.0, vb / 2.0], third],
    )
}

/// `terminal.rs`'s head-on pair: bodies 0 and 1 at rest at `(∓1, 0)`, body 2 at rest at `(0, 8)`; at `r_coll = 0.1`
/// pair 2 collides near `t = 2.2`.
fn head_on() -> Ic<f64> {
    centred(
        [1.0; 3],
        [[-1.0, 0.0], [1.0, 0.0], [0.0, 8.0]],
        [[0.0; 2]; 3],
    )
}

/// `terminal.rs`'s expanding equilateral triangle of side 10, each body moving radially outward at speed 1: a triple
/// ejection at the first window mark.
fn expanding() -> Ic<f64> {
    let rho = 10.0 / 3f64.sqrt();
    let dir = |k: usize| {
        let a = core::f64::consts::FRAC_PI_2 + k as f64 * 2.0 * core::f64::consts::FRAC_PI_3;
        [a.cos(), a.sin()]
    };
    centred(
        [1.0; 3],
        core::array::from_fn(|k| [rho * dir(k)[0], rho * dir(k)[1]]),
        core::array::from_fn(|k| [dir(k)[0], dir(k)[1]]),
    )
}

/// `terminal.rs`'s near-singular pass: the pair falls almost head-on and passes far inside the substep law's cap, so
/// `N_sub` reaches `N_max` and `saturated` is set.
fn near_singular() -> Ic<f64> {
    centred(
        [1.0; 3],
        [[-1.0, 0.002], [1.0, -0.002], [0.0, 6.0]],
        [[0.0; 2]; 3],
    )
}

/// `budget.rs`'s Burrau-like close-encounter state: the Pythagorean masses and positions scaled by 1/5, small momenta.
fn burrau_like() -> Ic<f64> {
    let m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
    let r = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f64; 2]| x.map(|c| c / 5.0));
    let p = [[0.03, -0.01], [-0.02, 0.04], [-0.01, -0.03]];
    Ic { m, r, p }
}

/// A non-finite decode output: `decode_failed`, `detail` 0.
fn non_finite() -> Ic<f64> {
    let mut ic = head_on();
    ic.r[1][0] = f64::NAN;
    ic
}

fn fixture(
    name: &'static str,
    ic: Ic<f64>,
    horizon: f64,
    r_coll: f64,
    occupant: Occupant,
) -> Fixture {
    Fixture {
        name,
        ic,
        horizon,
        r_coll,
        occupant,
    }
}

/// The fixtures, each chosen for the fields it reaches.
fn fixtures() -> Vec<Fixture> {
    use Occupant::*;
    vec![
        // `S`, `r_sh`, `p_sh`, the latches, `dmin_pair`, `t_dmin_step`, `bounded`.
        fixture("Burrau", orbits::burrau().ic, 10.0, 0.0, Yoshida4),
        // The departed bit, `closure_min` and `closure_step`.
        fixture(
            "figure-eight",
            drift::figure_eight().ic,
            13.0,
            0.0,
            Yoshida4,
        ),
        // A word of 73 symbols, past the 96 bits of `x`, `y`, `z`, so `payload` is written; `L_z ≠ 0`.
        fixture(
            "tight hierarchy",
            binary_third(0.2, 3.0, [0.0, 0.8]),
            30.0,
            0.0,
            Yoshida4,
        ),
        // The same to `T = 40`: past `fgw_capacity`, so `length` is the truncation sentinel 127.
        fixture(
            "long hierarchy",
            binary_third(0.2, 3.0, [0.0, 0.8]),
            40.0,
            0.0,
            Yoshida4,
        ),
        // `collision`, its pair in `detail`, `coll_floor`.
        fixture("head-on", head_on(), 5.0, 0.1, Yoshida4),
        // `escape`, `detail` 3.
        fixture("expanding", expanding(), 20.0, 0.0, Yoshida4),
        // `saturated`.
        fixture("near-singular", near_singular(), 4.0, 0.0, Yoshida4),
        // `budget_exhausted`, `cap_hits`, the regularised drift.
        fixture(
            "Burrau-like, tight budget",
            burrau_like(),
            50.0,
            0.0,
            HeggieTight,
        ),
        // `floor_hits`, under the same tight budget, which bounds the run.
        fixture(
            "Burrau-like, raised floor",
            burrau_like(),
            50.0,
            0.0,
            HeggieFloored,
        ),
        // `decode_failed`, the failed-state values and the sentinels.
        fixture("non-finite IC", non_finite(), 5.0, 0.0, Yoshida4),
    ]
}

/// Heggie's test values (`η = 0.01`, `Δ_sync = 1/8`, `f = 0.02`, the landing in force), as `budget.rs` takes them, with
/// the separation floor `tiny` and the budget `rate`, `n_min`. Test inputs.
fn hg_values<R: AzReal>(tiny: (f32, f64), rate: f64, n_min: u32) -> HgParams<R> {
    HgParams {
        eta: R::at_width(0.01, 0.01),
        delta_sync: R::at_width(0.125, 0.125),
        tiny: R::at_width(tiny.0, tiny.1),
        predictive_f: R::at_width(0.02, 0.02),
        step_budget_rate: R::at_width(rate as f32, rate),
        n_min,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        time: DEFAULT_TIME,
    }
}

/// `budget.rs`'s tight budget: 1000 steps at rate 0, fewer than Burrau-like's march to `T = 50` takes.
const TIGHT_STEPS: u32 = 1000;

/// A separation floor far above the reference's `TINY`, at the scale of Burrau-like's close passages, so it binds
/// there. A fixture input, not a threshold. Under a budget that does not bind, the march with it takes steps so small
/// in time that it does not reach `T` in a test's time, so it runs under the tight budget.
const FLOOR_TINY: f64 = 0.01;

struct Tight;

impl HgConfig for Tight {
    fn params<R: AzReal>() -> HgParams<R> {
        hg_values((1e-37, 1e-300), 0.0, TIGHT_STEPS)
    }
}

struct Floored;

impl HgConfig for Floored {
    fn params<R: AzReal>() -> HgParams<R> {
        hg_values((FLOOR_TINY as f32, FLOOR_TINY), 0.0, TIGHT_STEPS)
    }
}

/// The Benettin values every fixture runs at: `‖δ₀‖ = 10⁻⁵`, renormalised every 10 macro-steps. Test inputs.
fn benettin<R: P>() -> BenettinParams<R>
where
    f64: AsR<R>,
{
    BenettinParams {
        delta_0: AsR::<R>::to_r(1e-5),
        n_renorm: 10,
    }
}

/// `ic` at `R`.
fn at<R: P>(ic: &Ic<f64>) -> Ic<R>
where
    f64: AsR<R>,
{
    drift::ic_at::<R>(ic)
}

/// A produced sample: the FTLE tier's `SimState`, its word, its masses, and the decoder's `ICDescriptor` where its IC
/// is a decode's. A hand-built fixture has none: its `(m, r, p)` is no decoder's output, often outside the canonical
/// gauge (`Σm = 1`, `I = 1`) the descriptor's declared ranges are stated in, so the `ICDescriptor` fields are read from
/// the decoded survey's samples, whose writer is the decoder.
#[derive(Clone, Copy, Debug)]
struct Produced<R: P>
where
    f64: AsR<R>,
{
    s: SimStateFTLEOf<R>,
    word: [u32; 4],
    masses: [R; 3],
    descriptor: Option<ICDescriptorOf<R>>,
}

/// The producer of a fixture, given its occupant and horizon key: [`produce`] or, for a control, another path.
type ProducerFn<R> = fn(&Fixture, &WrapperUniforms<R>) -> (SimStateFTLEOf<R>, [u32; 4]);

fn produced_by<A: Advance, R: P>(
    f: &Fixture,
    key: &WrapperUniforms<R>,
) -> (SimStateFTLEOf<R>, [u32; 4])
where
    f64: AsR<R>,
{
    let ic = at::<R>(&f.ic);
    let mut words = [FreeGroupWord::default()];
    let s = produce::<A, R, [FreeGroupWord]>(
        &ic,
        None,
        key,
        CollisionRadius::from_r_coll(f.r_coll),
        benettin::<R>(),
        0,
        &mut words[..],
    );
    (s, words[0].free_group_word)
}

/// The producer under the fixture's occupant.
fn producer<R: P>(f: &Fixture, key: &WrapperUniforms<R>) -> (SimStateFTLEOf<R>, [u32; 4])
where
    f64: AsR<R>,
{
    match f.occupant {
        Occupant::Yoshida4 => produced_by::<Yoshida4, R>(f, key),
        Occupant::Euler => produced_by::<Euler, R>(f, key),
        Occupant::HeggieTight => produced_by::<Composed<Rk4, Heggie<Tight>>, R>(f, key),
        Occupant::HeggieFloored => produced_by::<Composed<Rk4, Heggie<Floored>>, R>(f, key),
    }
}

/// The FTLE tier as it stood before the producer, `integrate_ftle` alone: no word, so `last_symbol` is never mirrored
/// and the word is never written. The walk's control.
fn ftle_only<R: P>(f: &Fixture, key: &WrapperUniforms<R>) -> (SimStateFTLEOf<R>, [u32; 4])
where
    f64: AsR<R>,
{
    let ic = at::<R>(&f.ic);
    let radius = CollisionRadius::from_r_coll(f.r_coll);
    let s = match f.occupant {
        Occupant::Yoshida4 => integrate_ftle::<Yoshida4, R>(&ic, key, radius, benettin::<R>()),
        Occupant::Euler => integrate_ftle::<Euler, R>(&ic, key, radius, benettin::<R>()),
        Occupant::HeggieTight => {
            integrate_ftle::<Composed<Rk4, Heggie<Tight>>, R>(&ic, key, radius, benettin::<R>())
        }
        Occupant::HeggieFloored => {
            integrate_ftle::<Composed<Rk4, Heggie<Floored>>, R>(&ic, key, radius, benettin::<R>())
        }
    };
    (s, [0; 4])
}

/// The base tier's march of `f` under its occupant, through the wrapper's own cadence, monitored: what the producer
/// stores beside an occupant that owns its time mapping, with the failed shadow.
fn detecting<R: P>(f: &Fixture, key: &WrapperUniforms<R>) -> SimStateBaseOf<R>
where
    f64: AsR<R>,
{
    let ic = at::<R>(&f.ic);
    let radius = CollisionRadius::from_r_coll(f.r_coll);
    match f.occupant {
        Occupant::Yoshida4 => integrate_detecting::<Yoshida4, R>(&ic, key, radius),
        Occupant::Euler => integrate_detecting::<Euler, R>(&ic, key, radius),
        Occupant::HeggieTight => {
            integrate_detecting::<Composed<Rk4, Heggie<Tight>>, R>(&ic, key, radius)
        }
        Occupant::HeggieFloored => {
            integrate_detecting::<Composed<Rk4, Heggie<Floored>>, R>(&ic, key, radius)
        }
    }
}

/// Every fixture produced at `R` by `by` at its own horizon's sim key.
fn produce_all<R: P>(by: ProducerFn<R>) -> Vec<(Fixture, Produced<R>)>
where
    f64: AsR<R>,
{
    fixtures()
        .into_iter()
        .map(|f| {
            let (s, word) = by(&f, &sim_key::<R>(f.horizon));
            let masses = at::<R>(&f.ic).m;
            let descriptor = None;
            (
                f,
                Produced {
                    s,
                    word,
                    masses,
                    descriptor,
                },
            )
        })
        .collect()
}

/// The decoded survey's samples ([`survey`]) produced at `R` by `by` to `T = 0.05`, fifty macro-steps, each with the
/// decoder's `ICDescriptor`: the ledger walk's samples of the `ICDescriptor` fields, and more of every other.
fn produce_survey<R: P>(by: ProducerFn<R>) -> Vec<(Fixture, Produced<R>)>
where
    f64: AsR<R>,
{
    survey::<R>()
        .into_iter()
        .map(|(ic, d, degenerate)| {
            assert!(
                degenerate.is_none(),
                "the survey decodes no DEGENERATE sample"
            );
            let f = fixture("survey", ic_f64(&ic), 0.05, 0.0, Occupant::Yoshida4);
            let (s, word) = by(&f, &sim_key::<R>(f.horizon));
            (
                f,
                Produced {
                    s,
                    word,
                    masses: ic.m,
                    descriptor: Some(d),
                },
            )
        })
        .collect()
}

/// `ic` at f64, each value exactly.
fn ic_f64<R: P>(ic: &Ic<R>) -> Ic<f64>
where
    f64: AsR<R>,
{
    Ic {
        m: ic.m.map(Into::into),
        r: ic.r.map(|v| v.map(Into::into)),
        p: ic.p.map(|v| v.map(Into::into)),
    }
}

/// The ledger walk's samples at `R`: every fixture's and the decoded survey's, produced by `by`.
fn walk_samples<R: P>(by: ProducerFn<R>) -> Vec<(Fixture, Produced<R>)>
where
    f64: AsR<R>,
{
    let mut all = produce_all::<R>(by);
    all.extend(produce_survey::<R>(by));
    all
}

// ── producer_joins_the_tiers ────────────────────────────────────────────────────────────────────────────────────

/// `s`'s words, its reals by their bits, every field but `last_symbol` (masked out of `packed_a`) when `mask` is set.
fn words<R: P>(s: &SimStateFTLEOf<R>, mask: bool) -> Vec<u64>
where
    f64: AsR<R>,
{
    let bits = |x: R| Into::<f64>::into(x).to_bits();
    let mut out: Vec<u64> = [s.r, s.p, s.r_sh, s.p_sh]
        .iter()
        .flat_map(|v| v.iter().flatten().map(|&x| bits(x)))
        .collect();
    out.extend(
        [
            s.S,
            s.theta,
            s.mean_y,
            s.C_ty,
            s.E_0,
            s.Lz_0,
            s.closure_min,
            s._nhat_lag_0,
            s._nhat_lag_1,
            s._nhat_lag_2,
        ]
        .map(bits),
    );
    let last_symbol_bits = 0b11 << 8;
    let packed_a = if mask {
        s.packed_a & !last_symbol_bits
    } else {
        s.packed_a
    };
    out.extend(
        [
            packed_a,
            s.packed_b,
            s.times,
            s.total_substeps,
            s.budget_bits,
            s.dE_reg,
            u32::from(s.closure_step),
            u32::from(s._reserved),
            s.coll_floor_bits,
        ]
        .map(u64::from),
    );
    out
}

/// The word path's own run of `f`: `integrate_word` under the wrapper's cadence, its `SimState` and word.
fn word_path<A: Advance, R: P>(f: &Fixture, key: &WrapperUniforms<R>) -> (u32, [u32; 4])
where
    f64: AsR<R>,
{
    let ic = at::<R>(&f.ic);
    let mut cadence =
        WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, CollisionRadius::from_r_coll(f.r_coll));
    let mut words = [FreeGroupWord::default()];
    let out = integrate_word::<A, R, _, [FreeGroupWord]>(&ic, key, &mut cadence, 0, &mut words[..]);
    (out.packed_a, words[0].free_group_word)
}

/// Checks the producer against the two paths it joins, every fixture at `R`; `mask` false compares `last_symbol` with
/// `integrate_ftle`'s too, which never writes it (the control).
fn check_join<R: P>(mask: bool)
where
    f64: AsR<R>,
{
    let mut symbols = 0;
    for f in fixtures() {
        let key = sim_key::<R>(f.horizon);
        let (s, word) = producer::<R>(&f, &key);
        if f.ic.r[1][0].is_nan() {
            assert_eq!(
                sd_state(s.packed_a),
                STATE_DECODE_FAILED,
                "{}: not decode_failed",
                f.name
            );
            assert_eq!(
                word, [0; 4],
                "{}: a decode failure's word is not empty",
                f.name
            );
            continue;
        }
        let (ftle, _) = if matches!(f.occupant, Occupant::HeggieTight | Occupant::HeggieFloored) {
            // No shadow is marched beside an occupant that owns its time mapping: the failed one is stored.
            assert!(
                s.S.non_finite_bits()
                    || Into::<f64>::into(s.S) == Into::<f64>::into(s_failed::<R>()),
                "{} at {}: a shadow was stored beside an occupant that owns its time mapping",
                f.name,
                R::LABEL
            );
            (failed_shadow(detecting::<R>(&f, &key)), [0; 4])
        } else {
            ftle_only::<R>(&f, &key)
        };
        assert!(
            words(&s, mask) == words(&ftle, mask),
            "{} at {}: the produced SimState differs from integrate_ftle's",
            f.name,
            R::LABEL
        );
        if f.occupant == Occupant::Yoshida4 {
            let (packed_a, w) = word_path::<Yoshida4, R>(&f, &key);
            assert_eq!(
                sd_last_symbol(s.packed_a),
                sd_last_symbol(packed_a),
                "{} at {}: last_symbol is not the word path's",
                f.name,
                R::LABEL
            );
            assert_eq!(
                word,
                w,
                "{} at {}: the word is not the word path's",
                f.name,
                R::LABEL
            );
        }
        symbols += u32::from(sd_last_symbol(s.packed_a) != 0);
    }
    println!(
        "producer_joins_the_tiers at {}: {symbols} fixtures with last_symbol ≠ 0",
        R::LABEL
    );
}

#[test]
fn producer_joins_the_tiers() {
    check_join::<f32>(true);
    check_join::<f64>(true);
}

negative_control!(
    producer_joins_the_tiers,
    "`last_symbol` compared with integrate_ftle's, which never mirrors the word",
    expected = "differs from integrate_ftle's",
    check_join::<f64>(false)
);

// ── The ledger walk: the writers table ──────────────────────────────────────────────────────────────────────────

/// The payload ledger's entries.
fn entries() -> Vec<Entry> {
    ledger::payload::ledger()
        .entries
        .iter()
        .map(|b| b.build().expect("the ledger's entries are complete"))
        .collect()
}

/// The ledger's stored fields: every entry that is not derived.
fn stored() -> Vec<Entry> {
    entries()
        .into_iter()
        .filter(|e| !matches!(e.location, Location::Derived { .. }))
        .collect()
}

/// Every stored ledger field has a writer in `writers`, and every row names a stored field.
fn check_named(writers: &[Writer]) {
    let fields: Vec<&str> = stored().iter().map(|e| e.name).collect();
    for name in &fields {
        assert!(
            writers.iter().any(|w| w.field == *name),
            "ledger field `{name}`: nobody writes it (no row in WRITERS)"
        );
    }
    for w in writers {
        assert!(
            fields.contains(&w.field),
            "WRITERS row `{}` names no stored ledger field",
            w.field
        );
    }
    // A derived field is computed at read from stored fields; each it reads is one of the ledger's stored fields, so
    // has a writer.
    for e in entries() {
        if let Location::Derived { from } = &e.location {
            for f in from {
                assert!(
                    fields.contains(f),
                    "derived `{}` reads `{f}`, which no stored field is",
                    e.name
                );
            }
        }
    }
    println!("{} stored ledger fields, each with a writer", fields.len());
}

#[test]
fn writers_name_every_ledger_field() {
    check_named(WRITERS);
}

negative_control!(
    writers_name_every_ledger_field,
    "the table without `last_symbol`'s row",
    expected = "nobody writes it",
    check_named(
        &WRITERS
            .iter()
            .copied()
            .filter(|w| w.field != "last_symbol")
            .collect::<Vec<_>>()
    )
);

/// No field has two rows.
fn check_single(writers: &[Writer]) {
    let mut seen: BTreeMap<&str, Writer> = BTreeMap::new();
    for w in writers {
        if let Some(first) = seen.insert(w.field, *w) {
            panic!(
                "ledger field `{}` has two writers: `{}::{}` and `{}::{}`",
                w.field, first.file, first.function, w.file, w.function
            );
        }
    }
}

#[test]
fn writers_are_single() {
    check_single(WRITERS);
}

negative_control!(
    writers_are_single,
    "`state` written by the failure path too, as a second row",
    expected = "has two writers",
    check_single(
        &WRITERS
            .iter()
            .copied()
            .chain([Writer {
                field: "state",
                file: "driver/failure.rs",
                function: "sim_failed",
            }])
            .collect::<Vec<_>>()
    )
);

/// Every row's writer is declared in its file, `fn <function>` followed by `(` or `<`.
fn check_sources(writers: &[Writer]) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../kernel/src");
    for w in writers {
        let text = std::fs::read_to_string(root.join(w.file))
            .unwrap_or_else(|e| panic!("`{}`'s writer file {}: {e}", w.field, w.file));
        let declared = [format!("fn {}(", w.function), format!("fn {}<", w.function)]
            .iter()
            .any(|d| text.contains(d.as_str()));
        assert!(
            declared,
            "`{}`'s writer: no `fn {}` in crates/kernel/src/{}",
            w.field, w.function, w.file
        );
    }
}

#[test]
fn writers_exist_in_source() {
    check_sources(WRITERS);
}

negative_control!(
    writers_exist_in_source,
    "a row naming a writer that does not exist",
    expected = "no `fn",
    check_sources(&[Writer {
        field: "theta",
        file: "driver/shape.rs",
        function: "write_theta",
    }])
);

/// A file other than a field's declared writer's that may call its generated setter, and why: the failure path, the
/// march's initial word, the stages a declared writer composes, and the bring-up, toolchain and parity patterns, which
/// write no sample. `"*"` is every field.
const EXCEPTIONS: &[(&str, &str, &str)] = &[
    ("*", "driver/failure.rs", "the one writer of every failed-state value"),
    ("*", "bringup.rs", "the bring-up mode's known pattern: no march"),
    ("*", "toolchain.rs", "the toolchain probe's round trip: no march"),
    ("*", "payload/roundtrip.rs", "the descriptor parity check: no march"),
    ("saturated", "driver/substep.rs", "the substep law's `N_max` bit, which the declared writer ORs"),
    ("d_min", "driver/wrapper.rs", "the march's initial word, `d_min` unset"),
    ("last_symbol", "word/append.rs", "the crossing's carried descriptor, which the declared writer mirrors"),
    ("payload", "driver/producer.rs", "the empty word a sample starts from"),
    ("length", "driver/producer.rs", "the empty word a sample starts from"),
    (
        "dmin_pair",
        "detect/collision.rs",
        "`Detector::label`, a second writer that no producer path calls (reported on TASK-M3-22's PR)",
    ),
];

/// The generated setters of the packed field `e`: `set_<field>` and its variants, the whole word's `pack_<word>`, and
/// the word buffer's `fgw_pack` for `fgw_w`'s fields.
fn setters(e: &Entry) -> Vec<String> {
    let Location::Packed { word, .. } = &e.location else {
        return Vec::new();
    };
    let mut out = vec![
        format!("set_{}", e.name),
        format!("set_{}_unset", e.name),
        format!("set_{}_release", e.name),
        format!("pack_{word}"),
    ];
    if *word == "fgw_w" {
        out.push("fgw_pack".to_owned());
    }
    out
}

/// Every `.rs` file under `dir`, by its path relative to `root`.
fn sources(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("a kernel source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            sources(root, &path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            let rel = path
                .strip_prefix(root)
                .expect("under the root")
                .to_string_lossy()
                .replace('\\', "/");
            out.push((
                rel,
                std::fs::read_to_string(&path).expect("a readable source"),
            ));
        }
    }
}

/// Whether `text` calls `name`: `name(` not preceded by an identifier character, on a line that is neither a comment
/// nor `name`'s own declaration.
fn calls(text: &str, name: &str) -> bool {
    let needle = format!("{name}(");
    text.lines().any(|line| {
        let code = line.trim_start();
        if code.starts_with("//") || code.contains(&format!("fn {name}")) {
            return false;
        }
        code.match_indices(&needle).any(|(i, _)| {
            !code[..i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
    })
}

/// No file but a packed field's declared writer's, or one of `exceptions`, calls that field's generated setters: a
/// second writer added in the kernel's sources fails here, not only one added to the table. The generated files, which
/// define the setters, are not read.
fn check_call_sites(exceptions: &[(&str, &str, &str)]) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../kernel/src");
    let mut files = Vec::new();
    sources(&root, &root, &mut files);
    files.retain(|(rel, _)| !rel.starts_with("payload/generated"));
    let mut sites = 0;
    for e in stored() {
        let names = setters(&e);
        if names.is_empty() {
            continue;
        }
        let writer = WRITERS
            .iter()
            .find(|w| w.field == e.name)
            .expect("a writer");
        for (rel, text) in &files {
            if !names.iter().any(|n| calls(text, n)) {
                continue;
            }
            sites += 1;
            let allowed = rel == writer.file
                || exceptions
                    .iter()
                    .any(|(f, file, _)| (*f == "*" || *f == e.name) && rel == file);
            assert!(
                allowed,
                "`{}` is written in crates/kernel/src/{rel}, which is neither its writer's file ({}) nor an exception: \
                 a second writer",
                e.name, writer.file
            );
        }
    }
    println!("{sites} setter call sites, each its field's writer's file or a named exception");
}

#[test]
fn writers_single_in_source() {
    check_call_sites(EXCEPTIONS);
}

negative_control!(
    writers_single_in_source,
    "the exceptions without the substep law's write of `saturated`",
    expected = "a second writer",
    check_call_sites(
        &EXCEPTIONS
            .iter()
            .copied()
            .filter(|(_, file, _)| *file != "driver/substep.rs")
            .collect::<Vec<_>>()
    )
);

// ── The ledger walk: each field read by its metadata ────────────────────────────────────────────────────────────

/// The stored word `name` of `s` (or the word buffer's `.w`, `fgw_w`).
fn packed_word<R: P>(s: &SimStateFTLEOf<R>, word: [u32; 4], name: &str) -> u32
where
    f64: AsR<R>,
{
    match name {
        "packed_a" => s.packed_a,
        "packed_b" => s.packed_b,
        "times" => s.times,
        "budget_bits" => s.budget_bits,
        "dE_reg" => s.dE_reg,
        "coll_floor_bits" => s.coll_floor_bits,
        "fgw_w" => word[3],
        _ => panic!("no reader for the ledger's word `{name}`: the walk does not know it"),
    }
}

/// The stored scalar field `name` of `s`, each component, by the struct member the ledger names.
fn scalar<R: P>(s: &SimStateFTLEOf<R>, name: &str) -> Vec<f64>
where
    f64: AsR<R>,
{
    let v = |x: R| -> f64 { x.into() };
    let vec6 = |a: [[R; 2]; 3]| a.iter().flatten().map(|&x| v(x)).collect();
    match name {
        "r" => vec6(s.r),
        "p" => vec6(s.p),
        "r_sh" => vec6(s.r_sh),
        "p_sh" => vec6(s.p_sh),
        "S" => vec![v(s.S)],
        "theta" => vec![v(s.theta)],
        "mean_y" => vec![v(s.mean_y)],
        "C_ty" => vec![v(s.C_ty)],
        "E_0" => vec![v(s.E_0)],
        "Lz_0" => vec![v(s.Lz_0)],
        "total_substeps" => vec![f64::from(s.total_substeps)],
        "closure_min" => vec![v(s.closure_min)],
        "closure_step" => vec![f64::from(s.closure_step)],
        _ => panic!("no reader for the ledger's scalar field `{name}`: the walk does not know it"),
    }
}

/// The `ICDescriptor` field `name` of `d`.
fn ic_field<R: P>(d: &ICDescriptorOf<R>, name: &str) -> f64
where
    f64: AsR<R>,
{
    let x = match name {
        "m0" => d.m0,
        "m1" => d.m1,
        "m2" => d.m2,
        "q_mass" => d.q_mass,
        "rho_mag" => d.rho_mag,
        "lambda_mag" => d.lambda_mag,
        "rho_ratio" => d.rho_ratio,
        "rho_angle" => d.rho_angle,
        "K_0" => d.K_0,
        "V_0" => d.V_0,
        "virial_ratio" => d.virial_ratio,
        "r_min_pair_0" => d.r_min_pair_0,
        _ => panic!(
            "no reader for the ledger's ICDescriptor field `{name}`: the walk does not know it"
        ),
    };
    x.into()
}

/// The stored field `e` of `p`, read by its ledger location and type: `None` for a descriptor field of a sample with no
/// descriptor.
fn field<R: P>(e: &Entry, p: &Produced<R>) -> Option<Vec<f64>>
where
    f64: AsR<R>,
{
    if e.provenance == Provenance::Decode {
        return p.descriptor.as_ref().map(|d| vec![ic_field(d, e.name)]);
    }
    Some(match &e.location {
        Location::Packed {
            word,
            offset,
            width,
        } => {
            let bits = extract(packed_word(&p.s, p.word, word), *offset, *width);
            let value = match e.ty {
                FieldType::UBits => f64::from(bits),
                FieldType::F16Pair => f64::from(f16_bits_to_f32(bits as u16)),
                FieldType::Log16 => f64::from(log16_to_f32(bits as u16)),
                ref other => panic!("`{}`: no decoder for the packed type {other:?}", e.name),
            };
            vec![value]
        }
        Location::Scalar(_) => scalar(&p.s, e.name),
        Location::Derived { .. } => unreachable!("only stored fields are read"),
    })
}

/// `p` with every stored field at the zero its struct starts from: the value of a field nobody writes.
fn zero<R: P>(p: &Produced<R>) -> Produced<R>
where
    f64: AsR<R>,
{
    Produced {
        s: SimStateFTLEOf::default(),
        word: [0; 4],
        masses: p.masses,
        descriptor: p.descriptor.map(|_| ICDescriptorOf::default()),
    }
}

/// Every stored field leaves its zero on at least one of `samples`.
fn check_written<R: P>(samples: &[(Fixture, Produced<R>)])
where
    f64: AsR<R>,
{
    let mut unwritten = Vec::new();
    for e in stored() {
        let mut first: Option<&str> = None;
        for (f, p) in samples {
            let (Some(got), Some(none)) = (field(&e, p), field(&e, &zero(p))) else {
                continue;
            };
            let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            if bits(&got) != bits(&none) {
                first = Some(f.name);
                break;
            }
        }
        match first {
            Some(name) => println!("  {:<18} first written on {name}", e.name),
            None => unwritten.push(e.name),
        }
    }
    assert!(
        unwritten.is_empty(),
        "at {}: no real march writes {unwritten:?}: each stays at its zero on every fixture",
        R::LABEL
    );
}

#[test]
fn every_field_written_on_real_marches() {
    println!("f32:");
    check_written(&walk_samples::<f32>(producer::<f32>));
    println!("f64:");
    check_written(&walk_samples::<f64>(producer::<f64>));
}

negative_control!(
    every_field_written_on_real_marches,
    "the FTLE tier as it stood before the producer: integrate_ftle alone, no word",
    expected = "no real march writes",
    check_written(&walk_samples::<f64>(ftle_only::<f64>))
);

/// Whether `v` is within `range`.
fn within(range: &Range, v: f64) -> bool {
    let lo = match range.lo {
        Bound::Closed(b) => v >= b,
        Bound::Open(b) => v > b,
        Bound::Unbounded => true,
    };
    let hi = match range.hi {
        Bound::Closed(b) => v <= b,
        Bound::Open(b) => v < b,
        Bound::Unbounded => true,
    };
    lo && hi && !v.is_nan()
}

/// Every stored value of `samples` is within its field's declared range or is its sentinel.
fn check_ranges<R: P>(samples: &[(Fixture, Produced<R>)])
where
    f64: AsR<R>,
{
    let mut sentinels = 0;
    for e in stored() {
        for (f, p) in samples {
            let Some(values) = field(&e, p) else { continue };
            for v in values {
                if e.sentinel.is_some_and(|s| s == v) {
                    sentinels += 1;
                    continue;
                }
                assert!(
                    within(&e.range, v),
                    "{} at {}: `{}` = {v:e} is outside its declared range {} and is not its sentinel {:?}",
                    f.name,
                    R::LABEL,
                    e.name,
                    e.range,
                    e.sentinel
                );
            }
        }
    }
    println!(
        "{}: every stored value in range; {sentinels} sentinel values",
        R::LABEL
    );
}

#[test]
fn every_field_in_its_declared_range() {
    check_ranges(&walk_samples::<f32>(producer::<f32>));
    check_ranges(&walk_samples::<f64>(producer::<f64>));
}

negative_control!(
    every_field_in_its_declared_range,
    "a `closure_min` stored negative, below its declared [0, ∞)",
    expected = "outside its declared range",
    {
        let mut samples = walk_samples::<f64>(producer::<f64>);
        samples[0].1.s.closure_min = -1.0;
        check_ranges(&samples);
    }
);

// ── The scalar views (debug_tooling_plan §D; REQ-TOOL-036) ──────────────────────────────────────────────────────

/// The read-side `SimState` of a produced f32 sample, as the views read it.
fn read(p: &Produced<f32>, key: &WrapperUniforms<f32>) -> SimState {
    let s: &SimStateFTLE = &p.s;
    let masses = p.masses;
    sim_state_from_ftle(
        s,
        p.word,
        true,
        0.0,
        false,
        masses,
        &ReadParams {
            dt_macro: key.dt_macro,
            delta_0: 1e-5,
            n_renorm: 10,
            horizon_steps: key.horizon_steps,
        },
    )
}

/// `key` with the horizon `steps`.
fn with_steps<R: Copy>(key: &WrapperUniforms<R>, steps: u32) -> WrapperUniforms<R> {
    WrapperUniforms {
        horizon_steps: steps,
        ..*key
    }
}

/// `t_end_step` is the completed-step count, latched at termination: the head-on sample collides with `t_end_step = k`;
/// produced to the horizon `k − 1` it reaches the horizon with no event, `bounded` at `k − 1`; produced to `k`, `k + 1`
/// and twice its horizon it is the same collision at `k`, every word. (The collision fires on a state the march reaches
/// in its `k`-th macro-step, the step's last under `N_sub = 1`, so the run to the horizon `k` reaches it too.) `shift`
/// is added to the stored count before the check (the control's broken latch).
fn check_t_end_step(shift: u32) {
    let f = fixtures()
        .into_iter()
        .find(|f| f.name == "head-on")
        .expect("head-on");
    let key = sim_key::<f32>(f.horizon);
    let latch = |s: SimStateFTLE| SimStateFTLE {
        times: s.times + shift,
        ..s
    };
    let (s, _) = producer::<f32>(&f, &key);
    let s = latch(s);
    assert_eq!(
        sd_state(s.packed_a),
        STATE_COLLISION,
        "head-on did not collide"
    );
    let k = tm_t_end_step(s.times);
    assert!(k > 1 && k < key.horizon_steps, "head-on collided at {k}");
    let (before, _) = producer::<f32>(&f, &with_steps(&key, k - 1));
    let before = latch(before);
    assert_eq!(
        (sd_state(before.packed_a), tm_t_end_step(before.times)),
        (STATE_BOUNDED, k - 1),
        "to the horizon {}, the run is not bounded there: t_end_step is not the completed-step count",
        k - 1
    );
    for steps in [k, k + 1, 2 * key.horizon_steps] {
        let (later, _) = producer::<f32>(&f, &with_steps(&key, steps));
        assert_eq!(
            words(&latch(later), false),
            words(&s, false),
            "to the horizon {steps}, the collision at {k} is stored differently: t_end_step is not latched"
        );
    }
    println!(
        "head-on: collision at t_end_step = {k} of {}",
        key.horizon_steps
    );
}

#[test]
fn scalar_view_t_end_step_latched() {
    check_t_end_step(0);
}

negative_control!(
    scalar_view_t_end_step_latched,
    "a count that includes the terminal's own, uncompleted, macro-step",
    expected = "is not the completed-step count",
    check_t_end_step(1)
);

/// A cadence that records the least pair separation over every state the march hands it, the IC's included, and the
/// largest coordinate magnitude, around the wrapper's cadence, every call passed on.
struct Tap<'a, R: P>
where
    f64: AsR<R>,
{
    inner: &'a mut WrapperCadence<R>,
    d_min: f64,
    coord_max: f64,
}

impl<'a, R: P> Tap<'a, R>
where
    f64: AsR<R>,
{
    fn new(inner: &'a mut WrapperCadence<R>, r: &[[R; 2]; 3]) -> Self {
        let mut tap = Tap {
            inner,
            d_min: f64::INFINITY,
            coord_max: 0.0,
        };
        tap.see(r);
        tap
    }

    fn see(&mut self, r: &[[R; 2]; 3]) {
        let r: [[f64; 2]; 3] = r.map(|v| v.map(Into::into));
        for (i, j) in [(1, 2), (2, 0), (0, 1)] {
            let d = (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]);
            self.d_min = self.d_min.min(d);
        }
        for c in r.iter().flatten() {
            self.coord_max = self.coord_max.max(c.abs());
        }
    }
}

impl<R: P> Cadence<R> for Tap<'_, R>
where
    f64: AsR<R>,
{
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
        if !self.inner.failure().fired {
            self.see(&state.r);
        }
    }
    fn done(&self) -> bool {
        self.inner.done()
    }
    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        self.inner.sample(state, index, quantum);
    }
    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }
    fn regularised_drift(&mut self, drift: R) {
        self.inner.regularised_drift(drift);
    }
    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }
    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }
    fn latched(&mut self) {
        self.inner.latched();
    }
    fn non_finite(&mut self, detail: u32) {
        self.inner.non_finite(detail);
    }
    fn failure(&self) -> kernel::detect::label::Fired {
        self.inner.failure()
    }
}

/// `d_min > 0` and is the least separation over every state the march took, the IC's included, on the stepper fixtures
/// at `R`, to the stored form's resolution. The criterion: `|d_min − d_ref| ≤ 2⁻¹⁰·d_ref + 2⁻²¹·c_max`, where `d_ref`
/// is the recorded least separation (in f64 of the march's own positions) and `c_max` the largest coordinate magnitude
/// over the march. Derivation: the f16 latch rounds to nearest, relative 2⁻¹¹; the latched `d²` is the quantised one,
/// the positions rounded to f32 (`Quantise`) and `d²` formed in f32, which moves `d` by at most about `2·2⁻²⁴·√2·c_max`
/// from the rounded coordinates plus `2·2⁻²⁴·d` from the three roundings of `d²`; both doubled. `scale` multiplies the
/// stored value first (the control's broken latch).
fn check_d_min<R: P>(scale: f32)
where
    f64: AsR<R>,
{
    for f in fixtures() {
        if f.occupant != Occupant::Yoshida4 || f.ic.r[1][0].is_nan() {
            continue;
        }
        let key = sim_key::<R>(f.horizon);
        let ic = at::<R>(&f.ic);
        let (s, _) = producer::<R>(&f, &key);
        let mut cadence = WrapperCadence::with_radius(
            &ic.m,
            &ic.r,
            &ic.p,
            CollisionRadius::from_r_coll(f.r_coll),
        );
        let (tapped, d_ref, c_max) = {
            let mut tap = Tap::new(&mut cadence, &ic.r);
            let out = integrate_with::<Yoshida4, R, _>(&ic, &key, &mut tap);
            (out, tap.d_min, tap.coord_max)
        };
        let tapped = monitored(tapped, &cadence);
        assert_eq!(
            tapped.packed_a & !(0b11 << 8),
            s.packed_a & !(0b11 << 8),
            "{}: the tapped march is not the producer's",
            f.name
        );
        let d_min = f64::from(pa_d_min(s.packed_a) * scale);
        let tol = d_ref * 2f64.powi(-10) + c_max * 2f64.powi(-21);
        println!(
            "{:<16} {}: d_min {d_min:.6e}, trajectory minimum {d_ref:.6e}, |Δ|/tol {:.3}",
            f.name,
            R::LABEL,
            (d_min - d_ref).abs() / tol
        );
        assert!(
            d_min > 0.0,
            "{} at {}: d_min = {d_min} is not > 0",
            f.name,
            R::LABEL
        );
        assert!(
            (d_min - d_ref).abs() <= tol,
            "{} at {}: d_min {d_min:e} is not the trajectory minimum {d_ref:e} (tolerance {tol:e})",
            f.name,
            R::LABEL
        );
    }
}

#[test]
fn scalar_view_d_min_is_trajectory_minimum() {
    check_d_min::<f32>(1.0);
    check_d_min::<f64>(1.0);
}

negative_control!(
    scalar_view_d_min_is_trajectory_minimum,
    "a latch 1% above the minimum, as a latch of the macro-step states alone would read",
    expected = "is not the trajectory minimum",
    check_d_min::<f64>(1.01)
);

/// `K` and `|V|` of `(m, r, p)` in f64, and `Σ (|x p_y| + |y p_x|)`.
fn magnitudes(m: [f64; 3], r: [[f64; 2]; 3], p: [[f64; 2]; 3]) -> (f64, f64, f64) {
    let k: f64 = (0..3)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
        .sum();
    let v: f64 = [(1, 2), (2, 0), (0, 1)]
        .iter()
        .map(|&(i, j)| m[i] * m[j] / (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]))
        .sum();
    let l: f64 = (0..3)
        .map(|i| (r[i][0] * p[i][1]).abs() + (r[i][1] * p[i][0]).abs())
        .sum();
    (k, v, l)
}

/// The latches bound the final drift the views read: `|energy_drift| ≤ dE_max·(1 + 2⁻⁸) + 24u(K + |V|) + 2u|E_0|` and
/// `|Lz_drift| ≤ dLz_max·(1 + 2⁻⁸) + 16u·Σ(|x p_y| + |y p_x|) + 2u|Lz_0|`, `u = 2⁻²⁴`, `K`, `V` at the stored final
/// state, on the stepper fixtures produced at f32 and read through the read side. Derivation: the `log16` form rounds
/// to nearest, relative 2⁻⁹, so the stored latch is at least `max|ΔE|·(1 − 2⁻⁹)`; the monitor's `ΔE` at the final
/// state and the read side's are each `H` formed in f32 with at most about 11 roundings relative to `K + |V|` (each
/// kinetic term 4, each pair term 6, the sums 5), and `E_0` subtracted (2u); `L_z`'s three cross terms 4 each. `shrink`
/// divides the stored latches first (the control's latch that missed the final state).
fn check_latches(shrink: f32) {
    let mut worst = (0.0f64, 0.0f64);
    for (f, p) in produce_all::<f32>(producer::<f32>) {
        if f.occupant != Occupant::Yoshida4 || f.ic.r[1][0].is_nan() {
            continue;
        }
        let key = sim_key::<f32>(f.horizon);
        let v = read(&p, &key);
        let m = p.masses.map(f64::from);
        let r = p.s.r.map(|x| x.map(f64::from));
        let q = p.s.p.map(|x| x.map(f64::from));
        let (k, pot, l) = magnitudes(m, r, q);
        let e_tol = f64::from(v.dE_max / shrink) * (1.0 + 2f64.powi(-8))
            + 24.0 * U32 * (k + pot)
            + 2.0 * U32 * f64::from(v.E_0).abs();
        let l_tol = f64::from(v.dLz_max / shrink) * (1.0 + 2f64.powi(-8))
            + 16.0 * U32 * l
            + 2.0 * U32 * f64::from(v.Lz_0).abs();
        let (de, dl) = (f64::from(v.energy_drift).abs(), f64::from(v.Lz_drift).abs());
        println!(
            "{:<16} |energy_drift| {de:.3e} ≤ {e_tol:.3e} (dE_max {:.3e}); |Lz_drift| {dl:.3e} ≤ {l_tol:.3e} (dLz_max {:.3e})",
            f.name, v.dE_max, v.dLz_max
        );
        worst = (worst.0.max(de / e_tol), worst.1.max(dl / l_tol));
        assert!(
            de <= e_tol,
            "{}: |energy_drift| {de:e} exceeds dE_max's bound {e_tol:e}",
            f.name
        );
        assert!(
            dl <= l_tol,
            "{}: |Lz_drift| {dl:e} exceeds dLz_max's bound {l_tol:e}",
            f.name
        );
    }
    println!(
        "worst ratio to the bound: energy {:.3}, L_z {:.3}",
        worst.0, worst.1
    );
}

#[test]
fn scalar_view_latches_bound_final_drift() {
    check_latches(1.0);
}

negative_control!(
    scalar_view_latches_bound_final_drift,
    "latches a thousandth of their value, as a latch that missed the march's largest drift would hold",
    expected = "exceeds",
    check_latches(1000.0)
);

/// The energy drift's shape over the horizon: on the figure-eight at f64, `dt_macro = 10⁻²` with `N_sub` held at 1 (a
/// step coarse enough that Yoshida-4's truncation error is far above round-off), between `T = 4` and `T = 16`. The
/// criteria: a symplectic stepper's drift stays bounded, `dE_max(16) ≤ 2·dE_max(4)`, and is the truncation's, not
/// round-off's, `dE_max(4) ≥ 10³·2⁻⁵³·|E_0|`; Euler's grows secularly, `|ΔE(16)| ≥ 2·|ΔE(4)|`, `ΔE` read from the stored
/// state in f64. A symplectic map's energy error oscillates about a bounded mean, so four times the time cannot double
/// its maximum; Euler's error on a bounded orbit grows linearly in `t` to first order, so four times the time gives
/// about four times the drift, and twice is the margin. `symplectic` and `secular` are the occupants run as each (the
/// control swaps them).
fn check_drift_shape(symplectic: Occupant, secular: Occupant) {
    let ic = drift::figure_eight().ic;
    let config = NsubConfig {
        n_max: 1,
        ..NsubConfig::DEFAULT
    };
    let key = |horizon: f64| WrapperUniforms {
        dt_macro: 0.01,
        horizon_steps: (horizon / 0.01).round() as u32,
        n_max: 1,
        thresholds: thresholds(&config).expect("the key generates"),
    };
    let run = |occupant, horizon: f64| {
        let f = fixture("figure-eight", ic, horizon, 0.0, occupant);
        let (s, _) = producer::<f64>(&f, &key(horizon));
        assert_eq!(
            sd_state(s.packed_a),
            STATE_BOUNDED,
            "{occupant:?} to T = {horizon}: not bounded"
        );
        let (k, v, _) = magnitudes(ic.m, s.r, s.p);
        (
            f64::from(log16_to_f32((s.packed_b & 0xffff) as u16)),
            k - v - s.E_0,
            s.E_0,
        )
    };
    let (sym_4, sym_16) = (run(symplectic, 4.0), run(symplectic, 16.0));
    let (sec_4, sec_16) = (run(secular, 4.0), run(secular, 16.0));
    let floor = 1e3 * f64::EPSILON / 2.0 * sym_4.2.abs();
    println!(
        "{symplectic:?}: dE_max {:.3e} at T = 4 (round-off floor ×10³: {floor:.3e}), {:.3e} at T = 16; {secular:?}: ΔE \
         {:.3e} at T = 4, {:.3e} at T = 16",
        sym_4.0, sym_16.0, sec_4.1, sec_16.1
    );
    assert!(
        sym_4.0 >= floor,
        "{symplectic:?}'s drift {:e} is round-off's, not the truncation's: the step is too fine to read its shape",
        sym_4.0
    );
    assert!(
        sym_16.0 <= 2.0 * sym_4.0,
        "{symplectic:?}'s drift is not bounded: dE_max {:e} at T = 16 against {:e} at T = 4",
        sym_16.0,
        sym_4.0
    );
    assert!(
        sec_16.1.abs() >= 2.0 * sec_4.1.abs(),
        "{secular:?}'s drift is not secular: |ΔE| {:e} at T = 16 against {:e} at T = 4",
        sec_16.1.abs(),
        sec_4.1.abs()
    );
}

#[test]
fn scalar_view_energy_drift_shape() {
    check_drift_shape(Occupant::Yoshida4, Occupant::Euler);
}

negative_control!(
    scalar_view_energy_drift_shape,
    "Euler taken for the symplectic stepper",
    expected = "drift is not bounded",
    check_drift_shape(Occupant::Euler, Occupant::Yoshida4)
);

/// The drift fields are signed and on diverging scales: the ledger gives `energy_drift` and `Lz_drift` the diverging
/// scale over a range that spans zero, and the views read both signs over the fixtures' payloads. `unsigned` takes the
/// magnitude first (the control's unsigned view).
fn check_signed(unsigned: bool) {
    for name in ["energy_drift", "Lz_drift"] {
        let e = entries()
            .into_iter()
            .find(|e| e.name == name)
            .expect("a ledger entry");
        assert_eq!(
            e.scale,
            Scale::Diverging,
            "`{name}` is not on a diverging scale"
        );
        assert!(
            e.range_state().spans_zero(),
            "`{name}`'s range does not span zero"
        );
    }
    let mut signs = [[0u32; 2]; 2];
    for (f, p) in produce_all::<f32>(producer::<f32>) {
        if f.ic.r[1][0].is_nan() {
            continue;
        }
        let v = read(&p, &sim_key::<f32>(f.horizon));
        for (k, d) in [v.energy_drift, v.Lz_drift].into_iter().enumerate() {
            let d = if unsigned { d.abs() } else { d };
            signs[k][usize::from(d > 0.0)] += u32::from(d != 0.0);
        }
    }
    println!(
        "energy_drift: {} negative, {} positive; Lz_drift: {} negative, {} positive",
        signs[0][0], signs[0][1], signs[1][0], signs[1][1]
    );
    for (k, name) in ["energy_drift", "Lz_drift"].iter().enumerate() {
        assert!(
            signs[k][0] > 0,
            "`{name}` is never negative on the fixtures: not a signed view"
        );
        assert!(
            signs[k][1] > 0,
            "`{name}` is never positive on the fixtures: not a signed view"
        );
    }
}

#[test]
fn scalar_view_drift_fields_signed() {
    check_signed(false);
}

negative_control!(
    scalar_view_drift_fields_signed,
    "the drifts' magnitudes, an unsigned view",
    expected = "is never negative",
    check_signed(true)
);

// ── The energy agreement (debug_tooling_plan §G; REQ-VAL-117, REQ-VAL-027) ──────────────────────────────────────

/// The survey's latent slice: `z = z₀ + u·q₁ + v·q₂` over a 24 × 24 grid of `(u, v)` in `[−1, 1]²`, `wrapper.rs`'s
/// `Latent` chart's vectors.
fn survey<R: P>() -> Vec<(Ic<R>, ICDescriptorOf<R>, Option<decode::Degenerate>)>
where
    f64: AsR<R>,
{
    let z0 = [0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1];
    let q1 = [0.5, 0.0, 0.2, 0.0, 0.0, 0.1, 0.0, 0.3];
    let q2 = [0.0, 0.5, 0.0, -0.2, 0.1, 0.0, 0.2, 0.0];
    let n = 24;
    let mut out = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let (u, v) = (
                -1.0 + 2.0 * i as f64 / (n - 1) as f64,
                -1.0 + 2.0 * j as f64 / (n - 1) as f64,
            );
            let z: [R; 8] = core::array::from_fn(|k| (z0[k] + u * q1[k] + v * q2[k]).to_r());
            let d = decode::decode::<Defaults, R>(&decode::Latent::new(z));
            out.push((d.ic, d.descriptor, d.degenerate));
        }
    }
    out
}

/// The stored `E_0` of `ic` at `R`: the producer at a one-step horizon (`E_0` is written before the loop).
fn stored_e0<R: P>(ic: &Ic<R>, degenerate: Option<decode::Degenerate>) -> (u32, R)
where
    f64: AsR<R>,
{
    let key = with_steps(&sim_key::<R>(1.0), 1);
    let mut words = [FreeGroupWord::default()];
    let s = produce::<Yoshida4, R, [FreeGroupWord]>(
        ic,
        degenerate,
        &key,
        CollisionRadius::none(),
        benettin::<R>(),
        0,
        &mut words[..],
    );
    (sd_state(s.packed_a), s.E_0)
}

/// `|E_0 − (K_0 + V_0)| ≤ 24u(K_0 + |V_0|)` on every sample of the synthetic fixtures and the decoded survey at `R`,
/// `u` the precision's unit roundoff, with every sample whose `E_0`, `K_0` or `V_0` is non-finite on its bits, and
/// every `decode_failed` one, excluded and counted (R-436). Derivation: `E_0` is the monitor's `H` of the march's IC
/// and `K_0 + V_0` the descriptor's, the same sum of the same three kinetic and three pair terms in `R`, each formed
/// with at most about 11 roundings relative to `K + |V|` (each kinetic term 4, each pair term 6, the sums 5): their
/// difference is at most twice that, 22u, taken as 24u. `kick` scales the IC's momenta for the stored `E_0` only (the
/// control: an `E_0` of a state other than the descriptor's). Returns the worst `|ΔE|/(K_0 + |V_0|)` in units of `u`.
fn check_e0<R: P>(kick: f64) -> f64
where
    f64: AsR<R>,
{
    let mut synthetic: Vec<(Ic<R>, ICDescriptorOf<R>, Option<decode::Degenerate>)> = fixtures()
        .iter()
        .map(|f| {
            let ic = at::<R>(&f.ic);
            (ic, describe(&ic), None)
        })
        .collect();
    synthetic.retain(|(ic, _, _)| ic.r.iter().flatten().all(|x| !x.non_finite_bits()));
    let mut worst = 0.0f64;
    for (label, samples) in [("synthetic", synthetic), ("survey", survey::<R>())] {
        let (mut checked, mut excluded, mut failed) = (0, 0, 0);
        let mut worst_here = 0.0f64;
        for (ic, d, degenerate) in samples {
            let kicked = Ic {
                p: ic.p.map(|v| v.map(|x| x * kick.to_r())),
                ..ic
            };
            let (state, e_0) = stored_e0(&kicked, degenerate);
            if state == STATE_DECODE_FAILED {
                failed += 1;
                continue;
            }
            if e_0.non_finite_bits() || d.K_0.non_finite_bits() || d.V_0.non_finite_bits() {
                excluded += 1;
                continue;
            }
            let (e_0, k_0, v_0): (f64, f64, f64) = (e_0.into(), d.K_0.into(), d.V_0.into());
            let sum: f64 = energy(&d).into();
            let err = (e_0 - sum).abs() / ((k_0 + v_0.abs()) * R::U);
            worst_here = worst_here.max(err);
            checked += 1;
            assert!(
                err <= 24.0,
                "{label} at {}: |E_0 − (K_0 + V_0)| is {err:.1}u of K_0 + |V_0|, above 24u: E_0 {e_0:e}, K_0 + V_0 {sum:e}",
                R::LABEL
            );
        }
        println!(
            "e0_agreement {label} at {}: {checked} checked, worst {worst_here:.2}u; {excluded} non-finite excluded, {failed} decode_failed",
            R::LABEL
        );
        worst = worst.max(worst_here);
    }
    worst
}

#[test]
fn e0_agreement() {
    // REQ-VAL-027: no `E₀` in the `ICDescriptor`; `E_0` is `SimState`'s, the descriptor's energy derived at read.
    for e in entries() {
        assert!(
            !(e.provenance == Provenance::Decode && e.name.eq_ignore_ascii_case("e_0")),
            "the ICDescriptor stores an E₀ field, `{}`",
            e.name
        );
    }
    check_e0::<f32>(1.0);
    check_e0::<f64>(1.0);
}

negative_control!(
    e0_agreement,
    "an E_0 of the IC with its momenta scaled by 1.001",
    expected = "above 24u",
    {
        check_e0::<f64>(1.001);
    }
);

// ── The shape views (debug_tooling_plan §E; REQ-TOOL-037) ───────────────────────────────────────────────────────

/// A sample that latched a terminal stores every word the same however far past it the horizon runs, so the live
/// views over it are frozen: the head-on collision and the three-way escape, produced at their horizon `H`, at
/// `H + 1` and at `2H`, at f32 and f64. `names` picks the samples (the control picks one with no terminal).
fn check_frozen(names: &[&str]) {
    for &name in names {
        let f = fixtures()
            .into_iter()
            .find(|f| f.name == name)
            .expect("a fixture");
        for steps in [1, 2] {
            let key = sim_key::<f32>(f.horizon);
            let (s, w) = producer::<f32>(&f, &key);
            let later = with_steps(&key, key.horizon_steps * steps + (2 - steps));
            let (t, wt) = producer::<f32>(&f, &later);
            assert!(
                words(&s, false) == words(&t, false) && w == wt,
                "{name} at f32: not frozen: its payload at {} steps differs from {}",
                later.horizon_steps,
                key.horizon_steps
            );
            let key = sim_key::<f64>(f.horizon);
            let (s, w) = producer::<f64>(&f, &key);
            let (t, wt) = producer::<f64>(&f, &with_steps(&key, later.horizon_steps));
            assert!(
                words(&s, false) == words(&t, false) && w == wt,
                "{name} at f64: not frozen: its payload at {} steps differs from {}",
                later.horizon_steps,
                key.horizon_steps
            );
        }
        println!("{name}: frozen past its terminal at f32 and f64");
    }
}

#[test]
fn shape_views_terminal_frozen() {
    check_frozen(&["head-on", "expanding"]);
}

negative_control!(
    shape_views_terminal_frozen,
    "a sample with no terminal, which marches on past the horizon",
    expected = "not frozen",
    check_frozen(&["figure-eight"])
);
