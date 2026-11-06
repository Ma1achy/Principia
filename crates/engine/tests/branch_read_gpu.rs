//! The read-back's dispatchers (`engine::decode_dispatch`; TASK-M4-27; REQ-VAL-347), on the session's adapter: that the
//! words come back at their stride, canon's operands the input's bits, a sample with every branch's `present` word as the
//! chart and `η_E` say, and an empty input no dispatch. Whether the backends agree is a measurement, `cargo xtask gate
//! cross-backend-branches`, and no test here reads a rate (R-649). Each test registers the control that must make it fail
//! (R-176).

use std::path::Path;

use engine::contract::fast_math::{FastMath, StageMode};
use engine::decode_dispatch::{branch_read_canon_gpu, BranchReadStage, BRANCH_READ_CANON_ENTRY};
use kernel::chart::Latent;
use kernel::decode::Ic;
use kernel::stage::branch_read as read;
use kernel::stage::decode::{
    BurrauDefaults, BurrauThetaDeltaMDefaults, BurrauThetaKDefaults, BurrauThetaLzDefaults,
    EuclidPlaneDefaults, InvariantLEDefaults, InvariantLKDefaults, LatentDefaults,
    LatentStickBreaking, MassSimplexDefaults, MixedMu1AlphaDefaults, ShapeSphereDefaults,
};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn wgsl() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/spirv/kernel.wgsl");
    std::fs::read_to_string(path)
        .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

fn harness() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

fn state(k: f32) -> Ic<f32> {
    Ic {
        m: [0.2, 0.3, 0.5],
        r: [[0.1 + k, 0.2], [0.7, -0.3 - k], [-0.4, 0.9]],
        p: [[0.1, -0.2], [0.3, 0.05], [-0.4, 0.15]],
    }
}

fn operand_words(s: &Ic<f32>) -> Vec<u32> {
    let m = s.m.iter().map(|x| x.to_bits());
    m.chain(s.r.iter().flatten().map(|x| x.to_bits())).collect()
}

fn check_canon(sample: usize) {
    let h = harness();
    let states = [state(0.0), state(0.25), state(0.5)];
    let setting = FastMath::default();
    let got = branch_read_canon_gpu(h.device(), h.queue(), &wgsl(), &states, setting).unwrap();
    assert_eq!(got.len(), 3 * read::CANON_WORDS);
    let at = sample * read::CANON_WORDS;
    assert_eq!(
        got[at..at + read::CANON_OPERANDS],
        operand_words(&states[1]),
        "a state's operands are not its bits"
    );
    assert_eq!(read::CANON_WORDS, 40);
    assert_eq!(BRANCH_READ_CANON_ENTRY, "stage_branch_read_canon");
}

#[test]
fn branch_read_gpu_canon_echoes_its_operands() {
    check_canon(1);
    let h = harness();
    let none = branch_read_canon_gpu(h.device(), h.queue(), "", &[], FastMath::default());
    assert_eq!(none.unwrap(), Vec::<u32>::new());
}

negative_control!(
    branch_read_gpu_canon_echoes_its_operands,
    "another sample's words",
    expected = "a state's operands are not its bits",
    check_canon(2)
);

fn latent() -> Latent<f32> {
    let eight = |k: f32| core::array::from_fn(|i| k * (i as f32 - 3.5) / 8.0);
    Latent {
        z0: eight(1.0),
        q1: eight(0.5),
        q2: eight(-0.25),
    }
}

fn check_decode(e_star: Option<f32>, raised: u32) {
    let h = harness();
    let stage = BranchReadStage::<LatentDefaults>::new(h.device(), &wgsl(), FastMath::default())
        .expect("the read-back's pipeline");
    assert_eq!(stage.mode(), StageMode::Off);
    let uv = [[0.5, 0.5], [0.25, 0.75]];
    let got = stage
        .dispatch(h.device(), h.queue(), &latent(), e_star, &uv)
        .unwrap();
    assert_eq!(got.len(), 2 * read::DECODE_WORDS);
    for sample in got.chunks(read::DECODE_WORDS) {
        assert_eq!(sample[read::M01_AT], 1, "M01_TINY always runs");
        assert_eq!(
            sample[read::FEASIBILITY_AT],
            0,
            "Latent has no construction"
        );
        assert_eq!(
            sample[read::ENERGY_INFEASIBLE_AT],
            raised,
            "η_E's row is not as its target says"
        );
    }
    let none = stage.dispatch(h.device(), h.queue(), &latent(), None, &[]);
    assert_eq!(none.unwrap(), Vec::<u32>::new());
}

#[test]
fn branch_read_gpu_decode_rows_follow_chart_and_target() {
    check_decode(None, 0);
    check_decode(Some(-0.3), 1);
}

negative_control!(
    branch_read_gpu_decode_rows_follow_chart_and_target,
    "a target with η_E's row read off",
    expected = "η_E's row is not as its target says",
    check_decode(Some(-0.3), 0)
);

fn check_entries(wgsl: &str) {
    let h = harness();
    let d = h.device();
    let s = FastMath::default();
    macro_rules! each {
        ($($v:ident),*) => {$(
            BranchReadStage::<$v>::new(d, wgsl, s).expect("an entry has no pipeline");
        )*};
    }
    each!(
        LatentDefaults,
        LatentStickBreaking,
        InvariantLEDefaults,
        InvariantLKDefaults,
        MassSimplexDefaults,
        BurrauDefaults,
        EuclidPlaneDefaults,
        MixedMu1AlphaDefaults,
        BurrauThetaKDefaults,
        BurrauThetaLzDefaults,
        BurrauThetaDeltaMDefaults,
        ShapeSphereDefaults
    );
}

#[test]
fn branch_read_gpu_every_variant_has_its_entry() {
    check_entries(&wgsl());
}

negative_control!(
    branch_read_gpu_every_variant_has_its_entry,
    "a module with no such entries",
    expected = "an entry has no pipeline",
    check_entries("@compute @workgroup_size(1) fn other() {}")
);
