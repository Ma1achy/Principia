//! `SimUniforms` and dispatch's refusals (`engine::contract::sim_uniforms`, `engine::dispatch::validate`;
//! integrator_contract Part 3, Part 5; dd_simstate_payload §2, `times`; dd_generation_root §3.7, "Open"; R-86, R-132,
//! R-502, R-575, R-610; TASK-M3-06):
//! - `sim_uniforms_defaults`: the default `SimUniforms` is Part 3's table, and `dt_macro = T/65535` above `T = 65.535`,
//!   `10⁻³` at and below it (REQ-INT-026).
//! - `horizon_refusal`: a `(T, dt_macro)` giving 65536 macro-steps is refused at dispatch and launches nothing, 65535
//!   is accepted; the other refusals' edges; the joint `total_substeps` bound; and a terminal sample's `t_end_step` is
//!   the latched step count (REQ-PAY-037).
//! - `collision_softening_defaults`: `r_coll` nonzero and `epsilon` 0 by default, no co-moving field, and `epsilon > 0`
//!   tagged in the provenance and on the sim key (REQ-SYS-018).
//! - `sim_uniforms_nsub_table`: the table `SimUniforms` carries is the host generator's for a changed config, and
//!   dispatch returns the generator's refusal at `N_max = L_max + 1` and launches nothing (REQ-INT-028, in part; R-502).
//! - `horizon_default`: the default horizon, `T = 200`, and `T = 10`, 0.5 and 0.4 accepted; nothing in `[0.4, 200]`
//!   refused, and a `T` below 0.4 refused with its reason (REQ-INT-081).

use std::cell::Cell;

use engine::contract::sim_uniforms::{dt_macro_for, horizon_steps, SimUniforms, Softening};
use engine::dispatch::validate::{
    check_total_substeps, dispatch, validate, DispatchError, Validated,
};
use kernel::decode::{self, Latent};
use kernel::driver::nsub_table::{pack, threshold_bits, NsubConfig, NsubTableError};
use kernel::driver::substep::NsubThresholds;
use kernel::driver::wrapper::{integrate, WrapperUniforms};
use kernel::generated::links::Defaults;
use kernel::generated::nsub_thresholds::{DEFAULT_THRESHOLD_BITS, NSUB_TABLE_MAX};
use kernel::occupant::kdk::Kdk;
use kernel::payload::{sd_state, tm_t_end_step, STATE_BOUNDED};
use validation::negative_control;

/// A dispatch check: the sim key's `horizon_steps`, or its refusal.
type Check = dyn Fn(&SimUniforms) -> Result<u32, DispatchError>;

/// Dispatch's own check.
fn real(u: &SimUniforms) -> Result<u32, DispatchError> {
    validate(u).map(|v| v.horizon_steps())
}

/// `SimUniforms::default()` at the horizon `t`.
fn at(t: f64) -> SimUniforms {
    SimUniforms::default().with_horizon(t)
}

/// A table's bits, `THR[1] … THR[L_max]`, read back from its rows.
fn bits(table: &NsubThresholds) -> Vec<u32> {
    table
        .rows
        .iter()
        .flat_map(|row| row.to_array())
        .take(NSUB_TABLE_MAX)
        .collect()
}

// ── sim_uniforms_defaults (REQ-INT-026) ──────────────────────────────────────────────────────────────────────────

/// `u` is Part 3's table: `T_horizon = 50`, `dt_macro = 10⁻³`, `N_max = 64`, `r_sub = 0.05`, `gamma_sub = 1.5`,
/// `r_coll = 10⁻³`, `tau = 10⁻³`, the 0.4 escape window, `r_close = 0.01`, `eps_E = eps_L = 10⁻⁶`, `G = M_total = 1`,
/// `epsilon = 0`, no `n_renorm` or `δ₀` yet, and the build-time table.
fn check_defaults(u: &SimUniforms) {
    let expected: [(&str, f64, f64); 14] = [
        ("T_horizon", u.t_horizon, 50.0),
        ("dt_macro", u.dt_macro, 1e-3),
        ("N_max", f64::from(u.n_max), 64.0),
        ("r_sub", u.r_sub, 0.05),
        ("gamma_sub", u.gamma_sub, 1.5),
        ("r_coll", u.r_coll, 1e-3),
        ("tau", u.tau, 1e-3),
        ("escape window", u.escape_window, 0.4),
        ("r_close", u.r_close, 0.01),
        ("eps_E", u.eps_e, 1e-6),
        ("eps_L", u.eps_l, 1e-6),
        ("G", u.g, 1.0),
        ("M_total", u.m_total, 1.0),
        ("epsilon", u.epsilon, 0.0),
    ];
    for (name, got, want) in expected {
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "the default {name} is {got}, not Part 3's {want}"
        );
    }
    assert_eq!(
        u.n_renorm, None,
        "n_renorm has a default before REQ-INT-076"
    );
    assert_eq!(u.delta0, None, "δ₀ has a default before REQ-INT-076");
    assert_eq!(
        bits(&u.thresholds),
        DEFAULT_THRESHOLD_BITS,
        "the default table is not the build-time table"
    );
}

/// `dt_of` is `max(10⁻³, T/65535)`: `T/65535` above 65.535 where its quotient stays at 65535, `10⁻³` at and below.
fn check_dt_macro(dt_of: &dyn Fn(f64) -> f64) {
    for t in [65.536, 100.0, 150.0, 200.0] {
        let dt = dt_of(t);
        assert_eq!(dt, t / 65535.0, "dt_macro at T = {t} is {dt}, not T/65535");
        assert_eq!(horizon_steps(t, dt), 65535.0, "⌈T/dt_macro⌉ at T = {t}");
    }
    for t in [0.4, 0.5, 10.0, 50.0, 65.0, 65.535] {
        let dt = dt_of(t);
        assert_eq!(dt, 1e-3, "dt_macro at T = {t} is {dt}, not 10⁻³");
    }
    // 65.535 is the edge: 65.535/65535 rounds to 10⁻³ itself, so both arms agree there.
    assert_eq!(
        horizon_steps(65.535, dt_of(65.535)),
        65535.0,
        "⌈T/dt_macro⌉ at T = 65.535"
    );
    // At T = 65.538, T/65535 rounded to nearest gives a quotient whose ceiling is 65536; dt_macro is raised one ulp.
    let t = 65.538;
    assert_eq!(
        horizon_steps(t, t / 65535.0),
        65536.0,
        "the nearest dt_macro at {t}"
    );
    let dt = dt_of(t);
    assert_eq!(
        dt,
        (t / 65535.0).next_up(),
        "dt_macro at T = {t} is {dt}, not raised one ulp"
    );
    assert_eq!(horizon_steps(t, dt), 65535.0, "⌈T/dt_macro⌉ at T = {t}");
}

#[test]
fn sim_uniforms_defaults() {
    check_defaults(&SimUniforms::default());
}

negative_control!(
    sim_uniforms_defaults,
    "a horizon of 200 is not Part 3's default",
    expected = "the default T_horizon is 200",
    check_defaults(&at(200.0))
);

#[test]
fn sim_uniforms_defaults_dt_macro() {
    check_dt_macro(&dt_macro_for);
}

negative_control!(
    sim_uniforms_defaults_dt_macro,
    "T/65535 rounded to nearest, never raised, gives 65536 steps at T = 65.538",
    expected = "not raised one ulp",
    check_dt_macro(&|t: f64| (1e-3f64).max(t / 65535.0))
);

// ── horizon_refusal (REQ-PAY-037) ────────────────────────────────────────────────────────────────────────────────

/// `check` refuses `(T, dt_macro) = (64, 2⁻¹⁰)`, exactly 65536 macro-steps, naming them, and accepts
/// `(65535·2⁻¹⁰, 2⁻¹⁰)`, exactly 65535; `launch` is called only for the accepted one.
fn check_refuses_65536(check: &Check) {
    let dt = 1.0 / 1024.0;
    let over = SimUniforms {
        t_horizon: 64.0,
        dt_macro: dt,
        ..SimUniforms::default()
    };
    match check(&over) {
        Err(DispatchError::HorizonSteps { steps, .. }) => {
            assert_eq!(steps, 65536.0, "the refusal's step count")
        }
        other => panic!("(T, dt_macro) = (64, 2⁻¹⁰), 65536 steps, is not refused: {other:?}"),
    }
    let edge = SimUniforms {
        t_horizon: 65535.0 * dt,
        ..over
    };
    assert_eq!(check(&edge), Ok(65535), "65535 steps are refused");
}

/// `uniforms_of` gives the kernel's loop values for a validated sim key; the run at `T` = 0.4 and 1, `bounded` at the
/// horizon, latches `t_end_step` at `horizon_steps`, at f64 and at f32.
fn check_t_end_latched(uniforms_of: &dyn Fn(&Validated) -> WrapperUniforms<f64>) {
    let ic64 = decode::decode::<Defaults, f64>(&Latent::new([0.0; 8])).ic;
    let ic32 = decode::decode::<Defaults, f32>(&Latent::new([0.0; 8])).ic;
    for (t, steps) in [(0.4, 400), (1.0, 1000)] {
        let v = validate(&at(t)).expect("an in-range horizon is accepted");
        assert_eq!(v.horizon_steps(), steps, "horizon_steps at T = {t}");
        let s64 = integrate::<Kdk, f64>(&ic64, &uniforms_of(&v));
        let s32 = integrate::<Kdk, f32>(&ic32, &v.wrapper_uniforms_f32());
        for (label, packed_a, times) in [
            ("f64", s64.packed_a, s64.times),
            ("f32", s32.packed_a, s32.times),
        ] {
            assert_eq!(
                sd_state(packed_a),
                STATE_BOUNDED,
                "{label} at T = {t}: not bounded"
            );
            assert_eq!(
                tm_t_end_step(times),
                v.horizon_steps(),
                "{label} at T = {t}: t_end_step is not the latched step count"
            );
        }
    }
}

#[test]
fn horizon_refusal() {
    check_refuses_65536(&real);
    let launched = Cell::new(0);
    let over = SimUniforms {
        t_horizon: 64.0,
        dt_macro: 1.0 / 1024.0,
        ..SimUniforms::default()
    };
    let refused = dispatch(&over, |_| launched.set(launched.get() + 1));
    assert!(refused.is_err(), "65536 steps dispatched");
    assert_eq!(launched.get(), 0, "a refused sim key launched");
    let accepted = dispatch(&SimUniforms::default(), |v| {
        launched.set(launched.get() + 1);
        v.horizon_steps()
    });
    assert_eq!(accepted, Ok(50000), "the default sim key's dispatch");
    assert_eq!(launched.get(), 1, "an accepted sim key launches once");
    let message = refused.unwrap_err().to_string();
    assert!(
        message.contains("65536 macro-steps is above 65535"),
        "the refusal's reason: {message}"
    );
}

negative_control!(
    horizon_refusal,
    "a dispatch without the step limit accepts 65536 steps",
    expected = "65536 steps, is not refused",
    check_refuses_65536(&|u: &SimUniforms| Ok(horizon_steps(u.t_horizon, u.dt_macro) as u32))
);

#[test]
fn horizon_refusal_t_end_step_latched() {
    check_t_end_latched(&Validated::wrapper_uniforms_f64);
}

negative_control!(
    horizon_refusal_t_end_step_latched,
    "a host that hands the kernel one step fewer than horizon_steps latches the wrong count",
    expected = "t_end_step is not the latched step count",
    check_t_end_latched(&|v: &Validated| WrapperUniforms {
        horizon_steps: v.horizon_steps() - 1,
        ..v.wrapper_uniforms_f64()
    })
);

/// `check` refuses `dt_macro` 0, negative or not finite, an escape window 0, negative or not finite, and a `T` not
/// finite, each with its own refusal.
fn check_other_refusals(check: &Check) {
    for dt in [0.0, -1e-3, f64::NAN, f64::INFINITY] {
        let u = SimUniforms {
            dt_macro: dt,
            ..SimUniforms::default()
        };
        assert!(
            matches!(check(&u), Err(DispatchError::DtMacro { .. })),
            "dt_macro {dt} is not refused as such: {:?}",
            check(&u)
        );
    }
    for w in [0.0, -0.4, f64::NAN, f64::INFINITY] {
        let u = SimUniforms {
            escape_window: w,
            ..SimUniforms::default()
        };
        assert!(
            matches!(check(&u), Err(DispatchError::EscapeWindow { .. })),
            "the escape window {w} is not refused as such: {:?}",
            check(&u)
        );
    }
    for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let u = SimUniforms {
            t_horizon: t,
            ..SimUniforms::default()
        };
        assert!(
            matches!(check(&u), Err(DispatchError::HorizonNotFinite { .. })),
            "T_horizon {t} is not refused as such: {:?}",
            check(&u)
        );
    }
    // The smallest window is accepted and is the floor: T at it is accepted.
    let w = f64::MIN_POSITIVE;
    let u = SimUniforms {
        escape_window: w,
        t_horizon: 1.0,
        ..SimUniforms::default()
    };
    assert_eq!(check(&u), Ok(1000), "a positive window is refused");
}

#[test]
fn horizon_refusal_other_refusals() {
    check_other_refusals(&real);
}

negative_control!(
    horizon_refusal_other_refusals,
    "a dispatch that checks only the step count lets a zero dt_macro through to an infinite quotient",
    expected = "is not refused as such",
    check_other_refusals(&|u: &SimUniforms| {
        let steps = horizon_steps(u.t_horizon, u.dt_macro);
        if steps > 65535.0 {
            Err(DispatchError::HorizonSteps {
                t_horizon: u.t_horizon,
                dt_macro: u.dt_macro,
                steps,
            })
        } else {
            Ok(steps as u32)
        }
    })
);

/// `bound` accepts `horizon_steps × N_max` up to `2³² − 1` (`65535 × 65537`, and `(2³² − 1) × 1`), and refuses one
/// above (`65535 × 65538`, `(2³² − 1) × 2`, `2¹⁶ × 2¹⁶`).
fn check_total_substeps_bound(bound: &dyn Fn(u32, u32) -> Result<(), DispatchError>) {
    for (h, n) in [(65535, 65537), (u32::MAX, 1), (1, u32::MAX), (65535, 256)] {
        assert_eq!(bound(h, n), Ok(()), "{h} × {n} ≤ 2³² − 1 is refused");
    }
    for (h, n) in [(65535, 65538), (u32::MAX, 2), (65536, 65536)] {
        assert_eq!(
            bound(h, n),
            Err(DispatchError::TotalSubsteps {
                horizon_steps: h,
                n_max: n
            }),
            "{h} × {n} > 2³² − 1 is not refused"
        );
    }
}

#[test]
fn horizon_refusal_total_substeps_bound() {
    check_total_substeps_bound(&check_total_substeps);
    let message = check_total_substeps(65535, 65538).unwrap_err().to_string();
    assert!(
        message.contains("65535 × 65538"),
        "the refusal's reason: {message}"
    );
}

negative_control!(
    horizon_refusal_total_substeps_bound,
    "a bound on the wrapped u32 product accepts 2¹⁶ × 2¹⁶, which wraps to 0",
    expected = "is not refused",
    check_total_substeps_bound(&|h: u32, n: u32| {
        if h.wrapping_mul(n) == 0 && h != 0 && n != 0 {
            Ok(())
        } else {
            check_total_substeps(h, n)
        }
    })
);

// ── collision_softening_defaults (REQ-SYS-018) ───────────────────────────────────────────────────────────────────

/// The default `u` has `r_coll` nonzero and `epsilon` 0, untagged; each field is one number, constant across the
/// dispatch (fixed at `t = 0`), and there is none other, so no co-moving option; `epsilon > 0` is tagged `softened`
/// in the provenance and on the sim key, whose bytes then differ from the unsoftened run's.
fn check_collision_softening(u: &SimUniforms) {
    // Every field, by name: a field added to `SimUniforms` (a co-moving flag or rate among them) fails to compile here.
    let SimUniforms {
        t_horizon: _,
        dt_macro: _,
        n_max: _,
        r_sub: _,
        gamma_sub: _,
        r_coll,
        tau: _,
        escape_window: _,
        r_close: _,
        eps_e: _,
        eps_l: _,
        g: _,
        m_total: _,
        epsilon,
        n_renorm: _,
        delta0: _,
        thresholds: _,
    } = *u;
    assert!(r_coll > 0.0, "r_coll is {r_coll}, not nonzero by default");
    assert_eq!(
        epsilon.to_bits(),
        0.0f64.to_bits(),
        "epsilon is {epsilon}, not 0 by default"
    );
    assert_eq!(
        u.softening(),
        Softening::Unsoftened,
        "the default run is tagged softened"
    );
    assert_eq!(
        u.softening().tag(),
        None,
        "the default run carries a provenance tag"
    );
    let plain = u.sim_key_bytes();
    assert_eq!(
        plain.len(),
        2 * 8 + 4 + 11 * 8 + 1 + 1 + 1,
        "the sim key's bytes"
    );
    assert_eq!(plain.last(), Some(&0), "the unsoftened run's sim-key tag");
    for eps in [1e-3, f64::MIN_POSITIVE] {
        let soft = SimUniforms { epsilon: eps, ..*u };
        assert_eq!(
            soft.softening(),
            Softening::Softened,
            "epsilon {eps} is not tagged"
        );
        assert_eq!(
            soft.softening().tag(),
            Some("softened"),
            "epsilon {eps}'s provenance tag"
        );
        let key = soft.sim_key_bytes();
        assert_eq!(key.last(), Some(&1), "epsilon {eps}'s sim-key tag");
        assert_ne!(key, plain, "epsilon {eps} pools with the unsoftened run");
        assert_eq!(
            validate(&soft).map(|v| v.horizon_steps()),
            Ok(50000),
            "epsilon {eps} is refused"
        );
    }
    // The table is derived, so it adds nothing to the sim key (R-502); every carried value does.
    let other_table = SimUniforms {
        thresholds: pack(&[0; NSUB_TABLE_MAX]),
        ..*u
    };
    assert_eq!(
        other_table.sim_key_bytes(),
        plain,
        "the table is on the sim key"
    );
    let with_benettin = SimUniforms {
        n_renorm: Some(8),
        delta0: Some(1e-8),
        ..*u
    };
    let key = with_benettin.sim_key_bytes();
    assert_eq!(key.len(), plain.len() + 4 + 8, "n_renorm's and δ₀'s bytes");
    assert_ne!(key, plain, "n_renorm and δ₀ are off the sim key");
}

#[test]
fn collision_softening_defaults() {
    check_collision_softening(&SimUniforms::default());
}

negative_control!(
    collision_softening_defaults,
    "a softened default is not REQ-SYS-018's",
    expected = "not 0 by default",
    check_collision_softening(&SimUniforms {
        epsilon: 1e-3,
        ..SimUniforms::default()
    })
);

/// Each sim-key value moves the sim key's bytes: a change to any field but the table changes them.
#[test]
fn collision_softening_defaults_every_value_on_the_sim_key() {
    check_every_value_keyed(&SimUniforms::sim_key_bytes);
}

/// `key_of` changes for a change to each carried value.
fn check_every_value_keyed(key_of: &dyn Fn(&SimUniforms) -> Vec<u8>) {
    let d = SimUniforms::default();
    let base = key_of(&d);
    let changed: [(&str, SimUniforms); 16] = [
        (
            "t_horizon",
            SimUniforms {
                t_horizon: 49.0,
                ..d
            },
        ),
        (
            "dt_macro",
            SimUniforms {
                dt_macro: 2e-3,
                ..d
            },
        ),
        ("n_max", SimUniforms { n_max: 65, ..d }),
        ("r_sub", SimUniforms { r_sub: 0.06, ..d }),
        (
            "gamma_sub",
            SimUniforms {
                gamma_sub: 1.6,
                ..d
            },
        ),
        ("r_coll", SimUniforms { r_coll: 2e-3, ..d }),
        ("tau", SimUniforms { tau: 2e-3, ..d }),
        (
            "escape_window",
            SimUniforms {
                escape_window: 0.5,
                ..d
            },
        ),
        ("r_close", SimUniforms { r_close: 0.02, ..d }),
        ("eps_e", SimUniforms { eps_e: 2e-6, ..d }),
        ("eps_l", SimUniforms { eps_l: 2e-6, ..d }),
        ("g", SimUniforms { g: 2.0, ..d }),
        ("m_total", SimUniforms { m_total: 2.0, ..d }),
        ("epsilon", SimUniforms { epsilon: 1e-3, ..d }),
        (
            "n_renorm",
            SimUniforms {
                n_renorm: Some(8),
                ..d
            },
        ),
        (
            "delta0",
            SimUniforms {
                delta0: Some(1e-8),
                ..d
            },
        ),
    ];
    for (name, u) in changed {
        assert_ne!(key_of(&u), base, "{name} is off the sim key");
    }
}

negative_control!(
    collision_softening_defaults_every_value_on_the_sim_key,
    "a sim key of the horizon alone misses the rest",
    expected = "is off the sim key",
    check_every_value_keyed(&|u: &SimUniforms| u.t_horizon.to_bits().to_be_bytes().to_vec())
);

// ── sim_uniforms_nsub_table (REQ-INT-028, in part; R-502) ────────────────────────────────────────────────────────

/// `u`'s table is the host generator's for its `r_sub`, `gamma_sub` and `N_max`, bit for bit.
fn check_table_is_generators(u: &SimUniforms) {
    let law = NsubConfig {
        r_sub: u.r_sub,
        gamma_sub: u.gamma_sub,
        n_max: u.n_max,
    };
    let generated = threshold_bits(&law).expect("the config generates");
    assert_eq!(
        bits(&u.thresholds),
        generated,
        "the table SimUniforms carries is not the generator's for {law:?}"
    );
}

/// A non-default config: `r_sub`, `gamma_sub` and `N_max` each changed, `N_max` not a power of two.
const CHANGED: NsubConfig = NsubConfig {
    r_sub: 0.08,
    gamma_sub: 2.0,
    n_max: 100,
};

#[test]
fn sim_uniforms_nsub_table() {
    let u = SimUniforms::default()
        .with_substep_law(CHANGED.r_sub, CHANGED.gamma_sub, CHANGED.n_max)
        .expect("the changed config generates");
    assert_eq!(u.substep_law(), CHANGED, "the law carried");
    check_table_is_generators(&u);
    assert_ne!(
        bits(&u.thresholds),
        DEFAULT_THRESHOLD_BITS,
        "the changed table is the default's"
    );
    assert_eq!(real(&u), Ok(50000), "the changed config is refused");
    check_table_is_generators(&SimUniforms::default());

    // N_max = L_max is accepted; L_max + 1 is the generator's refusal, from the builder and from dispatch.
    let l_max = NSUB_TABLE_MAX as u32;
    let top = SimUniforms::default()
        .with_substep_law(CHANGED.r_sub, CHANGED.gamma_sub, l_max)
        .expect("N_max = L_max generates");
    assert_eq!(real(&top), Ok(50000), "N_max = L_max is refused");
    let refusal = NsubTableError::NMaxAboveTable { n_max: l_max + 1 };
    assert_eq!(
        SimUniforms::default().with_substep_law(CHANGED.r_sub, CHANGED.gamma_sub, l_max + 1),
        Err(refusal),
        "the builder at N_max = L_max + 1"
    );
    let over = SimUniforms {
        n_max: l_max + 1,
        ..top
    };
    let launched = Cell::new(0);
    let result = dispatch(&over, |_| launched.set(launched.get() + 1));
    assert_eq!(
        result,
        Err(DispatchError::Nsub(refusal)),
        "dispatch at N_max = L_max + 1"
    );
    assert_eq!(launched.get(), 0, "a refused sim key launched");
    assert_eq!(
        result.unwrap_err().to_string(),
        refusal.to_string(),
        "the generator's reason"
    );
    assert_eq!(
        real(&SimUniforms { n_max: 0, ..top }),
        Err(DispatchError::Nsub(NsubTableError::NMaxBelowOne)),
        "dispatch at N_max = 0"
    );

    // A table left from another config is refused at dispatch.
    let stale = SimUniforms {
        thresholds: pack(&DEFAULT_THRESHOLD_BITS),
        ..u
    };
    assert_eq!(
        real(&stale),
        Err(DispatchError::StaleThresholds),
        "a stale table is dispatched"
    );
}

negative_control!(
    sim_uniforms_nsub_table,
    "a SimUniforms with the default table under the changed config is not the generator's",
    expected = "is not the generator's for",
    check_table_is_generators(&SimUniforms {
        r_sub: CHANGED.r_sub,
        gamma_sub: CHANGED.gamma_sub,
        n_max: CHANGED.n_max,
        ..SimUniforms::default()
    })
);

// ── horizon_default (REQ-INT-081) ────────────────────────────────────────────────────────────────────────────────

/// `check` accepts the default horizon, 50 at `10⁻³` (50000 steps), `T = 200` at `200/65535` (65535 steps), and
/// `T = 10`, 0.5 and 0.4 at `10⁻³`.
fn check_horizon_lowered(check: &Check) {
    let d = SimUniforms::default();
    assert_eq!(
        (d.t_horizon, d.dt_macro),
        (50.0, 1e-3),
        "the default horizon"
    );
    assert_eq!(check(&d), Ok(50000), "the default horizon's steps");
    let top = at(200.0);
    assert_eq!(top.dt_macro, 200.0 / 65535.0, "dt_macro at T = 200");
    assert_eq!(check(&top), Ok(65535), "T = 200's steps");
    for (t, steps) in [(10.0, 10000), (0.5, 500), (0.4, 400)] {
        let u = at(t);
        assert_eq!(u.dt_macro, 1e-3, "dt_macro at T = {t}");
        assert_eq!(
            check(&u),
            Ok(steps),
            "T = {t} is not accepted at {steps} steps"
        );
    }
}

/// `check` refuses no `T` in `[0.4, 200]` (a grid of 0.05, its ends and their neighbours inside, and both sides of
/// 65.535), and refuses 0.39, 0, −1 and the f64 just below 0.4 with R-610's reason, and the f64 just above 200.
fn check_horizon_floor(check: &Check) {
    let mut ts: Vec<f64> = (8..=4000).map(|k| f64::from(k) * 0.05).collect();
    ts.extend([
        0.4,
        0.4f64.next_up(),
        200.0,
        200f64.next_down(),
        65.535,
        65.535f64.next_up(),
        65.538,
    ]);
    for t in ts {
        let u = at(t);
        let steps = check(&u).unwrap_or_else(|e| panic!("T = {t} in [0.4, 200] is refused: {e}"));
        assert!((1..=65535).contains(&steps), "T = {t}: {steps} steps");
    }
    for t in [0.39, 0.0, -1.0, 0.4f64.next_down()] {
        match check(&at(t)) {
            Err(e @ DispatchError::HorizonBelowWindow { .. }) => {
                let reason = e.to_string();
                assert!(
                    reason.contains("below one closure window (0.4) the escape arm cannot fire"),
                    "T = {t}'s reason: {reason}"
                );
            }
            other => panic!("T = {t}, below one closure window, is not refused: {other:?}"),
        }
    }
    let over = 200f64.next_up();
    assert!(
        matches!(check(&at(over)), Err(DispatchError::HorizonAboveCap { .. })),
        "T = {over}, above the cap, is not refused: {:?}",
        check(&at(over))
    );
}

#[test]
fn horizon_default() {
    check_horizon_lowered(&real);
}

negative_control!(
    horizon_default,
    "a floor at 50, the range before R-575, refuses T = 10",
    expected = "T = 10 is not accepted",
    check_horizon_lowered(&|u: &SimUniforms| {
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

#[test]
fn horizon_default_floor_is_one_window() {
    check_horizon_floor(&real);
}

negative_control!(
    horizon_default_floor_is_one_window,
    "the interim range (0, 200], R-575's, accepts 0.39",
    expected = "T = 0.39, below one closure window, is not refused",
    check_horizon_floor(&|u: &SimUniforms| {
        if u.t_horizon > 0.0 && u.t_horizon < 0.4 {
            Ok(horizon_steps(u.t_horizon, u.dt_macro) as u32)
        } else {
            real(u)
        }
    })
);
