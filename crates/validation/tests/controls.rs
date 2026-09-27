//! The negative controls of validation's `gpu::tests` and `prop::tests` unit tests, registered here by the test's
//! name because they reach only the crate's public API (REQ-VAL-152; R-199, R-201). Each runs its test's check on a
//! contaminated input and panics. `gpu_backend_env_rejects_unknown` and `gpu_backend_env_unset_defaults_by_platform`
//! register theirs beside them in `src/gpu.rs`. The GPU controls run where their tests do (TASK-M0-04).
#![cfg(feature = "controls")]

use std::cell::Cell;

use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use validation::gpu::{backend_from, first_mismatch, identity_fixture, GpuHarness, IDENTITY_WGSL};
use validation::negative_control;
use validation::prop::{check_with_seed, config, CASES};

fn harness() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// `gpu::tests`'s two `extractBits` overloads, which differ only on words with bit 31 set.
const EXTRACT_WGSL: &str = r"
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn as_i32(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&input)) { output[id.x] = bitcast<u32>(extractBits(bitcast<i32>(input[id.x]), 28u, 4u)); }
}
@compute @workgroup_size(64)
fn as_u32(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&input)) { output[id.x] = extractBits(input[id.x], 28u, 4u); }
}
";

negative_control!(
    gpu_harness_identity_round_trip,
    "the identity readback compared with the input with bit 31 of word 40000 flipped",
    {
        let input = identity_fixture();
        let mut flipped = input.clone();
        flipped[40000] ^= 1 << 31;
        let output = harness().run_wgsl(IDENTITY_WGSL, "identity", &[&input]);
        assert_eq!(first_mismatch(&flipped, &output), None);
    }
);

negative_control!(
    gpu_harness_can_fire,
    "the overloads required to differ on every word, on words with bit 31 clear",
    {
        let h = harness();
        let low: Vec<u32> = (0..4096u32)
            .map(|i| i.wrapping_mul(0x0001_0F31) & 0x7FFF_FFFF)
            .collect();
        let signed = h.run_wgsl(EXTRACT_WGSL, "as_i32", &[&low]);
        let unsigned = h.run_wgsl(EXTRACT_WGSL, "as_u32", &[&low]);
        assert!(signed.iter().zip(&unsigned).all(|(s, u)| s != u));
    }
);

negative_control!(
    metal_hosted_probe,
    "the probe's Metal check on the other platform's expectation: not Metal on macOS, Metal elsewhere",
    assert_eq!(
        harness().adapter_info().backend == wgpu::Backend::Metal,
        !cfg!(target_os = "macos")
    )
);

negative_control!(
    gpu_backend_env_selects_backend,
    "metal required to select the Vulkan backend",
    assert_eq!(backend_from(Some("metal")), Ok(wgpu::Backends::VULKAN))
);

negative_control!(
    prop_seed_is_printed_and_reproduces,
    "the first failing draw of one seed required to reproduce under the next seed",
    {
        let first_failing_draw = |seed| {
            let first = Cell::new(None);
            let _ = check_with_seed(seed, &any::<u32>(), |x| {
                if x >= 1 << 20 && first.get().is_none() {
                    first.set(Some(x));
                }
                prop_assert!(x < 1 << 20);
                Ok(())
            });
            first.get()
        };
        assert_eq!(first_failing_draw(1), first_failing_draw(2));
    }
);

negative_control!(
    prop_seed_config_has_no_persistence_file,
    "proptest's default config, which sets a persistence file, checked for none",
    assert!(Config::default().failure_persistence.is_none())
);

negative_control!(
    prop_seed_runs_the_provisional_case_count,
    "a config of half the cases required to run CASES",
    {
        let runs = Cell::new(0u32);
        let half = Config {
            cases: CASES / 2,
            ..config(1)
        };
        TestRunner::new(half)
            .run(&any::<u32>(), |_| {
                runs.set(runs.get() + 1);
                Ok(())
            })
            .expect("the property holds");
        assert_eq!(runs.get(), CASES);
    }
);
