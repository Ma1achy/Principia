//! qa's tests for TASK-M2-37's engine side, written from REQ-CHART-068 and REQ-CHART-070, each with its negative control
//! (R-176):
//!
//! - `qa_energy_axis_linear_option_sim_key` (REQ-CHART-068: "a chart parameter (warped | linear) in SimConfig, a sim
//!   key"; R-471: "recorded in the view state and the sidecar"): each axis's mapping defaults to `warped`; switching
//!   either axis to `linear` changes the `SimConfig` and its canonical text, the sim key's serialisation, which names
//!   the mapping; the ViewState record (`view_state::export`) carries it and restores the same key. Control: a
//!   "switch" that keeps the mapping keeps the key.
//! - `qa_pe_axis_refused_by_validator` (REQ-CHART-070: "a chart offering PE … is refused by the well-posedness
//!   validator"; chart_reference §2.1, "The `PE` axis: no chart offers it"): a chart declaring a `PE` axis is refused
//!   for its warp, wherever its other declarations stand. Control: the same chart with a `K` axis in its place is not
//!   refused for its warp.
//!
//! Every view and descriptor here is test data.

use std::collections::BTreeSet;

use engine::chart_authoring::axis::COORDINATES;
use engine::chart_authoring::{
    validate, Authored, AxisDescriptor, Block, ChartDescriptor, Coupling, Frozen, InvariantMeta,
    InvariantQuantity, Refusal,
};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Mapping, Plane, Quality,
    SimConfig, Slice, SliceFrame, Window, Z_Q3,
};
use engine::contract::{canonical, view_state};
use kernel::chart::mixed::Coordinate;
use kernel::chart::{ChartOut, Float, SystemImage, ValidationResult};
use kernel::decode::Latent;
use validation::negative_control;

/// A view with the axes' mappings `mapping`, every other group off its zero where that is free test data.
fn config(mapping: [Mapping; 2]) -> SimConfig {
    SimConfig {
        chart: Chart {
            window: Window {
                centre: [0.4, 0.55],
                half: [0.3, 0.25],
            },
            mapping,
        },
        plane: Plane {
            z0: [0.1, -0.2, 0.0, 0.3, 0.0, 0.0, 0.2, -0.1],
            q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 0.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

fn text(c: &SimConfig) -> String {
    canonical::to_string(c).expect("a canonical form")
}

const ALL: [[Mapping; 2]; 4] = [
    [Mapping::Warped, Mapping::Warped],
    [Mapping::Linear, Mapping::Warped],
    [Mapping::Warped, Mapping::Linear],
    [Mapping::Linear, Mapping::Linear],
];

/// The four mappings' sim keys, after `switch` maps each: distinct as configs and as canonical texts, each text naming
/// its mapping, and each ViewState record restoring its config.
fn check_keys(switch: impl Fn([Mapping; 2]) -> [Mapping; 2]) {
    let configs: Vec<SimConfig> = ALL.iter().map(|&m| config(switch(m))).collect();
    let texts: BTreeSet<String> = configs.iter().map(text).collect();
    assert!(
        texts.len() == ALL.len(),
        "a mapping change keeps the sim key: {} keys for {} mappings",
        texts.len(),
        ALL.len()
    );
    for (i, a) in configs.iter().enumerate() {
        for b in &configs[i + 1..] {
            assert!(
                a != b,
                "a mapping change keeps the sim key: {:?}",
                a.chart.mapping
            );
        }
        let name = |m: Mapping| match m {
            Mapping::Warped => "warped",
            Mapping::Linear => "linear",
        };
        let [h, v] = a.chart.mapping;
        let field = format!("\"mapping\":[\"{}\",\"{}\"]", name(h), name(v));
        assert!(
            text(a).contains(&field),
            "the sim key {} lacks {field}",
            text(a)
        );
        let record = view_state::export(a).expect("a record");
        assert!(
            record.contains(&field),
            "the ViewState record lacks {field}"
        );
        let back = view_state::restore(&record).expect("the record restores");
        assert!(
            back == *a && text(&back) == text(a),
            "the record restores another key"
        );
    }
}

#[test]
fn qa_energy_axis_linear_option_sim_key() {
    assert_eq!(
        <[Mapping; 2]>::default(),
        [Mapping::Warped; 2],
        "the default mapping"
    );
    assert_eq!(Mapping::default(), Mapping::Warped);
    check_keys(|m| m);
    for (m, k) in [
        (Mapping::Warped, kernel::chart::Mapping::Warped),
        (Mapping::Linear, kernel::chart::Mapping::Linear),
    ] {
        assert_eq!(
            kernel::chart::Mapping::from(m),
            k,
            "the kernel's mapping for {m:?}"
        );
    }
}

negative_control!(
    qa_energy_axis_linear_option_sim_key,
    "a switch of the vertical axis's mapping that keeps it keeps the sim key",
    expected = "keeps the sim key",
    check_keys(|[h, _]| [h, Mapping::Warped])
);

// ── REQ-CHART-070 ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A chart whose descriptor is the test's; the validator reads only the descriptor.
#[derive(Clone, Debug)]
struct Probe(ChartDescriptor);

impl kernel::chart::Chart for Probe {
    type Param = f64;

    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> ChartOut<F> {
        let c = <F as From<f64>>::from(0.1);
        ChartOut {
            z: Latent::new([u, c, c, c, c, c, v, c]),
        }
    }

    fn name(&self) -> &str {
        "Probe"
    }

    fn validate(&self, _u: f64, _v: f64) -> ValidationResult {
        ValidationResult::Valid
    }
}

impl Authored for Probe {
    fn descriptor(&self) -> ChartDescriptor {
        self.0.clone()
    }
}

/// A swept `z_μ₁` beside an invariant axis of `quantity` solving into the configuration block, reading the masses: a
/// potential-energy axis's shape (chart_reference §2.1: "a `PE` axis would solve into the configuration block").
fn config_axis_chart(quantity: InvariantQuantity) -> Probe {
    let axes = [
        AxisDescriptor::coordinate("z_mu1", Coordinate::ZMu1),
        AxisDescriptor::invariant(
            "U",
            InvariantMeta::new(quantity, Block::Config, Block::Mass.only()),
        ),
    ];
    let slice = COORDINATES
        .into_iter()
        .filter(|c| !matches!(c, Coordinate::ZMu1 | Coordinate::ZAlpha))
        .map(Frozen::Coordinate)
        .collect();
    Probe(ChartDescriptor {
        annotation: false,
        has_feasibility_boundary: true,
        axes,
        slice,
        conventions: vec![],
        system_image: SystemImage::Bijective,
        coupling: Coupling::none(),
    })
}

/// The chart with an axis of `quantity` is refused because chart_reference §2.1 writes no warp for it.
fn check_refused_for_warp(quantity: InvariantQuantity) {
    match validate(config_axis_chart(quantity), None) {
        Err(Refusal::NoWarpWritten { axis }) => {
            assert_eq!(axis, "U", "the refusal names another axis");
            let message = Refusal::NoWarpWritten { axis }.to_string();
            assert!(
                message.starts_with("refused") && message.contains('U'),
                "the message: {message}"
            );
        }
        other => panic!("the {quantity:?} axis is not refused for its warp: {other:?}"),
    }
}

#[test]
fn qa_pe_axis_refused_by_validator() {
    check_refused_for_warp(InvariantQuantity::PE);
}

negative_control!(
    qa_pe_axis_refused_by_validator,
    "the same chart with a K axis is not refused for its warp",
    expected = "is not refused for its warp",
    check_refused_for_warp(InvariantQuantity::K)
);
