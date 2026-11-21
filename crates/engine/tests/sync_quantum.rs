//! The sync quantum on the engine's side (integrator_contract Part 2a § "The sync grid is a fixed quantum of physical
//! time"; R-565):
//! - `sync_quantum_on_the_sim_key`: `SimUniforms` carries `Δ_sync`, the register's value, the one the kernel's marches
//!   read; a key carrying another value has other sim-key bytes, so a change re-integrates, and dispatch refuses it,
//!   since the kernel cannot march on another grid. Control: a sim key blind to `Δ_sync`.
//! - `sync_quantum_field_stops_on_the_grid`: the field backend marches to the latest boundary at or before the horizon,
//!   never past it: `T = 50` is boundary 400, `T = 0.4` (the horizon's floor, off the 1/8 grid) is boundary 3, at
//!   `t = 0.375`, and `T = 50.1` is 400 again. Control: the ceiling, the boundary after an off-grid horizon.

use engine::contract::sim_uniforms::SimUniforms;
use engine::dispatch::validate::{validate, DispatchError};
use engine::field::{boundaries, heggie_params, HgParams32};
use kernel::regularisation::sync::delta_sync;
use ledger::constants::DELTA_SYNC;
use validation::negative_control;

/// `key_of` is the sim key: the default carries the register's `Δ_sync`, the kernel's, which dispatch accepts; a key
/// with `Δ_sync = 0.1` has other bytes and is refused as such.
fn check_keyed(key_of: &dyn Fn(&SimUniforms) -> Vec<u8>) {
    let d = SimUniforms::default();
    assert_eq!(
        d.delta_sync.to_bits(),
        DELTA_SYNC.number().to_bits(),
        "the default Δ_sync is not the register's"
    );
    assert_eq!(
        d.delta_sync.to_bits(),
        delta_sync::<f64>().to_bits(),
        "the sim key's Δ_sync is not the kernel's"
    );
    assert_eq!(
        heggie_params().delta_sync.to_bits(),
        delta_sync::<f32>().to_bits(),
        "the field backend's Δ_sync is not the kernel's"
    );
    validate(&d).expect("the default sim key is dispatched");
    let other = SimUniforms {
        delta_sync: 0.1,
        ..d
    };
    assert_ne!(key_of(&other), key_of(&d), "Δ_sync is off the sim key");
    for x in [0.1, 0.0, f64::NAN, f64::INFINITY] {
        let got = validate(&SimUniforms { delta_sync: x, ..d }).map(|_| ());
        assert!(
            matches!(got, Err(DispatchError::SyncQuantum { delta_sync }) if delta_sync.to_bits() == x.to_bits()),
            "Δ_sync = {x} is not refused as such: {got:?}"
        );
    }
    println!(
        "Δ_sync = {} on the sim key, the kernel's; 0.1, 0, NaN and ∞ refused",
        d.delta_sync
    );
}

#[test]
fn sync_quantum_on_the_sim_key() {
    check_keyed(&SimUniforms::sim_key_bytes);
}

negative_control!(
    sync_quantum_on_the_sim_key,
    "a sim key blind to Δ_sync misses a change to it",
    expected = "Δ_sync is off the sim key",
    check_keyed(&|u: &SimUniforms| {
        SimUniforms {
            delta_sync: SimUniforms::default().delta_sync,
            ..*u
        }
        .sim_key_bytes()
    })
);

/// `stop` gives the boundary a march to each horizon stops on: the latest at or before it.
fn check_stops(stop: &dyn Fn(f64, &HgParams32) -> u32) {
    let params = heggie_params();
    for (t, want) in [
        (50.0, 400),
        (0.4, 3),
        (50.1, 400),
        (200.0, 1600),
        (0.125, 1),
    ] {
        let got = stop(t, &params);
        println!(
            "T = {t}: boundary {got}, t = {}",
            f64::from(got) * f64::from(params.delta_sync)
        );
        assert_eq!(
            got, want,
            "the march to T = {t} stops on boundary {got}, not the latest at or before it, {want}"
        );
    }
}

#[test]
fn sync_quantum_field_stops_on_the_grid() {
    check_stops(&boundaries);
}

negative_control!(
    sync_quantum_field_stops_on_the_grid,
    "the ceiling marches an off-grid horizon on to the boundary after it",
    expected = "not the latest at or before it",
    check_stops(&|t: f64, p: &HgParams32| (t / f64::from(p.delta_sync)).ceil() as u32)
);
