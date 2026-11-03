//! The physical-quantity axes' mapping on the sim key (R-471 E2, H7; R-506; REQ-CHART-068; TASK-M2-37):
//! - `energy_axis_linear_option_sim_key`: each axis's `warped | linear` mapping is a field of `SimConfig`'s chart
//!   group, `warped` by default; switching either axis's mapping changes the sim key and its canonical text, which
//!   names it, and the text reads back to the same key.
//! - `energy_axis_linear_option_kernel_mapping`: the engine's mapping is the kernel's chart parameter of the same name.

use engine::contract::canonical;
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Mapping, Plane, Quality,
    SimConfig, Slice, SliceFrame, Window, Z_Q3,
};
use validation::negative_control;

/// The sim key with the axes' mappings `mapping`, every other group as at M0.
fn config(mapping: [Mapping; 2]) -> SimConfig {
    SimConfig {
        chart: Chart {
            window: Window::SQUARE,
            mapping,
        },
        plane: Plane {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
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

/// Switching `to` from the default changes the sim key and its canonical text, which carries `to`'s mapping by name.
fn check_switch(to: [Mapping; 2]) {
    let warped = config(Default::default());
    let switched = config(to);
    assert!(
        switched != warped && text(&switched) != text(&warped),
        "the mapping {to:?} keeps the sim key"
    );
    let name = |m: Mapping| match m {
        Mapping::Warped => "\"warped\"",
        Mapping::Linear => "\"linear\"",
    };
    let field = format!("\"mapping\":[{},{}]", name(to[0]), name(to[1]));
    assert!(
        text(&switched).contains(&field),
        "the text {} lacks {field}",
        text(&switched)
    );
    let back: SimConfig = serde_json::from_str(&text(&switched)).expect("the text reads back");
    assert_eq!(back, switched, "the text reads back to another key");
}

#[test]
fn energy_axis_linear_option_sim_key() {
    assert_eq!(
        config(Default::default()).chart.mapping,
        [Mapping::Warped; 2],
        "the default mapping"
    );
    check_switch([Mapping::Linear, Mapping::Warped]);
    check_switch([Mapping::Warped, Mapping::Linear]);
    check_switch([Mapping::Linear, Mapping::Linear]);
}

negative_control!(
    energy_axis_linear_option_sim_key,
    "a mapping switched to the default keeps the sim key",
    expected = "keeps the sim key",
    check_switch([Mapping::Warped, Mapping::Warped])
);

#[test]
fn energy_axis_linear_option_kernel_mapping() {
    assert_eq!(
        kernel::chart::Mapping::from(Mapping::Warped),
        kernel::chart::Mapping::Warped
    );
    assert_eq!(
        kernel::chart::Mapping::from(Mapping::Linear),
        kernel::chart::Mapping::Linear
    );
}

negative_control!(
    energy_axis_linear_option_kernel_mapping,
    "the linear mapping read as the kernel's warp fails",
    expected = "assertion",
    assert_eq!(
        kernel::chart::Mapping::from(Mapping::Linear),
        kernel::chart::Mapping::Warped
    )
);
