//! The engine's side of the bring-up mode (colour_composition Appendix A; REQ-TOOL-015; R-75; RQ-221):
//! - `bringup_pattern_native`: enabling the bring-up mode on the sim key and dispatching the f64 native variant over
//!   the synthetic flat layout writes Appendix A's pattern, read back on the CPU through the generated unpack; the
//!   physics variant is not built (M1), and is refused.
//! - `bringup_pattern_changes_the_sim_key`: the kernel variant is on the sim key, `SimConfig`'s canonical text, by the
//!   kernel's own name for the mode.
//! - `bringup_pattern_sample_limit`: the dispatch covers at most 2¹⁹ samples, where the pattern is exact in f32.
//! - `bringup_pattern_words`: a `SimState` read back from its words is the one written, member by member at the
//!   ledger's offsets; a pattern read one word off fails the readback (pitfalls §9).
//! - `bringup_pattern_gpu_dispatch`: the GPU dispatch's plumbing, on a stand-in for the built kernel: one invocation
//!   per sample, every sample read back in order.
//!
//! The f32 SPIR-V build's GPU readback is `validation`'s `bringup_pattern_spirv`, in CI's `gpu-kernel` job.

use engine::bringup::{gpu, native, samples, ENTRY};
use engine::contract::canonical;
use engine::contract::fast_math::FastMath;
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice,
};
use engine::synthetic::{simstate_from_words, simstate_words, Synthetic};
use kernel::bringup::{max_samples, pattern, words_per_sample, BringUp, Variant};
use kernel::payload::SimStateFTLEOf;
use render::raster::Grid;
use validation::bringup::{check, check_f64, narrow};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The sim key with the kernel variant `kernel_variant`, every other group as at M0.
fn config(kernel_variant: KernelVariant) -> SimConfig {
    SimConfig {
        chart: Chart {},
        plane: Plane {},
        slice: Slice {},
        lock: Lock {},
        links: Links {},
        integrator: Integrator {},
        kernel_variant,
        horizon: Horizon {},
        collision: Collision {},
        quality: Quality {},
    }
}

/// The synthetic flat layout the tests dispatch over: 3 × 2 quads of 8 × 8 tiles, one ensemble copy, 2 px tiles.
fn grid() -> Grid {
    Synthetic::flat(Grid::new([3, 2], 8, 1, 2).expect("a grid"), 0).grid()
}

/// Each sample's f64 `SimState` is Appendix A's pattern.
fn check_native(out: &[SimStateFTLEOf<f64>]) {
    for (i, s) in out.iter().enumerate() {
        check_f64(i as u32, s).unwrap_or_else(|e| panic!("{e}"));
    }
}

#[test]
fn bringup_pattern_native() {
    let sim = config(KernelVariant::BringUp);
    let out = native(sim.kernel_variant, grid()).expect("the bring-up dispatch");
    assert_eq!(
        out.len(),
        grid().sample_count() as usize,
        "one SimState per sample"
    );
    check_native(&out);
    let physics = native(KernelVariant::Physics, grid());
    assert!(
        physics.is_err_and(|e| e.0.contains("only the bring-up mode is built")),
        "the physics variant dispatched, with no physics kernel built"
    );
}

negative_control!(
    bringup_pattern_native,
    "the f64 pattern with one sample's slots swapped must fail the readback",
    expected = "not the bring-up pattern's",
    {
        let mut out = native(KernelVariant::BringUp, grid()).expect("the bring-up dispatch");
        out.swap(5, 6);
        check_native(&out)
    }
);

/// The bring-up mode's canonical sim key differs from the physics one's, and carries the mode by the kernel's name for
/// it.
fn check_sim_key(physics: &SimConfig, bring_up: &SimConfig) {
    let (a, b) = (
        canonical::to_string(physics).expect("canonical"),
        canonical::to_string(bring_up).expect("canonical"),
    );
    assert_ne!(a, b, "enabling the bring-up mode leaves the sim key {a}");
    assert!(
        b.contains(&format!("\"kernel_variant\":\"{}\"", BringUp::NAME)),
        "the sim key {b} does not name the bring-up mode as the kernel does"
    );
    let back: SimConfig = serde_json::from_str(&b).expect("the sim key reads back");
    assert_eq!(
        back.kernel_variant,
        KernelVariant::BringUp,
        "the sim key reads back another variant"
    );
}

#[test]
fn bringup_pattern_changes_the_sim_key() {
    check_sim_key(
        &config(KernelVariant::Physics),
        &config(KernelVariant::BringUp),
    );
}

negative_control!(
    bringup_pattern_changes_the_sim_key,
    "a bring-up config that leaves the kernel variant at physics must fail",
    expected = "enabling the bring-up mode leaves the sim key",
    check_sim_key(
        &config(KernelVariant::Physics),
        &config(KernelVariant::Physics)
    )
);

/// A grid of exactly `n` samples: one quad of one tile, `n − 1` ensemble copies.
fn of(n: u32) -> Grid {
    Grid::new([1, 1], 1, n - 1, 1).expect("a grid")
}

/// The dispatch covers exactly the grids of at most `max` samples.
fn check_limit(max: u32) {
    assert_eq!(
        samples(KernelVariant::BringUp, of(max)).ok(),
        Some(max),
        "a grid of {max} samples is refused"
    );
    assert!(
        samples(KernelVariant::BringUp, of(max + 1)).is_err(),
        "a grid of {} samples is dispatched, past the pattern's exact range",
        max + 1
    );
}

#[test]
fn bringup_pattern_sample_limit() {
    assert_eq!(max_samples(), 1 << 19, "the limit is not 2¹⁹ samples");
    check_limit(max_samples());
}

negative_control!(
    bringup_pattern_sample_limit,
    "a limit one sample lower must fail",
    expected = "is dispatched, past the pattern's exact range",
    check_limit(max_samples() - 1)
);

/// Each of the first 300 samples' f32 pattern, as words, read back through `simstate_from_words` with its words
/// rotated by `shift`, is the pattern; a wrong length is refused.
fn check_words(shift: usize) {
    for i in 0..300 {
        let mut words = simstate_words(&pattern::<f32>(i));
        words.rotate_right(shift);
        let s = simstate_from_words(&words).unwrap_or_else(|e| panic!("{e}"));
        check(i, &s).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            narrow(&pattern::<f64>(i)),
            s,
            "sample {i}'s f64 pattern narrows to another"
        );
    }
    assert!(
        simstate_from_words(&[0; 35]).is_err(),
        "35 words read as a SimStateFTLE"
    );
    // Arbitrary words, every byte distinct, the u16 members' high bytes and NaN bits included, read back whole.
    let words: Vec<u32> = (0..words_per_sample() as u32)
        .map(|k| 0x0403_0201u32.wrapping_mul(4 * k + 1) ^ 0x8080_8080)
        .collect();
    let s = simstate_from_words(&words).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        simstate_words(&s),
        words,
        "arbitrary words do not read back whole"
    );
}

#[test]
fn bringup_pattern_words() {
    check_words(0);
}

negative_control!(
    bringup_pattern_words,
    "the pattern read one word off must fail the readback (pitfalls §9)",
    expected = "not the bring-up pattern's",
    check_words(1)
);

/// A stand-in for the built kernel's bring-up entry point, for the dispatch's own plumbing: each sample of a buffer
/// of `words`-word samples, one invocation per sample, written word by word with its index in the buffer plus 1.
fn stand_in(words: usize) -> String {
    format!(
        "@group(0) @binding(0) var<storage, read_write> simstate: array<u32>;
@compute @workgroup_size(64)
fn {ENTRY}(@builtin(global_invocation_id) id: vec3<u32>) {{
    if (id.x < arrayLength(&simstate) / {words}u) {{
        for (var k = 0u; k < {words}u; k++) {{
            let at = id.x * {words}u + k;
            simstate[at] = at + 1u;
        }}
    }}
}}
"
    )
}

/// The dispatch over `grid` returns one `SimState` per sample, each the stand-in's words, in order: every sample's
/// invocation ran, and the buffer read back whole.
fn check_dispatch(out: &[kernel::payload::SimStateFTLE], grid: Grid) {
    assert_eq!(
        out.len(),
        grid.sample_count() as usize,
        "the dispatch read back {} samples, not the grid's {}",
        out.len(),
        grid.sample_count()
    );
    let words = words_per_sample();
    for (i, s) in out.iter().enumerate() {
        let want: Vec<u32> = (0..words).map(|k| (i * words + k + 1) as u32).collect();
        assert_eq!(
            simstate_words(s),
            want,
            "sample {i} is not its invocation's words"
        );
    }
}

#[test]
fn bringup_pattern_gpu_dispatch() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    // 3 × 2 quads of 8 × 8 tiles, two copies: 768 samples, 12 workgroups.
    let out = gpu(
        h.device(),
        h.queue(),
        &stand_in(words_per_sample()),
        KernelVariant::BringUp,
        grid(),
        FastMath::Off,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    check_dispatch(&out, grid());
    let physics = gpu(
        h.device(),
        h.queue(),
        &stand_in(words_per_sample()),
        KernelVariant::Physics,
        grid(),
        FastMath::Off,
    );
    assert!(
        physics.is_err(),
        "the physics variant dispatched on the GPU, with no physics kernel built"
    );
}

negative_control!(
    bringup_pattern_gpu_dispatch,
    "a readback one sample short must fail",
    expected = "samples, not the grid's",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let out = gpu(
            h.device(),
            h.queue(),
            &stand_in(words_per_sample()),
            KernelVariant::BringUp,
            grid(),
            FastMath::Off,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        check_dispatch(&out[1..], grid())
    }
);
