//! The COM projection and the invariant monitor (`kernel::driver::{project, monitor, cadence}`; integrator_contract
//! Part 1, "COM projection and invariant monitoring"; dd_integrator §3.4, §3.5, §5 test 7; TASK-M3-05), and the drift
//! suspect predicates read at read time (payload §5; the generated read side):
//! - `com_projection_*`: after one macro-step through the wrapper from a state with a CoM offset and a bulk momentum,
//!   `R_com` and `P_com` are zero to rounding, every occupant, f32 and f64 (REQ-INT-010); the projection weighs by the
//!   masses its cadence cached, not the state's, so `M` is formed once (REQ-INT-010), and it moves only `r` and `p`
//!   (REQ-INT-011).
//! - `invariant_monitor_*`: on a rest start (`E₀ ≠ 0`, `L_{z,0} = 0`) the floored relative forms are finite; a synthetic
//!   spike-then-recover trace latches `max|ΔE| ≫ |ΔE_final|` (REQ-INT-012).
//! - `drift_suspect_read_time_*`: the payload stores no suspect bit; the predicates are the read side's, over the
//!   stored latches; Euler lights `SUSPECT_ENERGY` on every rest start of a grid and on dd tests 2 and 7 (REQ-INT-040).
//! - `close_encounter_drift_shape`: dd test 7's close-encounter IC (`validation::drift::close_encounter`) shows the
//!   stored `max|ΔE|` at least ten times the final drift read from the stored state (REQ-VAL-033).
//!
//! **Domain** (pitfalls §10): "`≫`" is read as at least ten times (applied per R-369). "Everywhere" is the rest starts
//! of the grid below and dd tests 2 and 7: Euler's relative drift there is at least 2.9 at the default horizon, while
//! over latent cubes with momentum it can fall to 10⁻⁷ on a fast, unbound start that barely interacts, below any
//! threshold that leaves the symplectic occupants unlit on test 2 (REVIEW_QUEUE RQ-626).

use kernel::decode::{self, Ic, Latent};
#[cfg(feature = "controls")]
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::monitor::{larger, magnitude, relative, Monitor};
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::project::ComProjection;
use kernel::driver::wrapper::{integrate_with, WrapperReal, WrapperUniforms};
use kernel::generated::links::Defaults;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{Geom, State, Stepper};
use kernel::payload::{
    canonical_nan, drift_suspect, energy_drift, pb_dE_max, pb_dLz_max, sim_state_from_base,
    suspect_energy, suspect_lz, ReadParams, SimStateBase, SUSPECT_ENERGY_THRESHOLD,
    SUSPECT_LZ_THRESHOLD,
};
use kernel::physics::energy::{angular_momentum, energy};
use validation::drift::{
    close_encounter, figure_eight, ic_at, integrate_fixture, AsR, Fixture, EPS_DEFAULT, T_DEFAULT,
};
use validation::negative_control;

/// A precision the wrapper runs at.
trait P: WrapperReal + Into<f64> + std::fmt::Debug {
    const LABEL: &'static str;
    /// The unit roundoff `u`, half the spacing at 1.
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

/// A decoded IC at f64, from latent controls `z`.
fn decoded(z: [f64; 8]) -> Ic<f64> {
    decode::decode::<Defaults, f64>(&Latent::new(z)).ic
}

/// The default sim key's macro-step, one macro-step long.
fn one_step<R: P>() -> WrapperUniforms<R>
where
    f64: AsR<R>,
{
    let config = NsubConfig::DEFAULT;
    WrapperUniforms {
        dt_macro: 1e-3.to_r(),
        horizon_steps: 1,
        n_max: config.n_max,
        thresholds: thresholds(&config).expect("the default sim key generates"),
    }
}

// ── The COM projection (REQ-INT-010, REQ-INT-011) ────────────────────────────────────────────────────────────────

/// A cadence that does nothing: the march with no projection, the control's.
#[cfg(feature = "controls")]
struct Unprojected;

#[cfg(feature = "controls")]
impl<R> Cadence<R> for Unprojected {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {}
    fn done(&self) -> bool {
        false
    }
}

/// `x` widened to f64, exactly.
fn wide<R: P>(x: R) -> f64 {
    x.into()
}

/// `R_com = Σ mᵢrᵢ / M` and `P_com = Σ pᵢ`, each component formed in f64 from the exact values of the masses `m`,
/// positions `r` and momenta `p`.
fn com<R: P>(m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> ([f64; 2], [f64; 2]) {
    let total: f64 = m.iter().map(|&x| wide(x)).sum();
    let c = [0, 1].map(|k| (0..3).map(|i| wide(m[i]) * wide(r[i][k])).sum::<f64>() / total);
    let q = [0, 1].map(|k| (0..3).map(|i| wide(p[i][k])).sum::<f64>());
    (c, q)
}

/// The largest position magnitude of `r` and the summed momentum magnitude of `p`: the scales the projection rounds
/// at.
fn scales<R: P>(r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> (f64, f64) {
    (
        r.iter()
            .flatten()
            .map(|&x| wide(x).abs())
            .fold(0.0, f64::max),
        p.iter().flatten().map(|&x| wide(x).abs()).sum(),
    )
}

/// Asserts that `R_com` and `P_com` of `(r, p)` under the masses `m` are zero to rounding: within `16u` of the
/// position and momentum scales `(r_scale, p_scale)` of the state the projection took (the larger of the start's and
/// the end's, [`scales`]), `u` the precision's unit roundoff; the projection's own rounding is a few `u` of each, the
/// subtracted `R_com` and `P_com` included.
fn check_com_zero<R: P>(
    what: &str,
    m: &[R; 3],
    r: &[[R; 2]; 3],
    p: &[[R; 2]; 3],
    (r_scale, p_scale): (f64, f64),
) {
    let (c, q) = com(m, r, p);
    let (r_tol, p_tol) = (16.0 * R::U * r_scale, 16.0 * R::U * p_scale);
    assert!(
        c.iter().all(|x| x.abs() <= r_tol),
        "{what} at {}: R_com = {c:?} is not zero to rounding (within {r_tol:e})",
        R::LABEL
    );
    assert!(
        q.iter().all(|x| x.abs() <= p_tol),
        "{what} at {}: P_com = {q:?} is not zero to rounding (within {p_tol:e})",
        R::LABEL
    );
}

/// The larger of two [`scales`].
fn wider(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0.max(b.0), a.1.max(b.1))
}

/// The IC `z` decodes to, moved off its centre of mass by `(0.37, −0.21)` and given the bulk velocity `(0.13, 0.29)`,
/// `pᵢ += mᵢ·V`, at `R`.
fn off_com<R: P>(z: [f64; 8]) -> Ic<R>
where
    f64: AsR<R>,
{
    let ic = decoded(z);
    let (c, v) = ([0.37, -0.21], [0.13, 0.29]);
    ic_at::<R>(&Ic {
        m: ic.m,
        r: ic.r.map(|[x, y]| [x + c[0], y + c[1]]),
        p: [0, 1, 2].map(|i| [ic.p[i][0] + ic.m[i] * v[0], ic.p[i][1] + ic.m[i] * v[1]]),
    })
}

/// The latent controls of the projection's ICs: a generic start, a near-collinear one and a rest start.
const Z: [[f64; 8]; 3] = [
    [0.3, -0.4, 0.2, -0.1, 0.25, 0.05, 0.1, -0.2],
    [-1.1, 0.9, -0.3, 0.4, 0.0, 0.2, -0.5, 0.6],
    [0.6, 0.2, 0.0, 0.0, 0.0, 0.0, 0.3, 0.1],
];

/// One macro-step of `A` at `R` from each off-CoM IC, with the wrapper's cadence: its `R_com` and `P_com` are zero.
fn one_step_projected<A: Stepper, R: P>(name: &str)
where
    f64: AsR<R>,
{
    for z in Z {
        let ic = off_com::<R>(z);
        let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
        let out = integrate_with::<A, R, _>(&ic, &one_step::<R>(), &mut cadence);
        let scale = wider(scales(&ic.r, &ic.p), scales(&out.r, &out.p));
        check_com_zero(&format!("{name} from {z:?}"), &ic.m, &out.r, &out.p, scale);
    }
}

#[test]
fn com_projection_after_one_step() {
    for_each_occupant::<f64>();
    for_each_occupant::<f32>();
}

/// [`one_step_projected`] for every occupant at `R`.
fn for_each_occupant<R: P>()
where
    f64: AsR<R>,
{
    one_step_projected::<Euler, R>("Euler");
    one_step_projected::<Kdk, R>("KDK");
    one_step_projected::<Yoshida4, R>("Yoshida-4");
    one_step_projected::<Yoshida6, R>("Yoshida-6");
    one_step_projected::<Rk4, R>("RK4");
}

negative_control!(
    com_projection_after_one_step,
    "the march with no projection keeps the CoM offset",
    expected = "is not zero to rounding",
    {
        let ic = off_com::<f64>(Z[0]);
        let out = integrate_with::<Kdk, f64, _>(&ic, &one_step::<f64>(), &mut Unprojected);
        let scale = wider(scales(&ic.r, &ic.p), scales(&out.r, &out.p));
        check_com_zero("KDK, unprojected", &ic.m, &out.r, &out.p, scale);
    }
);

/// The masses `m` with `m₀` and `m₂` exchanged and `m₁` halved: another state's masses.
fn altered<R: P>(m: [R; 3]) -> [R; 3]
where
    f64: AsR<R>,
{
    [m[2], (wide(m[1]) * 0.5).to_r(), m[0]]
}

/// Projects the off-CoM state of `z` whose masses are another state's ([`altered`]) with `cadence`, built from the
/// IC's own masses `m`, and asserts that the projection weighed by the masses the cadence cached: the projected
/// state's `R_com` and `P_com` under `m` are zero, `M` is `(m₀ + m₁) + m₂` bit for bit, the state's masses are
/// untouched, and the force evaluation is re-formed at the projected positions.
fn check_cached<R: P>(cadence: &WrapperCadence<R>, m: [R; 3], z: [f64; 8])
where
    f64: AsR<R>,
{
    let ic = off_com::<R>(z);
    let mut state = State {
        m: altered(m),
        r: ic.r,
        p: ic.p,
    };
    let mut geom = Geom::new(&state.m, &state.r);
    cadence.project(&mut state, &mut geom);
    let (c, q) = com(&m, &state.r, &state.p);
    let (r_scale, p_scale) = scales(&ic.r, &ic.p);
    assert!(
        c.iter().all(|x| x.abs() <= 16.0 * R::U * r_scale)
            && q.iter().all(|x| x.abs() <= 16.0 * R::U * p_scale),
        "at {}: the projection did not use the cached masses: R_com {c:?}, P_com {q:?} under the IC's masses",
        R::LABEL
    );
    let total = (m[0] + m[1]) + m[2];
    assert_eq!(
        wide(cadence.projection().total_mass()).to_bits(),
        wide(total).to_bits(),
        "at {}: M is not the cached (m0 + m1) + m2",
        R::LABEL
    );
    assert_eq!(
        cadence.projection().masses().map(wide::<R>),
        m.map(wide::<R>),
        "at {}: the projection's masses are not the IC's",
        R::LABEL
    );
    assert_eq!(
        state.m.map(wide::<R>),
        altered(m).map(wide::<R>),
        "at {}: the projection changed the masses",
        R::LABEL
    );
    assert_eq!(
        geom,
        Geom::new(&state.m, &state.r),
        "at {}: the force evaluation is not re-formed at the projected positions",
        R::LABEL
    );
}

/// The cadence of the IC of `z` at `R`, its projection formed from the IC's masses.
fn cadence_of<R: P>(z: [f64; 8]) -> (WrapperCadence<R>, [R; 3])
where
    f64: AsR<R>,
{
    let ic = ic_at::<R>(&decoded(z));
    (WrapperCadence::new(&ic.m, &ic.r, &ic.p), ic.m)
}

#[test]
fn com_projection_m_cached() {
    for z in Z {
        let (c, m) = cadence_of::<f64>(z);
        check_cached(&c, m, z);
        let (c, m) = cadence_of::<f32>(z);
        check_cached(&c, m, z);
    }
    // After a whole run the cadence's projection is still the one formed from the IC's masses.
    let ic = ic_at::<f64>(&decoded(Z[0]));
    let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let mut key = one_step::<f64>();
    key.horizon_steps = 200;
    integrate_with::<Yoshida4, f64, _>(&ic, &key, &mut cadence);
    assert_eq!(
        *cadence.projection(),
        ComProjection::new(&ic.m),
        "the projection changed during the run"
    );
}

negative_control!(
    com_projection_m_cached,
    "a projection formed from the state's masses, recomputed, weighs by the wrong masses",
    expected = "the projection did not use the cached masses",
    {
        let ic = ic_at::<f64>(&decoded(Z[0]));
        let other = altered(ic.m);
        let recomputed = WrapperCadence::new(&other, &ic.r, &ic.p);
        check_cached(&recomputed, ic.m, Z[0]);
    }
);

// ── The invariant monitor (REQ-INT-012) ──────────────────────────────────────────────────────────────────────────

/// The rest start of the IC `z` decodes to: its masses and positions, every momentum 0.
fn rest_start(z: [f64; 8]) -> Ic<f64> {
    let ic = decoded(z);
    Ic {
        m: ic.m,
        r: ic.r,
        p: [[0.0; 2]; 3],
    }
}

/// Asserts that the run `cadence` followed from a rest start stored `E₀ ≠ 0` and `L_{z,0} = 0`, and that the floored
/// relative forms of its final and max drifts are finite at the floors `eps_e`, `eps_l`.
fn check_rest_start<R: P>(what: &str, cadence: &WrapperCadence<R>, eps_e: R, eps_l: R) {
    let m = cadence.monitor_state();
    assert!(
        wide(m.e_0()) != 0.0 && wide(m.lz_0()) == 0.0,
        "{what}: not a rest start: E_0 {:?}, L_z0 {:?}",
        m.e_0(),
        m.lz_0()
    );
    let forms = [
        ("δ_E final", m.delta_e_final(eps_e)),
        ("δ_E max", m.delta_e_max(eps_e)),
        ("δ_L final", m.delta_l_final(eps_l)),
        ("δ_L max", m.delta_l_max(eps_l)),
    ];
    for (name, x) in forms {
        assert!(
            wide(x).is_finite(),
            "{what}: {name} = {x:?} is not finite at L_z0 = 0"
        );
    }
}

/// A short run of `A` at `R` from the rest start of `z`: two time units, 2000 macro-steps.
fn rest_run<A: Stepper, R: P>(z: [f64; 8]) -> WrapperCadence<R>
where
    f64: AsR<R>,
{
    integrate_fixture::<A, R>(&Fixture {
        name: "a rest start",
        ic: rest_start(z),
        horizon: 2.0,
    })
    .1
}

#[test]
fn invariant_monitor_rest_start() {
    let eps = EPS_DEFAULT;
    for z in [Z[0], Z[2]] {
        check_rest_start("Euler f64", &rest_run::<Euler, f64>(z), eps, eps);
        check_rest_start("KDK f64", &rest_run::<Kdk, f64>(z), eps, eps);
        let e = eps as f32;
        check_rest_start("Euler f32", &rest_run::<Euler, f32>(z), e, e);
        check_rest_start("KDK f32", &rest_run::<Kdk, f32>(z), e, e);
    }
}

negative_control!(
    invariant_monitor_rest_start,
    "with no floor, δ_L of a rest start divides by L_z0 = 0",
    expected = "is not finite at L_z0 = 0",
    check_rest_start(
        "Euler f64, no floor",
        &rest_run::<Euler, f64>(Z[0]),
        EPS_DEFAULT,
        0.0
    )
);

/// The state `(m, r, s·p)`: the IC's with its momenta scaled by `s`.
fn scaled(ic: &Ic<f64>, s: f64) -> State<f64> {
    State {
        m: ic.m,
        r: ic.r,
        p: ic.p.map(|v| v.map(|x| x * s)),
    }
}

/// Feeds the monitor of `ic` the states of `trace`, each `ic`'s with its momenta scaled by the trace's factor, and
/// asserts that it latched the worst drifts and the last ones, and that the shape is a spike that recovered:
/// `max|ΔE| ≥ 10·|ΔE_final|` and `max|ΔL_z| ≥ 10·|ΔL_z final|`.
fn check_spike(ic: &Ic<f64>, trace: &[f64]) {
    let geom = Geom::new(&ic.m, &ic.r);
    let mut m = Monitor::new(&ic.m, &ic.r, &ic.p, &geom);
    let (mut de_max, mut dlz_max) = (0.0_f64, 0.0_f64);
    let (mut de, mut dlz) = (0.0, 0.0);
    for &s in trace {
        let st = scaled(ic, s);
        m.update(&st.m, &st.r, &st.p, &geom);
        de = energy(&st.m, &st.p, &geom.pair) - m.e_0();
        dlz = angular_momentum(&st.r, &st.p) - m.lz_0();
        de_max = de_max.max(de.abs());
        dlz_max = dlz_max.max(dlz.abs());
    }
    assert_eq!(
        [m.de_final(), m.de_max(), m.dlz_final(), m.dlz_max()].map(f64::to_bits),
        [de, de_max, dlz, dlz_max].map(f64::to_bits),
        "the monitor's latches are not the trace's final and worst drifts"
    );
    assert!(
        m.de_max() >= 10.0 * m.de_final().abs() && m.dlz_max() >= 10.0 * m.dlz_final().abs(),
        "max|ΔE| = {:e} is not ≫ |ΔE_final| = {:e}, or max|ΔL_z| = {:e} not ≫ |ΔL_z final| = {:e}",
        m.de_max(),
        m.de_final().abs(),
        m.dlz_max(),
        m.dlz_final().abs()
    );
}

/// A trace that spikes and recovers: small, a spike of half again the momenta at the third step, then back to
/// within 10⁻⁶.
const SPIKE: [f64; 6] = [1.000_001, 1.000_002, 1.5, 1.2, 1.000_003, 1.000_001];

#[test]
fn invariant_monitor_spike_then_recover() {
    check_spike(&decoded(Z[0]), &SPIKE);
    check_spike(&decoded(Z[1]), &SPIKE);
}

negative_control!(
    invariant_monitor_spike_then_recover,
    "a trace that ends at its spike, a secular drift, has max ≈ final",
    expected = "is not ≫ |ΔE_final|",
    check_spike(&decoded(Z[0]), &[1.000_001, 1.2, 1.5])
);

/// Asserts the monitor's helpers at their boundaries: `mag`, the magnitude, either side of 0 and at it; `max`, the
/// latch's larger, above, below, equal, and past a NaN, which it never takes; and the floor of `relative` below the
/// initial value's magnitude, at it and above it.
fn check_helpers(mag: fn(f64) -> f64, max: fn(f64, f64) -> f64) {
    assert_eq!(
        [mag(-2.5), mag(2.5), mag(0.0)],
        [2.5, 2.5, 0.0],
        "the magnitude is wrong"
    );
    assert_eq!(
        mag(-0.0).to_bits(),
        (-0.0_f64).to_bits(),
        "the magnitude of −0 is not itself"
    );
    assert_eq!(
        [max(1.0, 2.0), max(2.0, 1.0), max(1.0, f64::NAN)],
        [2.0, 2.0, 1.0],
        "the latch's larger is wrong"
    );
    assert_eq!(
        max(0.0, -0.0).to_bits(),
        0.0_f64.to_bits(),
        "the latch's larger replaces an equal value"
    );
    assert_eq!(
        [
            relative(1.0_f64, 0.5e-6, 1e-6),
            relative(1.0_f64, -1e-6, 1e-6),
            relative(1.0_f64, -2.0, 1e-6)
        ],
        [1e6, 1e6, 0.5],
        "the relative form's floor is wrong"
    );
}

#[test]
fn invariant_monitor_helpers() {
    check_helpers(magnitude, larger);
}

negative_control!(
    invariant_monitor_helpers,
    "a magnitude that keeps the sign is no magnitude",
    expected = "the magnitude is wrong",
    check_helpers(|x| x, larger)
);

// ── The drift suspect predicates, read at read time (REQ-INT-040) ────────────────────────────────────────────────

/// Asserts that none of `names`, the payload's stored struct members, packed words and stored entries, is a suspect
/// bit: none names `suspect`.
fn check_no_suspect_bits(names: &[String]) {
    if let Some(n) = names.iter().find(|n| n.to_lowercase().contains("suspect")) {
        panic!("the payload stores a suspect bit, `{n}`");
    }
}

/// The payload's stored names: every struct member, every packed word and every entry that is not derived at read.
fn stored_names() -> Vec<String> {
    let l = ledger::payload::ledger();
    let mut names: Vec<String> = ledger::payload::structs()
        .iter()
        .flat_map(|s| s.members.iter().map(|m| m.name.to_owned()))
        .collect();
    names.extend(l.words.iter().map(|w| w.name.to_owned()));
    names.extend(
        l.entries
            .iter()
            .filter(|e| !matches!(e.location, Some(ledger::schema::Location::Derived { .. })))
            .filter_map(|e| e.name.map(str::to_owned)),
    );
    names
}

#[test]
fn drift_suspect_read_time_no_stored_bits() {
    let names = stored_names();
    assert!(
        names.iter().any(|n| n == "packed_b"),
        "the ledger's stored names hold no `packed_b`, the drift latches' word"
    );
    check_no_suspect_bits(&names);
}

negative_control!(
    drift_suspect_read_time_no_stored_bits,
    "a stored suspect flag is a suspect bit",
    expected = "the payload stores a suspect bit",
    {
        let mut names = stored_names();
        names.push("suspect_energy".to_owned());
        check_no_suspect_bits(&names);
    }
);

/// The read side's view of the stored `out`, with its masses `m`.
fn read(out: &SimStateBase, m: [f32; 3]) -> kernel::payload::SimState {
    let params = ReadParams {
        dt_macro: 1e-3,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 50_000,
    };
    sim_state_from_base(out, [0; 4], false, canonical_nan(), false, m, &params)
}

/// Asserts that the run of `name`, whose occupant's profile has `symplectic`, stored as `out` with masses `m`, lights
/// `SUSPECT_ENERGY` when read, and that the predicates are the read side's over the stored latches: the relative drift
/// `dE_max / max(ε_E, |E_0|)` against `τ_E` and `dLz_max` against `τ_L`, with the occupant's flag carried beside them.
fn check_lit(name: &str, out: &SimStateBase, m: [f32; 3], symplectic: bool) {
    let eps = EPS_DEFAULT as f32;
    let s = read(out, m);
    let (de_max, dlz_max) = (pb_dE_max(out.packed_b), pb_dLz_max(out.packed_b));
    assert_eq!(
        (s.dE_max.to_bits(), s.dLz_max.to_bits(), s.E_0.to_bits()),
        (de_max.to_bits(), dlz_max.to_bits(), out.E_0.to_bits()),
        "{name}: the read side's latches are not the stored ones"
    );
    let suspect = drift_suspect(&s, eps, symplectic);
    assert_eq!(
        (suspect.energy, suspect.lz, suspect.reads_dynamics),
        (
            de_max / eps.max(out.E_0.abs()) > SUSPECT_ENERGY_THRESHOLD,
            dlz_max > SUSPECT_LZ_THRESHOLD,
            symplectic
        ),
        "{name}: the predicates are not the read of the stored latches"
    );
    assert_eq!(
        (suspect.energy, suspect.lz),
        (suspect_energy(s.dE_max, s.E_0, eps), suspect_lz(s.dLz_max)),
        "{name}: drift_suspect is not the two predicates"
    );
    assert!(
        suspect.energy,
        "{name}: Euler does not light SUSPECT_ENERGY: dE_max {de_max:e} over |E_0| {:e}",
        out.E_0.abs()
    );
}

/// The rest starts of the grid: `z_α`, `z_β` on 8 × 8 points of `[−1.4, 1.4]²`, the momentum controls 0.
fn rest_grid() -> Vec<[f64; 8]> {
    let at = |k: usize| -1.4 + 2.8 * (k as f64 + 0.5) / 8.0;
    (0..64)
        .map(|k| [at(k / 8), at(k % 8), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        .collect()
}

/// [`check_lit`] on the run of `A` at f32 from `fixture`.
fn lit<A: Stepper>(name: &str, fixture: &Fixture) {
    let (out, _) = integrate_fixture::<A, f32>(fixture);
    let m = ic_at::<f32>(&fixture.ic).m;
    check_lit(name, &out, m, A::profile().symplectic);
}

/// Euler lights `SUSPECT_ENERGY` everywhere (REQ-INT-040), read as its bound, interacting states, where its secular
/// drift has the horizon to act: the 8 × 8 rest starts of [`rest_grid`] and dd tests 2 and 7. Over the whole latent
/// cube it does not (a sample that separates early stays under the threshold), which RQ-626 puts to the owner; this is
/// its option (a), applied meanwhile.
#[test]
fn drift_suspect_read_time_euler_everywhere() {
    for z in rest_grid() {
        let fixture = Fixture {
            name: "a rest start",
            ic: decoded(z),
            horizon: T_DEFAULT,
        };
        lit::<Euler>(&format!("Euler from the rest start {z:?}"), &fixture);
    }
    lit::<Euler>("Euler on test 2", &figure_eight());
    lit::<Euler>("Euler on test 7", &close_encounter());
}

negative_control!(
    drift_suspect_read_time_euler_everywhere,
    "KDK on test 2's orbit, its energy error at its oscillation, lights nothing",
    expected = "does not light SUSPECT_ENERGY",
    lit::<Kdk>("KDK on test 2", &figure_eight())
);

// ── dd test 7: the drift shape (REQ-VAL-033) ─────────────────────────────────────────────────────────────────────

/// The run of `A` at `R` from `fixture`: the stored `max|ΔE|`, the f16 latch, and the final drift read from the
/// stored state at f32, `H(r, p) − E_0` by the read side's `energy_drift`, with the live latches beside them.
fn shape<A: Stepper, R: P>(fixture: &Fixture) -> [f64; 4]
where
    f64: AsR<R>,
{
    let (out, cadence) = integrate_fixture::<A, R>(fixture);
    let narrow = |x: R| wide(x) as f32;
    let m = ic_at::<f32>(&fixture.ic).m;
    let final_read = energy_drift(
        out.r.map(|v| v.map(narrow)),
        out.p.map(|v| v.map(narrow)),
        m,
        narrow(out.E_0),
    );
    let live = cadence.monitor_state();
    [
        f64::from(pb_dE_max(out.packed_b)),
        f64::from(final_read).abs(),
        wide(live.de_max()),
        wide(live.de_final()).abs(),
    ]
}

/// Asserts that the run `name`'s maximum is at least ten times its final drift, the spike recovered.
fn check_shape(name: &str, max: f64, last: f64) {
    assert!(
        max >= 10.0 * last,
        "{name}: max|ΔE| = {max:e} is not ≫ |ΔE_final| = {last:e}"
    );
}

#[test]
fn close_encounter_drift_shape() {
    let fixture = close_encounter();
    // The stored latch against the final drift read from the stored state: KDK at both precisions and Yoshida-4 at
    // f64, whose spikes the f16 latch holds.
    let stored = [
        ("KDK f32", shape::<Kdk, f32>(&fixture)),
        ("KDK f64", shape::<Kdk, f64>(&fixture)),
        ("Yoshida-4 f64", shape::<Yoshida4, f64>(&fixture)),
    ];
    for (name, [max, last, ..]) in stored {
        println!("{name}: stored max|ΔE| {max:e}, final read {last:e}");
        check_shape(&format!("{name}, stored"), max, last);
    }
    // Every symplectic occupant at f64 on its live latches: Yoshida-6's spike, 4.7e-9, is below the f16 latch's least
    // subnormal, so it is checked live.
    for (name, [.., max, last]) in [
        ("KDK f64", stored[1].1),
        ("Yoshida-4 f64", stored[2].1),
        ("Yoshida-6 f64", shape::<Yoshida6, f64>(&fixture)),
    ] {
        println!("{name}: live max|ΔE| {max:e}, final {last:e}");
        check_shape(&format!("{name}, live"), max, last);
    }
}

negative_control!(
    close_encounter_drift_shape,
    "test 2's orbit has no close encounter: KDK's max is a few times its final drift",
    expected = "is not ≫ |ΔE_final|",
    {
        let [max, last, ..] = shape::<Kdk, f64>(&figure_eight());
        check_shape("KDK f64 on test 2", max, last);
    }
);

/// A state with `M ≠ 1`, nonzero `E₀` and `L_{z,0}`, no value masked.
fn plain_state() -> State<f64> {
    State {
        m: [0.75, 1.5, 3.25],
        r: [[0.5, -1.5], [2.5, 0.25], [-3.0, 1.75]],
        p: [[1.25, -0.5], [-2.0, 3.0], [0.75, 1.5]],
    }
}

/// Checks `Monitor`'s four relative forms after one update from `plain_state`, at a floor under both invariants.
fn check_relative_forms(read: fn(&Monitor<f64>, f64) -> [f64; 4]) {
    let s = plain_state();
    let mut monitor = Monitor::new(&s.m, &s.r, &s.p, &Geom::new(&s.m, &s.r));
    let mut r = s.r;
    r[0][0] += 0.25;
    let mut p = s.p;
    p[1][1] -= 0.125;
    monitor.update(&s.m, &r, &p, &Geom::new(&s.m, &r));
    let eps = 1e-6;
    let want = [
        relative(monitor.de_final(), monitor.e_0(), eps),
        relative(monitor.de_max(), monitor.e_0(), eps),
        relative(monitor.dlz_final(), monitor.lz_0(), eps),
        relative(monitor.dlz_max(), monitor.lz_0(), eps),
    ];
    assert!(want.iter().all(|x| *x != 0.0), "each drift is nonzero");
    assert_eq!(read(&monitor, eps), want, "the relative forms");
}

fn relative_forms(m: &Monitor<f64>, eps: f64) -> [f64; 4] {
    [
        m.delta_e_final(eps),
        m.delta_e_max(eps),
        m.delta_l_final(eps),
        m.delta_l_max(eps),
    ]
}

#[test]
fn invariant_monitor_relative_forms() {
    check_relative_forms(relative_forms);
}

negative_control!(
    invariant_monitor_relative_forms,
    "forms that read zero",
    expected = "the relative forms",
    check_relative_forms(|_, _| [0.0; 4])
);

/// A velocity form of a projection.
type VelocityForm = fn(&ComProjection<f64>, &[[f64; 2]; 3]) -> [[f64; 2]; 3];

/// Checks the projection at `M ≠ 1`: `R_com` and `V_com` zero after it, each body's relative place kept.
fn check_velocity_form(project: VelocityForm) {
    let s = plain_state();
    let c = ComProjection::new(&s.m);
    let total: f64 = s.m.iter().sum();
    let got = project(&c, &s.r);
    let mean = [0, 1].map(|k| (0..3).map(|i| s.m[i] * s.r[i][k]).sum::<f64>() / total);
    let weighted = c.weighted_mean(&s.r);
    for k in 0..2 {
        assert!((weighted[k] - mean[k]).abs() < 1e-14, "the weighted mean");
    }
    for (i, body) in got.iter().enumerate() {
        for (k, v) in body.iter().enumerate() {
            assert!(
                (v - (s.r[i][k] - mean[k])).abs() < 1e-14,
                "the velocity form at [{i}][{k}]"
            );
        }
    }
}

#[test]
fn com_projection_velocity_form() {
    check_velocity_form(ComProjection::project_velocities);
}

negative_control!(
    com_projection_velocity_form,
    "a velocity form that does nothing",
    expected = "the velocity form",
    check_velocity_form(|_, v| *v)
);

/// Checks that the monitored fields come from the cadence, whatever the march's state held.
fn check_monitored(lz_0: fn(f32, f32) -> f32) {
    let fixture = figure_eight();
    let (out, cadence) = integrate_fixture::<Euler, f32>(&Fixture {
        horizon: 0.01,
        ..fixture
    });
    let mut stale = out;
    stale.Lz_0 = 99.0;
    let got = kernel::driver::wrapper::monitored(stale, &cadence);
    assert_eq!(
        lz_0(got.Lz_0, cadence.monitor_state().lz_0()),
        cadence.monitor_state().lz_0(),
        "the stored L_z0"
    );
}

#[test]
fn invariant_monitor_stores_the_monitors_lz_0() {
    check_monitored(|got, _| got);
}

negative_control!(
    invariant_monitor_stores_the_monitors_lz_0,
    "the march's stale L_z0 kept",
    expected = "the stored L_z0",
    check_monitored(|_, _| 99.0)
);

/// Checks that a run with its word stores what `integrate` stores of the monitored fields.
fn check_word_run_monitored(
    march: fn(&Ic<f64>, &WrapperUniforms<f64>) -> kernel::payload::SimStateBaseOf<f64>,
) {
    let fixture = figure_eight();
    let u = validation::drift::sim_key::<f64>(0.25);
    let plain = kernel::driver::wrapper::integrate::<Euler, f64>(&fixture.ic, &u);
    let got = march(&fixture.ic, &u);
    assert_ne!(plain.packed_b, 0, "the plain run latched a drift");
    assert_eq!(got.packed_b, plain.packed_b, "the word run's latches");
    assert_eq!(got.E_0, plain.E_0, "the word run's E_0");
    assert_eq!(got.Lz_0, plain.Lz_0, "the word run's L_z0");
}

fn word_run(ic: &Ic<f64>, u: &WrapperUniforms<f64>) -> kernel::payload::SimStateBaseOf<f64> {
    let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let mut words = [kernel::word::FreeGroupWord::default()];
    kernel::driver::wrapper::integrate_word::<Euler, f64, _, _>(ic, u, &mut cadence, 0, &mut words)
}

#[test]
fn invariant_monitor_word_run_stores_its_latches() {
    check_word_run_monitored(word_run);
}

negative_control!(
    invariant_monitor_word_run_stores_its_latches,
    "a run whose latches were not written",
    expected = "the word run's latches",
    check_word_run_monitored(|ic, u| {
        let mut out = word_run(ic, u);
        out.packed_b = 0;
        out
    })
);
