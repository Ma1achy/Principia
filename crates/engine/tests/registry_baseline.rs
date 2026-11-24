//! The kernel charts `fixtures/chart_composition/digest.json` holds no case of, recorded from today's code before the
//! primitive registry (TASK-M2-66) so the registry's `presets_bitwise` has a baseline for every chart of
//! `kernel::chart::REGISTRY` (`fixtures/chart_composition/registry_charts.json`).
//!
//! `digest.json` reaches the latent chart (the presets and the saved latent charts), `InvariantLE`, the shape sphere
//! and `Mixed`, through the chart switch. The other seven charts of the registry are recorded here: each one's
//! representative from `validation::roundtrip::charts`, the parameters `well_posedness.rs` registers it with, its
//! decoded states at `path_support`'s 3 × 3 probe of the unit square as one FNV-1a 64 word (`path_support::chart_ic`,
//! the digest's own `ic=` word). `InvariantLK` takes `path_support::invariant_lk`, the chart of the lock's tests.
//!
//! **The criterion**, written before the first run: each chart's word equals the recorded one exactly, and every
//! recorded chart is produced. The file is written only by hand (`PRINCIPIA_BLESS_REGISTRY_BASELINE=1`), once, on the
//! commit before the registry; after that a different word is a finding to report, never a re-record.
//!
//! **Control**: one mantissa bit of one decoded momentum flipped before it is hashed. It must fail.

mod path_support;

use std::collections::BTreeMap;

use engine::contract::sim_config::Window;
use kernel::chart::ChartDecode;
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use path_support::{fnv, ic_words, invariant_lk, root, PROBE};
use validation::negative_control;
use validation::roundtrip::charts;

/// The baseline, relative to the workspace root.
const BASELINE: &str = "fixtures/chart_composition/registry_charts.json";

/// The words of `chart`'s decoded states at the probe through `window`, with `flip` applied to each state first.
fn word<C>(chart: &C, window: &Window, flip: fn(&mut Ic<f64>)) -> String
where
    C: ChartDecode,
    f64: From<C::Param>,
{
    let mut bytes = Vec::new();
    for t in PROBE {
        for s in PROBE {
            let [u, v] = window.at(s, t);
            let mut ic = chart.decode_at::<Defaults, f64>(u, v).ic;
            flip(&mut ic);
            ic_words(&ic, &mut bytes);
        }
    }
    format!("ic={:016x}", fnv(&bytes))
}

/// No change.
fn keep(_: &mut Ic<f64>) {}

/// The low mantissa bit of body 0's `p_x` flipped (the control).
#[cfg(feature = "controls")]
fn flip_bit(ic: &mut Ic<f64>) {
    ic.p[0][0] = f64::from_bits(ic.p[0][0].to_bits() ^ 1);
}

/// Each uncovered chart of `kernel::chart::REGISTRY`, by its name, and its word.
pub fn words(flip: fn(&mut Ic<f64>)) -> BTreeMap<String, String> {
    let w = Window::SQUARE;
    BTreeMap::from([
        ("Burrau".into(), word(&charts::burrau::<f64>(), &w, flip)),
        (
            "EuclidPlane".into(),
            word(&charts::euclid_plane::<f64>(), &w, flip),
        ),
        (
            "BurrauThetaK".into(),
            word(&charts::burrau_theta_k::<f64>(), &w, flip),
        ),
        (
            "BurrauThetaLz".into(),
            word(&charts::burrau_theta_lz::<f64>(), &w, flip),
        ),
        (
            "BurrauThetaDeltaM".into(),
            word(&charts::burrau_theta_delta_m::<f64>(), &w, flip),
        ),
        ("InvariantLK".into(), word(&invariant_lk(), &w, flip)),
        (
            "MassSimplex".into(),
            word(&charts::mass_simplex::<f64>(), &w, flip),
        ),
    ])
}

/// The recorded baseline.
pub fn recorded() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root().join(BASELINE)).expect("the registry baseline");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    serde_json::from_value(value["charts"].clone()).expect("the charts")
}

fn check(flip: fn(&mut Ic<f64>), plain: bool) {
    let actual = words(flip);
    // Only the plain arm records: a control arm, run under the same variable, never writes its flipped words.
    if std::env::var_os("PRINCIPIA_BLESS_REGISTRY_BASELINE").is_some() && plain {
        let value = serde_json::json!({
            "what": "The kernel charts digest.json holds no case of, recorded from today's code before the primitive registry (TASK-M2-66): each chart's decoded states at path_support's 3 x 3 probe of the unit square, FNV-1a 64 (crates/engine/tests/registry_baseline.rs).",
            "command": "PRINCIPIA_BLESS_REGISTRY_BASELINE=1 cargo test -p engine --test registry_baseline",
            "charts": actual,
        });
        let text = serde_json::to_string_pretty(&value).expect("JSON") + "\n";
        std::fs::write(root().join(BASELINE), text).expect("the registry baseline written");
        return;
    }
    let expected = recorded();
    assert_eq!(
        expected.keys().collect::<Vec<_>>(),
        actual.keys().collect::<Vec<_>>(),
        "the registry baseline's charts"
    );
    for (name, want) in &expected {
        assert_eq!(
            &actual[name], want,
            "{name}: not bit for bit with the registry baseline"
        );
    }
}

#[test]
fn registry_baseline_bitwise() {
    check(keep, true);
}

negative_control!(
    registry_baseline_bitwise,
    "one mantissa bit of a decoded momentum flipped before hashing",
    expected = "not bit for bit with the registry baseline",
    check(flip_bit, false)
);
