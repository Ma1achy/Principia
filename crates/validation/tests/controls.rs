//! The negative controls of validation's `gpu::tests` and `prop::tests` unit tests, registered here by the test's
//! name because they reach only the crate's public API (REQ-VAL-152; R-199, R-201). Each runs its test's own check,
//! from `gpu::checks` or `prop::checks` (REQ-VAL-158; R-215), on a contaminated input, and trips that check's
//! assertion (R-212). `gpu_backend_env_rejects_unknown` and `gpu_backend_env_unset_defaults_by_platform` register
//! theirs beside them in `src/gpu.rs`. The GPU controls run where their tests do (TASK-M0-04).
#![cfg(feature = "controls")]

use engine::contract::fast_math::{FastMath, StageMode};
use proptest::test_runner::Config;
use validation::gpu::checks::*;
use validation::gpu::{identity_fixture, IDENTITY_WGSL};
use validation::negative_control;
use validation::prop::checks::*;
use validation::prop::{config, CASES};

negative_control!(
    gpu_harness_identity_round_trip,
    "the identity readback compared with the input with bit 31 of word 40000 flipped",
    expected = "identity dispatch is not bit-exact",
    {
        let input = identity_fixture();
        let mut flipped = input.clone();
        flipped[40000] ^= 1 << 31;
        check_bit_exact(
            &flipped,
            &harness().run_wgsl(IDENTITY_WGSL, "identity", &[&input]),
        );
    }
);

negative_control!(
    gpu_harness_can_fire,
    "the overloads required to differ on every word, on words with bit 31 clear",
    expected = "i32 and u32 overloads agree on bit-31 words",
    {
        let low: Vec<u32> = (0..4096u32)
            .map(|i| i.wrapping_mul(0x0001_0F31) & 0x7FFF_FFFF)
            .collect();
        check_overloads_differ(&harness(), &low);
    }
);

negative_control!(
    compute_fast_math_off_is_correctly_rounded,
    "the probe's divisions compared with multiplication by the divisor's f32 reciprocal, one ulp off on R-296's columns",
    expected = "with compute fast-math off,",
    check_off_exact(&harness(), |a, b| a * (1.0 / b))
);

negative_control!(
    compute_fast_math_switch_acts,
    "setting on required to show the other mode than the one its backend compiles: off on Metal, on on Vulkan",
    expected = "the divisions do not show the compute stage compiled",
    {
        let h = harness();
        let other = match h.adapter_info().backend {
            wgpu::Backend::Metal => StageMode::Off,
            _ => StageMode::On,
        };
        check_switch(&h, FastMath::On, other);
    }
);

negative_control!(
    compute_fast_math_defaults_off,
    "setting on checked as the default",
    expected = "the compute fast-math setting does not default to off",
    check_defaults_off(&harness(), FastMath::On)
);

negative_control!(
    compute_fast_math_harness_features,
    "the harness's features checked against the passthrough's on every backend and SHADER_F16 beside it",
    expected = "the harness's device does not have exactly the compute entry point's features",
    check_features(
        &harness(),
        wgpu::Features::PASSTHROUGH_SHADERS | wgpu::Features::SHADER_F16
    )
);

negative_control!(
    metal_hosted_probe,
    "the probe's backend check on the other platform's expectation: Vulkan on macOS, Metal elsewhere",
    expected = "PRIN_GPU_BACKEND did not give a",
    {
        let other = if cfg!(target_os = "macos") {
            wgpu::Backend::Vulkan
        } else {
            wgpu::Backend::Metal
        };
        check_backend(harness().adapter_info(), other);
    }
);

negative_control!(
    gpu_backend_env_selects_backend,
    "metal required to select the Vulkan backend",
    expected = "PRIN_GPU_BACKEND=metal did not select",
    check_selects("metal", wgpu::Backends::VULKAN)
);

negative_control!(
    prop_seed_is_printed_and_reproduces,
    "the failing case of one seed required to reproduce under the next seed",
    expected = "the printed seed did not reproduce the failing case",
    check_reproduces(1, 2)
);

negative_control!(
    prop_seed_config_has_no_persistence_file,
    "proptest's default config, which sets a persistence file, checked for none",
    expected = "the shared config sets a failure-persistence file",
    check_no_persistence(&Config::default())
);

negative_control!(
    prop_seed_runs_the_confirmed_case_count,
    "a config of half the cases required to run CASES",
    expected = "the shared config ran a different case count",
    check_runs_cases(Config {
        cases: CASES / 2,
        ..config(1)
    })
);
