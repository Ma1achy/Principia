//! QA's tests for TASK-M3-06, written from the requirements it closes, each with its negative control (R-176):
//!
//! - REQ-INT-026: the default `SimUniforms` is integrator_contract Part 3's table, value by value as the statement
//!   lists it, and `dt_macro = max(10⁻³, T/65535)`, `T/65535` above `T = 65.535`.
//! - REQ-PAY-037: dispatch refuses `⌈T/dt_macro⌉ = 65536` (and more), accepts 65535, launches nothing it refuses,
//!   asserts `horizon_steps × N_max ≤ 2³² − 1` at its exact edge, and `t_end_step` of a sample run to the horizon is
//!   the latched step count, 65535 included.
//! - REQ-PAY-040: `t_end_step` in bits 0–15 and `t_dmin_step` in bits 16–31, each bit in its own place, exact, and the
//!   display fraction read through the dispatched `horizon_steps` uniform.
//! - REQ-SYS-018: `r_coll` nonzero and `epsilon` 0 by default; one fixed number each, no co-moving option; a run with
//!   `epsilon` other than 0 never reads as an unsoftened one, in its tag or its sim key.
//! - REQ-INT-081: `T_horizon ∈ [0.4, 200]` is accepted everywhere, swept densely and at its edges, with
//!   `⌈T/dt_macro⌉ ≤ 65535`; below 0.4 is refused with R-610's reason; above 200 is refused.
//! - The binding half of REQ-INT-028 (R-502): the table a changed-law `SimUniforms` carries realises that law's
//!   `N_sub = clamp(⌈(r_sub/r_min)^γ_sub⌉, 1, N_max)`, and `N_max = L_max + 1` is refused, launching nothing.

use std::cell::Cell;

use engine::contract::sim_uniforms::{dt_macro_for, SimUniforms, Softening};
use engine::dispatch::validate::{check_total_substeps, dispatch, validate, DispatchError};
use kernel::decode::{self, Latent};
use kernel::driver::nsub_table::NsubTableError;
use kernel::driver::substep::{n_sub, NsubThresholds};
use kernel::driver::wrapper::{integrate, WrapperUniforms};
use kernel::generated::links::Defaults;
use kernel::generated::nsub_thresholds::NSUB_TABLE_MAX;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{
    pack_times, sd_state, set_t_dmin_step, set_t_end_step, tm_t_dmin_fraction, tm_t_dmin_step,
    tm_t_end_fraction, tm_t_end_step, STATE_BOUNDED,
};
use validation::negative_control;

/// A dispatch rule under test: the accepted `horizon_steps`, or the refusal.
type Rule = dyn Fn(&SimUniforms) -> Result<u32, DispatchError>;

/// Dispatch's own rule.
fn real(u: &SimUniforms) -> Result<u32, DispatchError> {
    validate(u).map(|v| v.horizon_steps())
}

/// The default sim key at the horizon `t`, as the settings would set it.
fn at(t: f64) -> SimUniforms {
    SimUniforms::default().with_horizon(t)
}

/// A small deterministic generator (xorshift64*), so every sweep is the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[lo, hi]`.
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 / (1u64 << 53) as f64)
    }
}

// ── REQ-INT-026: Part 3's defaults ──────────────────────────────────────────────────────────────────────────────

/// `u` holds the statement's values: `T_horizon` 50, `dt_macro` 10⁻³, `N_max` 64, `r_sub` 0.05, `gamma_sub` 1.5,
/// `r_coll` set (nonzero, REQ-SYS-018), `tau` set, the 0.4 escape window, `r_close` 0.01, `eps_E` = `eps_L` = 10⁻⁶,
/// `G` = `M_total` = 1, `epsilon` 0.
fn check_part3_defaults(u: &SimUniforms) {
    let exact = [
        ("T_horizon", u.t_horizon, 50.0),
        ("dt_macro", u.dt_macro, 1e-3),
        ("r_sub", u.r_sub, 0.05),
        ("gamma_sub", u.gamma_sub, 1.5),
        ("escape window", u.escape_window, 0.4),
        ("r_close", u.r_close, 0.01),
        ("eps_E", u.eps_e, 1e-6),
        ("eps_L", u.eps_l, 1e-6),
        ("G", u.g, 1.0),
        ("M_total", u.m_total, 1.0),
        ("epsilon", u.epsilon, 0.0),
    ];
    for (name, got, want) in exact {
        let want: f64 = want;
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "default {name} is {got}, Part 3 says {want}"
        );
    }
    assert_eq!(u.n_max, 64, "default N_max is {}, Part 3 says 64", u.n_max);
    assert!(
        u.r_coll.is_finite() && u.r_coll > 0.0,
        "default r_coll is {}, Part 3 says nonzero",
        u.r_coll
    );
    assert!(
        u.tau.is_finite() && u.tau > 0.0,
        "default tau is {}, not a settling threshold",
        u.tau
    );
    assert_eq!(
        real(u),
        Ok(50_000),
        "the default sim key is not dispatched at 50000 macro-steps"
    );
}

#[test]
fn qa_sim_uniforms_defaults_part3() {
    check_part3_defaults(&SimUniforms::default());
}

negative_control!(
    qa_sim_uniforms_defaults_part3,
    "a default whose r_close is r_sub's 0.05 is not Part 3's",
    expected = "default r_close is 0.05",
    check_part3_defaults(&SimUniforms {
        r_close: 0.05,
        ..SimUniforms::default()
    })
);

/// `dt_of` gives `dt_macro` for `T` as R-132's `max(10⁻³, T/65535)`: exactly 10⁻³ at T = 0.4, 0.5, 10 and 50, exactly
/// `200/65535` at T = 200, and `T/65535` above `T = 65.535`. Where the quotient is taken in f64 the macro-step may sit
/// one ulp above the rounded quotient, no more, which is the least that keeps `⌈T/dt⌉ ≤ 65535` evaluable in f64
/// (REQ-INT-081); never below it, which would give 65536 steps.
fn check_dt_macro(dt_of: &dyn Fn(f64) -> f64) {
    for t in [0.4, 0.5, 10.0, 50.0, 65.0] {
        assert_eq!(dt_of(t), 1e-3, "dt_macro at T = {t} is not 10⁻³");
    }
    assert_eq!(
        dt_of(200.0),
        200.0 / 65535.0,
        "dt_macro at T = 200 is not 200/65535"
    );
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..20_000 {
        let t = rng.uniform(65.6, 200.0);
        let q = t / 65535.0;
        let dt = dt_of(t);
        assert!(
            dt == q || dt == q.next_up(),
            "dt_macro at T = {t} is {dt}, not T/65535 = {q} (or its next f64 up)"
        );
    }
}

#[test]
fn qa_sim_uniforms_defaults_dt_macro() {
    check_dt_macro(&dt_macro_for);
    check_dt_macro(&|t| at(t).dt_macro);
}

negative_control!(
    qa_sim_uniforms_defaults_dt_macro,
    "a macro-step without the 10⁻³ floor is T/65535 at every T",
    expected = "is not 10⁻³",
    check_dt_macro(&|t| t / 65535.0)
);

// ── REQ-PAY-037: the u16 limit and the latched t_end_step ───────────────────────────────────────────────────────

/// `rule` refuses every `(T, dt_macro)` whose `⌈T/dt_macro⌉` is 65536 or more, with the typed horizon-steps refusal,
/// and accepts 65535: binary-exact pairs (no rounding in the quotient) and a decimal `dt_macro`, each with `T` in the
/// range, so only the step limit can refuse them.
fn check_u16_limit(rule: &Rule) {
    let base = SimUniforms::default();
    let over = [
        (64.0, 1.0 / 1024.0),                    // 65536 exactly
        (16.0, 1.0 / 4096.0),                    // 65536 exactly
        (128.0, 1.0 / 1024.0),                   // 131072
        (65.5355, 1e-3),                         // ⌈65535.5⌉ = 65536
        (200.0, 1e-3),                           // 200000
        (65535.0 / 1024.0 + 1e-9, 1.0 / 1024.0), // a hair past 65535
    ];
    for (t, dt) in over {
        let u = SimUniforms {
            t_horizon: t,
            dt_macro: dt,
            ..base
        };
        assert!(
            matches!(rule(&u), Err(DispatchError::HorizonSteps { .. })),
            "(T, dt_macro) = ({t}, {dt}), ⌈T/dt⌉ = {}, is not refused at dispatch: {:?}",
            (t / dt).ceil(),
            rule(&u)
        );
    }
    let at_limit = [
        (65535.0 / 1024.0, 1.0 / 1024.0),
        (65535.0 / 4096.0, 1.0 / 4096.0),
    ];
    for (t, dt) in at_limit {
        let u = SimUniforms {
            t_horizon: t,
            dt_macro: dt,
            ..base
        };
        assert_eq!(
            rule(&u),
            Ok(65535),
            "65535 steps at ({t}, {dt}) are not accepted"
        );
    }
}

#[test]
fn qa_horizon_refusal_u16_limit() {
    check_u16_limit(&real);
}

negative_control!(
    qa_horizon_refusal_u16_limit,
    "a dispatch that truncates the step count to u16 accepts 65536 steps as 0",
    expected = "is not refused at dispatch",
    check_u16_limit(&|u: &SimUniforms| Ok(((u.t_horizon / u.dt_macro).ceil() as u32) & 0xFFFF))
);

/// A refused sim key launches nothing; an accepted one launches once with its `horizon_steps`.
fn check_refused_launches_nothing(rule_dispatch: &dyn Fn(&SimUniforms, &Cell<u32>) -> bool) {
    let launched = Cell::new(0);
    let over = SimUniforms {
        t_horizon: 64.0,
        dt_macro: 1.0 / 1024.0,
        ..SimUniforms::default()
    };
    let accepted = rule_dispatch(&over, &launched);
    assert!(
        !accepted && launched.get() == 0,
        "65536 steps launched {} time(s)",
        launched.get()
    );
    assert!(
        rule_dispatch(&SimUniforms::default(), &launched) && launched.get() == 1,
        "the default sim key did not launch once"
    );
}

#[test]
fn qa_horizon_refusal_launches_nothing() {
    check_refused_launches_nothing(&|u, launched| {
        dispatch(u, |_| launched.set(launched.get() + 1)).is_ok()
    });
}

negative_control!(
    qa_horizon_refusal_launches_nothing,
    "a dispatch that launches before it validates launches a refused sim key",
    expected = "65536 steps launched 1 time(s)",
    check_refused_launches_nothing(&|u, launched| {
        launched.set(launched.get() + 1);
        validate(u).is_ok()
    })
);

/// `bound` holds `horizon_steps × N_max ≤ 2³² − 1` at its exact edge: `65535 × 65537 = 2³² − 1` passes, one more
/// `N_max` and `65536 × 65536 = 2³²` (which wraps to 0 in u32) are refused, and the largest steps at `N_max = L_max`
/// pass.
fn check_joint_bound(bound: &dyn Fn(u32, u32) -> Result<(), DispatchError>) {
    assert_eq!(
        bound(65535, 65537),
        Ok(()),
        "65535 × 65537 = 2^32 − 1 is refused"
    );
    assert_eq!(
        bound(65535, NSUB_TABLE_MAX as u32),
        Ok(()),
        "65535 × L_max is refused"
    );
    for (h, n) in [(65535, 65538), (65536, 65536), (u32::MAX, 2), (2, u32::MAX)] {
        assert!(
            matches!(bound(h, n), Err(DispatchError::TotalSubsteps { .. })),
            "{h} × {n} > 2^32 − 1 is not refused: {:?}",
            bound(h, n)
        );
    }
}

#[test]
fn qa_horizon_refusal_joint_bound() {
    check_joint_bound(&check_total_substeps);
}

negative_control!(
    qa_horizon_refusal_joint_bound,
    "a bound multiplied in wrapping u32 lets 65536 × 65536 through as 0",
    expected = "65536 × 65536 > 2^32 − 1 is not refused",
    check_joint_bound(&|h: u32, n: u32| {
        if h.wrapping_mul(n) == 0 && h != 0 && n != 0 {
            Ok(())
        } else {
            check_total_substeps(h, n)
        }
    })
);

/// `uniforms_of` gives the wrapper's loop values for `T`; the sample integrated to the horizon is `bounded` with
/// `t_end_step` the latched `⌈T/dt_macro⌉`, and its `t_end_step / horizon_steps` reads 1, at T = 200 (65535, the top
/// of the u16), T = 3.2105 (⌈3210.5⌉ = 3211) and the floor, T = 0.4 (400).
fn check_latched(uniforms_of: &dyn Fn(f64) -> (u32, WrapperUniforms<f64>)) {
    let ic = decode::decode::<Defaults, f64>(&Latent::new([0.0; 8])).ic;
    for (t, want) in [(200.0, 65535u32), (3.2105, 3211), (0.4, 400)] {
        let (horizon_steps, wu) = uniforms_of(t);
        assert_eq!(horizon_steps, want, "horizon_steps at T = {t}");
        let s = integrate::<Kdk, f64>(&ic, &wu);
        assert_eq!(sd_state(s.packed_a), STATE_BOUNDED, "T = {t}: not bounded");
        assert_eq!(
            tm_t_end_step(s.times),
            want,
            "T = {t}: t_end_step is not the latched step count"
        );
        assert_eq!(
            tm_t_end_fraction(s.times, horizon_steps),
            1.0,
            "T = {t}: a sample at the horizon does not read 1 of the horizon"
        );
    }
}

#[test]
fn qa_horizon_refusal_t_end_latched() {
    check_latched(&|t| {
        let v = validate(&at(t)).expect("an in-range horizon is accepted");
        (v.horizon_steps(), v.wrapper_uniforms_f64())
    });
}

negative_control!(
    qa_horizon_refusal_t_end_latched,
    "a host that skipped the refusal at (64, 2⁻¹⁰) hands the kernel 65536 steps, which the u16 cannot latch",
    expected = "t_end_step is not the latched step count",
    check_latched(&|t| {
        let v = validate(&at(t)).expect("an in-range horizon is accepted");
        if t == 200.0 {
            let wu = WrapperUniforms {
                dt_macro: 1.0 / 1024.0,
                horizon_steps: 65536,
                ..v.wrapper_uniforms_f64()
            };
            (65535, wu)
        } else {
            (v.horizon_steps(), v.wrapper_uniforms_f64())
        }
    })
);

// ── REQ-PAY-040: times as exact u16 step indices ────────────────────────────────────────────────────────────────

/// `pack` puts each bit of `t_end_step` at bits 0–15 and of `t_dmin_step` at 16–31, nothing else, and the accessors and
/// setters read and write each half without touching the other; the display fractions are the index over the
/// `horizon_steps` uniform.
fn check_times(pack: &dyn Fn(u32, u32) -> u32) {
    for i in 0..16 {
        let b = 1u32 << i;
        assert_eq!(pack(b, 0), b, "t_end_step bit {i} is not at bit {i}");
        assert_eq!(
            pack(0, b),
            b << 16,
            "t_dmin_step bit {i} is not at bit {}",
            16 + i
        );
    }
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for _ in 0..100_000 {
        let r = rng.next();
        let (end, dmin) = ((r & 0xFFFF) as u32, ((r >> 16) & 0xFFFF) as u32);
        let w = pack(end, dmin);
        assert_eq!(
            (tm_t_end_step(w), tm_t_dmin_step(w)),
            (end, dmin),
            "times {w:#010X} does not round-trip ({end}, {dmin})"
        );
        let other = ((r >> 32) & 0xFFFF) as u32;
        assert_eq!(
            tm_t_end_step(set_t_dmin_step(w, other)),
            end,
            "setting t_dmin_step moved t_end_step"
        );
        assert_eq!(
            tm_t_dmin_step(set_t_end_step(w, other)),
            dmin,
            "setting t_end_step moved t_dmin_step"
        );
        let h = 1 + ((r >> 48) as u32 % 65535);
        assert_eq!(
            (tm_t_end_fraction(w, h), tm_t_dmin_fraction(w, h)),
            (end as f32 / h as f32, dmin as f32 / h as f32),
            "times {w:#010X}: the display fractions are not the indices over horizon_steps {h}"
        );
    }
}

#[test]
fn qa_times_roundtrip() {
    check_times(&pack_times);
}

negative_control!(
    qa_times_roundtrip,
    "a pack that swaps the halves puts t_end_step in the high bits",
    expected = "t_end_step bit 0 is not at bit 0",
    check_times(&|end: u32, dmin: u32| pack_times(dmin, end))
);

// ── REQ-SYS-018: collision and softening ────────────────────────────────────────────────────────────────────────

/// Every field of `SimUniforms`, named, with no `..`: a co-moving switch or a time-dependent softening added to the
/// struct stops this compiling. `epsilon` and `r_coll` are each one `f64`, fixed for the dispatch.
fn every_field(epsilon: f64, r_coll: f64) -> SimUniforms {
    let d = SimUniforms::default();
    SimUniforms {
        t_horizon: d.t_horizon,
        dt_macro: d.dt_macro,
        n_max: d.n_max,
        r_sub: d.r_sub,
        gamma_sub: d.gamma_sub,
        r_coll,
        tau: d.tau,
        escape_window: d.escape_window,
        r_close: d.r_close,
        eps_e: d.eps_e,
        eps_l: d.eps_l,
        g: d.g,
        m_total: d.m_total,
        epsilon,
        n_renorm: d.n_renorm,
        delta0: d.delta0,
        thresholds: d.thresholds,
    }
}

/// `debug` (a `SimUniforms`' fields, as `Debug` names them) has no co-moving option.
fn check_no_comoving(debug: &str) {
    let lower = debug.to_lowercase();
    for word in ["comoving", "co_moving", "co-moving"] {
        assert!(!lower.contains(word), "a co-moving option exists: {debug}");
    }
}

/// `r_coll` nonzero and `epsilon` 0 by default and unsoftened, untagged; every `epsilon > 0` (down to the least
/// subnormal) is tagged in provenance, and its sim key differs from the unsoftened key and from another `epsilon`'s.
fn check_softening_tag(of: &dyn Fn(f64) -> SimUniforms) {
    let d = of(0.0);
    assert!(d.r_coll > 0.0, "r_coll is not nonzero by default");
    assert_eq!(d.epsilon, 0.0, "epsilon is not 0 by default");
    assert_eq!(
        d.softening(),
        Softening::Unsoftened,
        "the default run is softened"
    );
    assert_eq!(
        d.softening().tag(),
        None,
        "the default run carries a softening tag"
    );
    let base_key = d.sim_key_bytes();
    let mut keys = vec![base_key.clone()];
    for eps in [f64::from_bits(1), 1e-12, 1e-3, 0.05, 1.0] {
        let u = of(eps);
        assert_eq!(
            u.softening(),
            Softening::Softened,
            "epsilon = {eps:e} is not tagged softened"
        );
        assert!(
            u.softening().tag().is_some(),
            "epsilon = {eps:e} carries no provenance tag"
        );
        let key = u.sim_key_bytes();
        assert!(
            !keys.contains(&key),
            "epsilon = {eps:e} shares a sim key with another run"
        );
        keys.push(key);
        assert!(
            validate(&u).is_ok(),
            "epsilon = {eps:e} (optional) is refused"
        );
    }
}

#[test]
fn qa_collision_softening_defaults() {
    check_no_comoving(&format!("{:?}", SimUniforms::default()));
    check_softening_tag(&|eps| every_field(eps, SimUniforms::default().r_coll));
}

negative_control!(
    qa_collision_softening_defaults,
    "a tag that only fires at epsilon ≥ 10⁻⁶ pools the least softening with unsoftened runs",
    expected = "is not tagged softened",
    check_softening_tag(&|eps| every_field(if eps < 1e-6 { 0.0 } else { eps }, 1e-3))
);

#[test]
fn qa_collision_softening_no_comoving_control_input() {
    check_no_comoving("SimUniforms { t_horizon: 50.0, epsilon: 0.0 }");
}

negative_control!(
    qa_collision_softening_no_comoving_control_input,
    "a SimUniforms with a co-moving switch is caught",
    expected = "a co-moving option exists",
    check_no_comoving("SimUniforms { epsilon: 0.0, epsilon_comoving: false }")
);

/// A softening length other than 0 that dispatch accepts is never tagged unsoftened: softened gravity is a different
/// force law at any `epsilon` (dd_generation_root §3.7, "Open"), and a negative or non-number length is either not a
/// length, to be refused, or a softening, to be tagged, but never an unsoftened run.
fn check_nonzero_epsilon_never_unsoftened(judge: &Judge) {
    for eps in [-1e-3, -1.0, f64::NAN, f64::NEG_INFINITY] {
        let (accepted, tag) = judge(&every_field(eps, 1e-3));
        assert!(
            !accepted || tag == Softening::Softened,
            "epsilon = {eps} is dispatched tagged {tag:?}, pooled by its tag with epsilon = 0 runs"
        );
    }
}

/// A sim key's fate: whether dispatch accepts it, and its softening tag.
type Judge = dyn Fn(&SimUniforms) -> (bool, Softening);

#[test]
fn qa_collision_softening_nonzero_epsilon_never_unsoftened() {
    check_nonzero_epsilon_never_unsoftened(&|u| (validate(u).is_ok(), u.softening()));
}

negative_control!(
    qa_collision_softening_nonzero_epsilon_never_unsoftened,
    "a dispatch that accepts every epsilon and tags only epsilon > 0 pools a negative one with epsilon = 0",
    expected = "pooled by its tag with epsilon = 0 runs",
    check_nonzero_epsilon_never_unsoftened(&|u| {
        let tag = if u.epsilon > 0.0 {
            Softening::Softened
        } else {
            Softening::Unsoftened
        };
        (true, tag)
    })
);

// ── REQ-INT-081: the horizon's range ─────────────────────────────────────────────────────────────────────────────

/// The reason R-610 gives for a horizon below the floor.
const BELOW_FLOOR: &str = "below one closure window (0.4) the escape arm cannot fire";

/// `rule`, at the default sim key with the horizon `T` set by the settings, accepts every `T` in `[0.4, 200]`: the
/// edges and their neighbours, the verify's values, the `T/65535` crossover's neighbourhood ulp by ulp, and a dense
/// sweep; each accepted with `⌈T/dt_macro⌉ ≤ 65535` and equal to the f64 ceiling of the carried pair.
fn check_range_accepted(rule: &Rule) {
    let mut ts = vec![
        0.4,
        0.4f64.next_up(),
        0.5,
        1.0,
        10.0,
        50.0,
        65.535,
        100.0,
        200.0f64.next_down(),
        200.0,
    ];
    let mut x = 65.535f64;
    for _ in 0..2000 {
        x = x.next_down();
    }
    for _ in 0..4000 {
        ts.push(x);
        x = x.next_up();
    }
    let mut x = 200.0f64;
    for _ in 0..2000 {
        ts.push(x);
        x = x.next_down();
    }
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for _ in 0..20_000 {
        ts.push(rng.uniform(0.4, 200.0));
    }
    for t in ts {
        let u = at(t);
        match rule(&u) {
            Ok(steps) => {
                assert!(steps <= 65535, "T = {t} dispatched at {steps} steps");
                assert_eq!(
                    steps as f64,
                    (t / u.dt_macro).ceil(),
                    "T = {t}: horizon_steps is not ⌈T/dt_macro⌉"
                );
            }
            Err(e) => panic!("T = {t} in [0.4, 200] is refused at dispatch: {e}"),
        }
    }
    assert_eq!(rule(&at(200.0)), Ok(65535), "T = 200 is not 65535 steps");
    for t in [0.4, 0.5, 10.0] {
        assert_eq!(at(t).dt_macro, 1e-3, "dt_macro at T = {t} is not 10⁻³");
    }
}

#[test]
fn qa_horizon_default_range_accepted() {
    assert_eq!(
        SimUniforms::default().t_horizon,
        50.0,
        "default T_horizon is not 50"
    );
    check_range_accepted(&real);
}

negative_control!(
    qa_horizon_default_range_accepted,
    "the range before R-575, floored at 50, refuses T = 10",
    expected = "in [0.4, 200] is refused at dispatch",
    check_range_accepted(&|u: &SimUniforms| {
        if u.t_horizon < 50.0 {
            Err(DispatchError::HorizonBelowWindow {
                t_horizon: u.t_horizon,
                escape_window: 50.0,
            })
        } else {
            real(u)
        }
    })
);

/// `rule` refuses `T` below the floor, 0.39, 0, −1, the f64 just below 0.4 and the least positive, each with R-610's
/// reason; and `T` above the cap, the f64 just above 200 and 1000, and `T` not a number or infinite.
fn check_range_refused(rule: &Rule) {
    for t in [0.39, 0.0, -1.0, 0.4f64.next_down(), f64::from_bits(1), -0.0] {
        match rule(&at(t)) {
            Ok(steps) => panic!("T = {t}, below 0.4, is accepted at {steps} steps"),
            Err(e) => {
                let reason = e.to_string();
                assert!(
                    reason.contains(BELOW_FLOOR),
                    "T = {t} is refused, but not with R-610's reason: {reason}"
                );
            }
        }
    }
    for t in [
        200.0f64.next_up(),
        1000.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        assert!(
            rule(&at(t)).is_err(),
            "T = {t}, outside [0.4, 200], is accepted"
        );
    }
}

#[test]
fn qa_horizon_default_below_floor_refused() {
    check_range_refused(&real);
}

negative_control!(
    qa_horizon_default_below_floor_refused,
    "the interim range (0, 200] accepts 0.39",
    expected = "T = 0.39, below 0.4, is accepted",
    check_range_refused(&|u: &SimUniforms| {
        if u.t_horizon > 0.0 && u.t_horizon < 0.4 {
            Ok((u.t_horizon / u.dt_macro).ceil() as u32)
        } else {
            real(u)
        }
    })
);

// ── R-502: the table SimUniforms carries realises its law ───────────────────────────────────────────────────────

/// `table` for `(r_sub, gamma_sub, n_max)` gives `N_sub = clamp(⌈(r_sub/r_min)^γ⌉, 1, N_max)` (REQ-INT-028's
/// specification) at every sampled `r_min` whose `(r_sub/r_min)^γ` is not within `4γ` f32 ulps (relative) of a whole
/// number: there the f32 rounding of the threshold and of `d²` (each half an ulp of `d²`, `γ/2` ulps of the power) may
/// decide either side.
fn check_table_realises_law(table: &NsubThresholds, r_sub: f64, gamma: f64, n_max: u32) {
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    let mut checked = 0;
    for _ in 0..50_000 {
        let r_min = r_sub * 10f64.powf(rng.uniform(-2.5, 0.5));
        let x = (r_sub / r_min).powf(gamma);
        if (x - x.round()).abs() < 4.0 * gamma * f64::from(f32::EPSILON) * x {
            continue;
        }
        let want = (x.ceil() as u64).clamp(1, u64::from(n_max)) as u32;
        let d2 = (r_min * r_min) as f32;
        let got = n_sub(d2, table, n_max);
        assert_eq!(
            got, want,
            "r_min = {r_min} under (r_sub, gamma_sub, N_max) = ({r_sub}, {gamma}, {n_max}): N_sub {got}, the law {want}"
        );
        checked += 1;
    }
    assert!(checked > 49_000, "only {checked} samples away from edges");
}

#[test]
fn qa_sim_uniforms_nsub_table_realises_law() {
    let (r_sub, gamma, n_max) = (0.08, 2.0, 37);
    let u = SimUniforms::default()
        .with_substep_law(r_sub, gamma, n_max)
        .expect("a changed law within L_max is generated");
    assert!(validate(&u).is_ok(), "the changed law's sim key is refused");
    check_table_realises_law(&u.thresholds, r_sub, gamma, n_max);
    check_table_realises_law(&SimUniforms::default().thresholds, 0.05, 1.5, 64);
}

negative_control!(
    qa_sim_uniforms_nsub_table_realises_law,
    "the default table under the changed law is not that law",
    expected = "N_sub",
    check_table_realises_law(
        &kernel::driver::nsub_table::pack(
            &kernel::generated::nsub_thresholds::DEFAULT_THRESHOLD_BITS
        ),
        0.08,
        2.0,
        37
    )
);

/// `build` makes the sim key with `N_max = n`; `L_max + 1` is the generator's typed refusal and launches nothing,
/// `L_max` dispatches.
fn check_n_max_refused(build: &dyn Fn(u32) -> SimUniforms) {
    let launched = Cell::new(0);
    let over = NSUB_TABLE_MAX as u32 + 1;
    let r = dispatch(&build(over), |_| launched.set(launched.get() + 1));
    assert!(
        matches!(
            r,
            Err(DispatchError::Nsub(NsubTableError::NMaxAboveTable { n_max })) if n_max == over
        ) && launched.get() == 0,
        "N_max = L_max + 1 = {over} is not the generator's refusal, or launched: {r:?}, {} launch(es)",
        launched.get()
    );
    let max = NSUB_TABLE_MAX as u32;
    assert!(
        dispatch(&build(max), |_| ()).is_ok(),
        "N_max = L_max = {max} is refused"
    );
}

#[test]
fn qa_sim_uniforms_nsub_table_n_max_refused() {
    check_n_max_refused(&|n| {
        let d = SimUniforms::default();
        match d.with_substep_law(d.r_sub, d.gamma_sub, n) {
            Ok(u) => u,
            Err(_) => SimUniforms { n_max: n, ..d },
        }
    });
}

negative_control!(
    qa_sim_uniforms_nsub_table_n_max_refused,
    "an N_max clamped to L_max on the host is dispatched, not refused",
    expected = "is not the generator's refusal",
    check_n_max_refused(&|n| {
        let d = SimUniforms::default();
        d.with_substep_law(d.r_sub, d.gamma_sub, n.min(NSUB_TABLE_MAX as u32))
            .expect("clamped")
    })
);
