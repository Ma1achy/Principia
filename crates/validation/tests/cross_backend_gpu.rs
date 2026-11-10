//! The record on the session's adapter (`cross_backend::record_gpu`; TASK-M4-27; REQ-VAL-347): the rows of a run come back
//! under their entries, one per input and branch, keyed as the record says. It reads no rate, and asserts no agreement
//! between backends: that is the measurement (R-649). `cargo xtask build-kernel` runs first, as CI runs it. Each test
//! registers the control that must make it fail (R-176).

use std::path::Path;

use engine::decode_dispatch::uniform_words;
use kernel::chart::Latent;
use kernel::decode::Ic;
use kernel::stage::decode::{DecodeUniforms, DecodeVariant, LatentDefaults, UniformChart};
use validation::cross_backend::{record_gpu, Batch, Inputs, Part, Record, CANON_ENTRY};
use validation::negative_control;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn inputs() -> Inputs {
    let eight = |k: f32| core::array::from_fn(|i| k * (i as f32 - 3.5) / 8.0);
    let chart = Latent {
        z0: eight(1.0),
        q1: eight(0.5),
        q2: eight(-0.25),
    };
    let u = DecodeUniforms {
        chart: chart.uniform(),
        ..Default::default()
    };
    let points = [[0.5f32, 0.5], [0.25, 0.75], [0.125, 0.875]];
    Inputs {
        canon: vec![Ic {
            m: [0.2, 0.3, 0.5],
            r: [[0.1, 0.2], [0.7, -0.3], [-0.4, 0.9]],
            p: [[0.1, -0.2], [0.3, 0.05], [-0.4, 0.15]],
        }],
        parts: vec![Part {
            name: "one".to_owned(),
            count: 1,
        }],
        batches: vec![Batch {
            entry: LatentDefaults::ENTRY.to_owned(),
            label: "a chart".to_owned(),
            uniform: uniform_words(&u)[..24].to_vec(),
            e_star: Some((-0.3f32).to_bits()),
            points: points.len(),
        }],
        points: points.iter().map(|p| p.map(f32::to_bits)).collect(),
    }
}

fn check_rows(r: &Record, entry: &str, rows: usize) {
    let g = r
        .groups
        .iter()
        .find(|g| g.entry == entry && g.branch == "M01_TINY");
    assert_eq!(
        g.map_or(0, |g| g.len()),
        rows,
        "a run's rows are not one per input"
    );
}

#[test]
fn cross_backend_gpu_record_holds_its_rows() {
    let inputs = inputs();
    let r = record_gpu(root(), &inputs, "a commit").expect("a GPU device");
    assert_eq!((r.commit.as_str(), r.inputs), ("a commit", 4));
    assert_eq!(r.inputs_hash, inputs.hash());
    assert_eq!(r.canon_operands.len(), 9);
    assert!(r.groups.iter().any(|g| g.entry == CANON_ENTRY));
    assert_eq!(r.fast_math, "setting off; compute compiled off");
    assert!(!r.adapter.is_empty());
    check_rows(&r, "stage_branch_read_latent_defaults", 3);
}

negative_control!(
    cross_backend_gpu_record_holds_its_rows,
    "rows asked of an entry the run did not run",
    expected = "a run's rows are not one per input",
    {
        let r = record_gpu(root(), &inputs(), "a commit").expect("a GPU device");
        check_rows(&r, "stage_branch_read_burrau_defaults", 3);
    }
);
