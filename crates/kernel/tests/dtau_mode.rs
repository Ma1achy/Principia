//! The dτ mode of Heggie's and Aarseth–Zare's step control (`step_size_in`; the watches' `PER_STEP`): the
//! step-limit ablation's `dtau` switch (pitfalls §8).
//!
//! Criteria, written before the run: on the registered figure-eight, with the limit and the clamp off and an entry size
//! twice the state's own per-step size `c₀`, the per-step mode sizes the step at `c₀`, below the entry size, and the
//! fixed mode at the entry size exactly; and `step_size`, the occupant's, is `step_size_in` in the per-step mode, bit
//! for bit, with the limit and the clamp on. The control reads the per-step size as the fixed mode's.

use kernel::occupant::State;
use kernel::regularisation::az::{self, AzParams, AzSystem};
use kernel::regularisation::heggie::{self as hg, HgParams, DEFAULT_TIME};
use validation::negative_control;

fn state() -> State<f64> {
    let ic = validation::drift::figure_eight().ic;
    State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    }
}

fn heggie(limit_and_clamp: bool) -> HgParams<f64> {
    HgParams {
        eta: 0.01,
        delta_sync: 0.125,
        tiny: 1e-300,
        predictive_f: if limit_and_clamp { 0.02 } else { 0.0 },
        step_budget_rate: 1e7,
        n_min: 1_000_000,
        clamp: limit_and_clamp,
        secant_tries: 4,
        bisect_tries: 64,
        time: DEFAULT_TIME,
    }
}

fn aarseth_zare(limit_and_clamp: bool) -> AzParams<f64> {
    let h = heggie(limit_and_clamp);
    AzParams {
        eta: h.eta,
        delta_sync: h.delta_sync,
        tiny: h.tiny,
        predictive_f: h.predictive_f,
        step_budget_rate: h.step_budget_rate,
        n_min: h.n_min,
        clamp: h.clamp,
        secant_tries: h.secant_tries,
        bisect_tries: h.bisect_tries,
    }
}

/// `(per-step h, fixed h, c₀, entry)` for Heggie and for Aarseth–Zare, the limit and the clamp off.
fn sizes() -> [(f64, f64, f64, f64); 2] {
    let s = state();
    let reg = hg::register(&s, 1e-300, DEFAULT_TIME);
    let p = heggie(false);
    let c0 = p.eta * p.delta_sync / reg.system.dt_dtau(&reg.q);
    let entry = 2.0 * c0;
    let hg_sizes = (
        hg::step_size_in(&reg.system, &reg.q, &reg.p, entry, &p, true).h,
        hg::step_size_in(&reg.system, &reg.q, &reg.p, entry, &p, false).h,
        c0,
        entry,
    );
    let (system, q, pa) = az::register(&s, 1e-300);
    let pz = aarseth_zare(false);
    let (a, b) = AzSystem::ab(&q);
    let c0 = pz.eta * pz.delta_sync / (a * b);
    let entry = 2.0 * c0;
    let az_sizes = (
        az::step_size_in(&system, &q, &pa, entry, &pz, true).h,
        az::step_size_in(&system, &q, &pa, entry, &pz, false).h,
        c0,
        entry,
    );
    [hg_sizes, az_sizes]
}

fn check_modes(read: impl Fn((f64, f64, f64, f64)) -> (f64, f64, f64, f64)) {
    for (name, sz) in ["Heggie", "Aarseth-Zare"].into_iter().zip(sizes()) {
        let (per_step, fixed, c0, entry) = read(sz);
        assert_eq!(
            per_step.to_bits(),
            c0.to_bits(),
            "{name}: the per-step size is not c0"
        );
        assert!(
            per_step < entry,
            "{name}: the per-step size is not below the entry size"
        );
        assert_eq!(
            fixed.to_bits(),
            entry.to_bits(),
            "{name}: the fixed mode's size is not the entry size"
        );
    }
}

#[test]
fn the_fixed_mode_sizes_at_the_entry() {
    check_modes(|s| s);
}

negative_control!(
    the_fixed_mode_sizes_at_the_entry,
    "the per-step size read as the fixed mode's",
    expected = "the fixed mode's size is not the entry size",
    check_modes(|(per_step, _, c0, entry)| (per_step, per_step, c0, entry))
);

/// `step_size` is `step_size_in`'s mode `per_step`, bit for bit, the limit and the clamp on, at the registration, at
/// an entry size below `c₀` (a cap hit) and at twice `c₀`.
fn check_occupants_size(per_step: bool) {
    let s = state();
    let reg = hg::register(&s, 1e-300, DEFAULT_TIME);
    let p = heggie(true);
    let c0 = p.eta * p.delta_sync / reg.system.dt_dtau(&reg.q);
    for entry in [c0 / 2.0, 2.0 * c0] {
        assert_eq!(
            hg::step_size(&reg.system, &reg.q, &reg.p, entry, &p),
            hg::step_size_in(&reg.system, &reg.q, &reg.p, entry, &p, per_step),
            "Heggie: the occupant's size is not the mode's"
        );
    }
    let (system, q, pa) = az::register(&s, 1e-300);
    let pz = aarseth_zare(true);
    let (a, b) = AzSystem::ab(&q);
    let c0 = pz.eta * pz.delta_sync / (a * b);
    for entry in [c0 / 2.0, 2.0 * c0] {
        assert_eq!(
            az::step_size(&system, &q, &pa, entry, &pz),
            az::step_size_in(&system, &q, &pa, entry, &pz, per_step),
            "Aarseth-Zare: the occupant's size is not the mode's"
        );
    }
}

#[test]
fn the_occupants_size_is_the_per_step_mode() {
    check_occupants_size(true);
}

negative_control!(
    the_occupants_size_is_the_per_step_mode,
    "the occupant's size read against the fixed mode",
    expected = "the occupant's size is not the mode's",
    check_occupants_size(false)
);
