//! QA tests for TASK-M3-14's R-972 arms (RQ-791 (d) + (a)), written from REQ-INT-039's verify and REQ-PAY-032's clause,
//! at f32, the payload's precision, read through the generated read side (`sim_state_from_ftle`) as the fragment's twin.
//! Each name carries `shadow_own_projection`, so the task's acceptance command for REQ-INT-039,
//! `cargo test -p kernel shadow_own_projection`, runs them.
//!
//! - `qa_m314_shadow_own_projection_shadow_only_collision` (REQ-INT-039, R-972 (d)): at a nonzero `r_coll` that only the
//!   shadow comes inside, the shadow's detector fires, and every stored field (the sample's and the shadow's `r_sh`,
//!   `p_sh`, `S`) is bit for bit that of the same run with the shadow's collision test removed; the sample is no
//!   collision; the shadow renormalised; `ftle_valid` stays true and `ftle` finite. Control: a shadow whose collision
//!   ends its march.
//! - `qa_m314_shadow_own_projection_coincident_shadow` (REQ-INT-039, REQ-PAY-032, R-972 (a), R-79): a shadow whose
//!   `‖δ₀‖` is below f32's resolution at the sample's state starts on the sample, so `δ = 0` and its sum is `−∞`, at the
//!   first renormalisation or only in the finalised sum: a healthy sample (bitwise the base tier's) stores no NaN or
//!   infinity, its `S` is `S_FAILED_BITS`; `ftle_valid` is false and `ftle` NaN. Control: the mark read as valid
//!   (`ftle_valid` without R-972's clause).
//! - `qa_m314_shadow_own_projection_aggregate_drops` (REQ-INT-039, REQ-PAY-032, R-255): a mean over `ftle` taken by
//!   `ftle_valid` over healthy samples and the failed-sum one equals the healthy samples' mean and counts them only.
//!   Control: the same mean where the mark is read as valid.
//!
//! **Domain** (pitfalls §10): Yoshida-4 at f32, the close pair at separation 0.03 over the sim key at `t = 1`, `r_coll`
//! a nonzero radius placed between the sample's and the shadow's least separations, or below both.
//!
//! Each test has a registered negative control naming the panic it expects (R-176).

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
#[cfg(feature = "controls")]
use kernel::driver::advance::{Advance, AdvanceParams};
use kernel::driver::benettin::{integrate_ftle_with, offset, Absolute, Benettin, BenettinParams};
use kernel::driver::cadence::WrapperCadence;
#[cfg(feature = "controls")]
use kernel::driver::wrapper::CoMarch;
use kernel::driver::wrapper::{integrate_detecting, integrate_with_co, monitored, WrapperUniforms};
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::State;
#[cfg(feature = "controls")]
use kernel::payload::{benettin_delta, ftle};
use kernel::payload::{
    completed_renorms, sd_is_failed, sd_state, sim_state_from_ftle, tm_t_end_step, ReadParams,
    SimStateBaseOf, SimStateFTLE, STATE_COLLISION, S_FAILED_BITS,
};
use validation::drift::sim_key;
use validation::negative_control;

/// The close pair (separation 0.03, on a circular mutual orbit) with a third body far off, the centre of mass at the
/// origin and every position component a shadow offset touches nonzero, so an offset below f32's resolution there
/// leaves the state unchanged.
fn pair() -> Ic<f32> {
    let p = ((2.0f64 / 3.0 / 0.03).sqrt() / 2.0 / 3.0) as f32;
    let third = 1.0f32 / 3.0;
    Ic {
        m: [third; 3],
        r: [[-0.015, -third], [0.015, -third], [0.0, 2.0 * third]],
        p: [[0.0, -p], [0.0, p], [0.0, 0.0]],
    }
}

fn key() -> WrapperUniforms<f32> {
    sim_key::<f32>(1.0)
}

/// The generated read side's parameters for `params`.
fn read_params(params: &BenettinParams<f32>) -> ReadParams {
    let k = key();
    ReadParams {
        dt_macro: k.dt_macro,
        delta_0: params.delta_0,
        n_renorm: params.n_renorm,
        horizon_steps: k.horizon_steps,
    }
}

/// A read of `(ftle, ftle_valid)` from a stored FTLE-tier sample.
type Reader = fn(&SimStateFTLE, &ReadParams) -> (f32, bool);

/// The generated read side's (`sim_state_from_ftle`, the fragment's Rust twin).
fn generated(s: &SimStateFTLE, rp: &ReadParams) -> (f32, bool) {
    let read = sim_state_from_ftle(s, [0; 4], false, 0.0, false, pair().m, rp);
    (read.ftle, read.ftle_valid)
}

/// `ftle_valid` as before R-972, its clause on `S` dropped (the control's reader): the mark read as a sum.
#[cfg(feature = "controls")]
fn without_clause(s: &SimStateFTLE, rp: &ReadParams) -> (f32, bool) {
    let n = tm_t_end_step(s.times);
    let valid = !sd_is_failed(s.packed_a) && n > 0 && completed_renorms(n, rp.n_renorm) > 0;
    let d = benettin_delta(s.r, s.p, s.r_sh, s.p_sh);
    (ftle(s.S, d, rp.delta_0, n, rp.dt_macro, valid), valid)
}

/// Every stored member's bits, in declaration order.
fn bits(s: &SimStateFTLE) -> Vec<u32> {
    let reals = [s.r, s.p, s.r_sh, s.p_sh]
        .into_iter()
        .flatten()
        .flatten()
        .chain([s.S, s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0, s.closure_min])
        .map(f32::to_bits);
    reals
        .chain([s.packed_a, s.packed_b, s.times, s.total_substeps])
        .chain([u32::from(s.closure_step), u32::from(s._reserved)])
        .collect()
}

/// The sample's own stored members' bits (every member but the shadow's `r_sh`, `p_sh`, `S`).
fn sample_bits(s: &SimStateFTLE) -> Vec<u32> {
    let reals = [s.r, s.p]
        .into_iter()
        .flatten()
        .flatten()
        .chain([s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0, s.closure_min])
        .map(f32::to_bits);
    reals
        .chain([s.packed_a, s.packed_b, s.times, s.total_substeps])
        .chain([u32::from(s.closure_step), u32::from(s._reserved)])
        .collect()
}

/// The base tier's members' bits, in [`sample_bits`]' order.
fn base_bits(s: &SimStateBaseOf<f32>) -> Vec<u32> {
    let reals = [s.r, s.p]
        .into_iter()
        .flatten()
        .flatten()
        .chain([s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0, s.closure_min])
        .map(f32::to_bits);
    reals
        .chain([s.packed_a, s.packed_b, s.times, s.total_substeps])
        .chain([u32::from(s.closure_step), u32::from(s._reserved)])
        .collect()
}

/// An FTLE-tier run: the sample's cadence at `sample_radius`, its shadow's at `shadow_radius`; the stored sample and
/// whether the shadow's own detector fired.
type Run =
    fn(&Ic<f32>, CollisionRadius, CollisionRadius, BenettinParams<f32>) -> (SimStateFTLE, bool);

/// The kernel's co-march with each trajectory's own threshold.
fn run(
    ic: &Ic<f32>,
    sample_radius: CollisionRadius,
    shadow_radius: CollisionRadius,
    params: BenettinParams<f32>,
) -> (SimStateFTLE, bool) {
    let mut cadence = WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, sample_radius);
    let mut shadow = Benettin::<f32, Absolute>::new(ic, shadow_radius, params);
    let out = integrate_with_co::<Yoshida4, f32, _, _>(ic, &key(), &mut cadence, &mut shadow);
    let out = monitored(out, &cadence);
    (shadow.write(out), shadow.cadence().detector().fired())
}

// ── qa_m314_shadow_own_projection_shadow_only_collision (REQ-INT-039; R-972 (d)) ──────────────────────────────────

fn pair_params() -> BenettinParams<f32> {
    BenettinParams {
        delta_0: 1e-3,
        n_renorm: 10,
    }
}

/// The sample's and the shadow's least `d²` at `r_coll = 0`, each from its own detector's latch.
fn least_d2(ic: &Ic<f32>) -> (f32, f32) {
    let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let mut shadow = Benettin::<f32, Absolute>::new(ic, CollisionRadius::none(), pair_params());
    integrate_with_co::<Yoshida4, f32, _, _>(ic, &key(), &mut cadence, &mut shadow);
    (
        cadence.detector().latch().d2(),
        shadow.cadence().detector().latch().d2(),
    )
}

fn check_shadow_only_collision(under_test: Run) {
    let ic = pair();
    let (sample_d2, shadow_d2) = least_d2(&ic);
    assert!(
        shadow_d2 < sample_d2,
        "fixture: the shadow's least d² {shadow_d2} is not below the sample's {sample_d2}"
    );
    let radius = CollisionRadius::from_squared(0.5 * (shadow_d2 + sample_d2));
    assert!(radius.squared() > 0.0, "fixture: r_coll is zero");

    let (s, fired) = under_test(&ic, radius, radius, pair_params());
    assert!(
        fired,
        "the shadow's own detector did not fire: no shadow-only collision"
    );
    let (removed, removed_fired) = run(&ic, radius, CollisionRadius::none(), pair_params());
    assert!(
        !removed_fired,
        "fixture: the shadow without a collision test fired"
    );
    assert_eq!(
        bits(&s),
        bits(&removed),
        "the run differs from the same run with the shadow's collision test removed"
    );
    let base = integrate_detecting::<Yoshida4, f32>(&ic, &key(), radius);
    assert_eq!(
        sample_bits(&s),
        base_bits(&base),
        "the sample is not the base tier's"
    );
    assert_ne!(sd_state(s.packed_a), STATE_COLLISION, "the sample collided");
    let n = tm_t_end_step(s.times);
    assert!(
        completed_renorms(n, pair_params().n_renorm) > 0 && s.S != 0.0 && s.S.is_finite(),
        "the shadow did not renormalise as a moving neighbour: n = {n}, S = {}",
        s.S
    );
    let (lambda, valid) = generated(&s, &read_params(&pair_params()));
    assert!(
        valid && lambda.is_finite(),
        "beside a shadow-only collision ftle_valid is {valid} and ftle {lambda}"
    );
}

#[test]
fn qa_m314_shadow_own_projection_shadow_only_collision() {
    check_shadow_only_collision(run);
}

/// A co-march whose shadow stops once its own detector has fired, as a shadow terminal would stop it.
#[cfg(feature = "controls")]
struct StopsAtTerminal(Benettin<f32, Absolute>);

#[cfg(feature = "controls")]
impl CoMarch<f32> for StopsAtTerminal {
    fn after_macro<A: Advance>(
        &mut self,
        start: &State<f32>,
        end: &State<f32>,
        t_now: u32,
        params: &AdvanceParams<f32>,
        taken: u32,
    ) {
        if !self.0.cadence().detector().fired() {
            self.0.after_macro::<A>(start, end, t_now, params, taken);
        }
    }
}

negative_control!(
    qa_m314_shadow_own_projection_shadow_only_collision,
    "a shadow whose collision ends its march must fail",
    expected = "the run differs from the same run with the shadow's collision test removed",
    check_shadow_only_collision(|ic, sample_radius, shadow_radius, params| {
        let mut cadence = WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, sample_radius);
        let mut shadow = StopsAtTerminal(Benettin::new(ic, shadow_radius, params));
        let out = integrate_with_co::<Yoshida4, f32, _, _>(ic, &key(), &mut cadence, &mut shadow);
        let out = monitored(out, &cadence);
        (shadow.0.write(out), shadow.0.cadence().detector().fired())
    })
);

// ── qa_m314_shadow_own_projection_coincident_shadow (REQ-INT-039, REQ-PAY-032; R-972 (a), R-79) ──────────────────

/// `‖δ₀‖ = 10⁻¹²`: each offset component, `±5·10⁻¹³`, is far below half an ulp of the pair's positions (`ulp(0.015) ≈
/// 9.3·10⁻¹⁰`), so the shadow starts on the sample.
const UNRESOLVED_DELTA_0: f32 = 1e-12;

/// A nonzero `r_coll` below the pair's separation, which no trajectory comes inside.
fn small_radius() -> CollisionRadius {
    CollisionRadius::from_squared(1e-4)
}

/// The coincident shadow's runs: renormalised every 10 macro-steps (the sum goes `−∞` at the first), or never within
/// the run (only the finalised sum at write is `−∞`).
fn coincident_runs() -> Vec<(BenettinParams<f32>, SimStateFTLE)> {
    let ic = pair();
    let x = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let start = offset(&x, UNRESOLVED_DELTA_0);
    assert!(
        start.r == x.r && start.p == x.p,
        "fixture: the offset at ‖δ₀‖ = 1e-12 moved the state"
    );
    [10, 100_000]
        .into_iter()
        .map(|n_renorm| {
            let params = BenettinParams {
                delta_0: UNRESOLVED_DELTA_0,
                n_renorm,
            };
            let (s, _) =
                integrate_ftle_with::<Yoshida4, f32, Absolute>(&ic, &key(), small_radius(), params);
            (params, s)
        })
        .collect()
}

fn check_coincident_shadow(read: Reader) {
    let base = integrate_detecting::<Yoshida4, f32>(&pair(), &key(), small_radius());
    for (params, s) in coincident_runs() {
        let at = params.n_renorm;
        assert!(
            !sd_is_failed(s.packed_a) && sd_state(s.packed_a) != STATE_COLLISION,
            "fixture (n_renorm {at}): the sample is not healthy"
        );
        assert_eq!(
            sample_bits(&s),
            base_bits(&base),
            "n_renorm {at}: the shadow's failed sum reached the sample"
        );
        let shadow_fields: Vec<f32> = [s.r_sh, s.p_sh]
            .into_iter()
            .flatten()
            .flatten()
            .chain([s.S])
            .collect();
        assert!(
            shadow_fields.iter().all(|v| v.is_finite()),
            "n_renorm {at}: the shadow's stored fields hold a non-finite value (R-79): {shadow_fields:?}"
        );
        assert_eq!(
            s.S.to_bits(),
            S_FAILED_BITS,
            "n_renorm {at}: a −∞ sum did not store the mark"
        );
        let (lambda, valid) = read(&s, &read_params(&params));
        assert!(
            !valid && lambda.is_nan(),
            "n_renorm {at}: a non-finite shadow sum reads ftle_valid {valid}, ftle {lambda}"
        );
    }
}

#[test]
fn qa_m314_shadow_own_projection_coincident_shadow() {
    check_coincident_shadow(generated);
}

negative_control!(
    qa_m314_shadow_own_projection_coincident_shadow,
    "a non-finite shadow S read as valid must fail",
    expected = "a non-finite shadow sum reads ftle_valid true",
    check_coincident_shadow(without_clause)
);

// ── qa_m314_shadow_own_projection_aggregate_drops (REQ-INT-039, REQ-PAY-032; R-255) ─────────────────────────────

/// The mean of `ftle` over the samples whose `ftle_valid` holds, and how many there are.
fn mean_by_validity(samples: &[(SimStateFTLE, ReadParams)], read: Reader) -> (f64, usize) {
    let valid: Vec<f64> = samples
        .iter()
        .map(|(s, rp)| read(s, rp))
        .filter(|&(_, v)| v)
        .map(|(l, _)| f64::from(l))
        .collect();
    (valid.iter().sum::<f64>() / valid.len() as f64, valid.len())
}

fn check_aggregate_drops(read: Reader) {
    let ic = pair();
    let healthy: Vec<(SimStateFTLE, ReadParams)> = [(1e-3, 10), (2e-3, 10), (1e-3, 20)]
        .into_iter()
        .map(|(delta_0, n_renorm)| {
            let params = BenettinParams { delta_0, n_renorm };
            let (s, _) =
                integrate_ftle_with::<Yoshida4, f32, Absolute>(&ic, &key(), small_radius(), params);
            (s, read_params(&params))
        })
        .collect();
    let (want, count) = mean_by_validity(&healthy, generated);
    assert!(
        count == healthy.len() && want.is_finite(),
        "fixture: the healthy samples are not all valid and finite"
    );
    let mut all = healthy.clone();
    all.extend(
        coincident_runs()
            .into_iter()
            .map(|(params, s)| (s, read_params(&params))),
    );
    let (got, got_count) = mean_by_validity(&all, read);
    assert!(
        got_count == healthy.len() && got.to_bits() == want.to_bits(),
        "the aggregate over ftle did not drop the failed-sum samples: mean {got} over {got_count}, want {want} over {}",
        healthy.len()
    );
}

#[test]
fn qa_m314_shadow_own_projection_aggregate_drops() {
    check_aggregate_drops(generated);
}

negative_control!(
    qa_m314_shadow_own_projection_aggregate_drops,
    "an aggregate whose read takes the mark as valid must fail",
    expected = "the aggregate over ftle did not drop the failed-sum samples",
    check_aggregate_drops(without_clause)
);
