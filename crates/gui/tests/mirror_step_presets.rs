//! The gui's chart presets and TASK-M2-57's mirror-step removal (REQ-DEC-055; R-865, R-898, R-919).
//!
//! The baseline, `fixtures/mirror_step_baseline/gui.json`, was written once on the task's baseline commit (3c5fb6375)
//! through its decoder, by the ignored writer this file held there: each of [`PRESETS`] at its home
//! ([`PRESET_HOMES`]), its sim key and its IC words over a 9 × 9 grid. The gui may not depend on the kernel
//! (systems_architecture §7.1; `cargo xtask deps`), so the decode half of the conversion test runs in the kernel
//! (`crates/kernel/tests/mirror_step_presets.rs`), which writes and checks
//! `fixtures/mirror_step_baseline/gui_converted.json`: each preset's plane with its base point's `z_β` converted from
//! `sigmoid_beta` (R-898), and whether it runs along `z_β`, redefined as an affine plane in the new coordinates
//! (R-919 Q2 (b)). Applied per R-369: the test's wiring. *Was:* the writer, here, with a `kernel` dev-dependency.
//!
//! - `mirror_step_presets_converted` (REQ-DEC-055): every preset's plane at its home, `z₀` its base point and
//!   `q₁ = σ_h·e_a`, `q₂ = σ_h·e_b`, equals `gui_converted.json`'s, bit for bit, by name and in order. Its control, in
//!   itself (gui's tests have no route to `negative_control!`, R-187): the home with the base point's `z_β` as the
//!   baseline held it, unconverted, fails.

use std::path::{Path, PathBuf};

use engine::contract::sim_config::Plane;
use gui::explore::manifold_view::{axis, AXES, PRESETS, PRESET_HOMES};
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/mirror_step_baseline")
        .join(name);
    let text = std::fs::read_to_string(path).expect("the fixture reads");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

/// Preset `i`'s plane at its home.
fn home(i: usize) -> Plane {
    let (a, b) = PRESETS[i];
    let h = PRESET_HOMES[i];
    Plane {
        z0: h.base,
        q1: axis(a, h.window),
        q2: axis(b, h.window),
    }
}

/// Checks `plane(i)` against `gui_converted.json`'s preset `i`, every preset, bit for bit.
fn check_converted(plane: impl Fn(usize) -> Plane) {
    let file = fixture("gui_converted.json");
    let items = file["items"].as_array().expect("items");
    assert_eq!(items.len(), PRESETS.len(), "a converted plane per preset");
    for (i, item) in items.iter().enumerate() {
        let (a, b) = PRESETS[i];
        let name = format!("{} × {}", AXES[a], AXES[b]);
        assert_eq!(item["name"], name.as_str(), "preset {i}'s name");
        let p = plane(i);
        for (key, got) in [("z0", p.z0), ("q1", p.q1), ("q2", p.q2)] {
            let want: Vec<&str> = item["plane"][key]
                .as_array()
                .expect("a vector")
                .iter()
                .map(|v| v.as_str().expect("hex"))
                .collect();
            assert_eq!(
                got.map(hex).to_vec(),
                want,
                "{name}: {key} is not the converted plane's"
            );
        }
    }
}

/// `check` must fail: the control's half, run in the test itself.
fn rejects(what: &str, check: impl FnOnce() + std::panic::UnwindSafe) {
    assert!(
        std::panic::catch_unwind(check).is_err(),
        "the control passes: {what}"
    );
}

#[test]
fn mirror_step_presets_converted() {
    check_converted(home);
    rejects("the base point's z_β unconverted", || {
        let baseline = fixture("gui.json");
        check_converted(|i| {
            let mut p = home(i);
            let old = baseline["items"][i]["plane"]["z0"][1]
                .as_str()
                .expect("hex");
            p.z0[1] = f64::from_bits(u64::from_str_radix(old, 16).expect("hex"));
            p
        });
    });
}
