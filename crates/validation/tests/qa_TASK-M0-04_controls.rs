//! Negative controls for qa's tests in `qa_TASK-M0-04.rs` and `qa_TASK-M0-04_r2.rs` (REQ-VAL-153; R-199, R-209).
//! Each runs its test's check, called from the shared modules its test calls too (REQ-VAL-157; R-215), on an input
//! the check must reject, so the test can fail (philosophy §4.4). The subprocess bodies are the `qa_child` binary's
//! (R-210).
#![cfg(feature = "controls")]

#[path = "support/qa_m0_04.rs"]
mod qa_m0_04;
#[path = "support/qa_m0_04_r2.rs"]
mod qa_m0_04_r2;

use qa_m0_04::*;
use qa_m0_04_r2::*;
use std::sync::atomic::{AtomicU32, Ordering};
use validation::negative_control;
use validation::prop;

/// 2^16 nonzero words.
fn words() -> Vec<u32> {
    (1..=1 << 16)
        .map(|w: u32| w.wrapping_mul(0x9E37_79B9) | 1)
        .collect()
}

const FAULTY: &str = r"
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn flip_low_bit(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&input)) { output[id.x] = input[id.x] ^ select(0u, 1u, id.x == 12345u); }
}
@compute @workgroup_size(64)
fn skip_last(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x + 1u < arrayLength(&input)) { output[id.x] = input[id.x]; }
}
";

negative_control!(
    qa_gpu_harness_identity_round_trips_2_16_words_bit_exact,
    "a dispatch that flips one low bit must fail the bit-exact check",
    expected = "identity is not bit-exact",
    {
        let h = harness();
        let input = words();
        let out = h.run_wgsl(FAULTY, "flip_low_bit", &[&input]);
        check_bit_exact(&input, &out, h.adapter_info());
    }
);

negative_control!(
    qa_gpu_harness_round_trips_lengths_off_the_workgroup_size,
    "a dispatch that leaves the last word unwritten must fail at a length off the workgroup size",
    expected = "identity failed at length 65",
    {
        let input = &words()[..65];
        let out = harness().run_wgsl(FAULTY, "skip_last", &[input]);
        check_length(65, input, &out);
    }
);

const SUB: &str = r"
@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read> b: array<u32>;
@group(0) @binding(2) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn sub(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&a)) { out[id.x] = a[id.x] - b[id.x]; }
}
";

negative_control!(
    qa_gpu_harness_binds_several_inputs_in_order,
    "inputs bound in the swapped order must fail the a - b check",
    expected = "a - b is wrong: inputs bound out of order or lost",
    {
        let a = words();
        let b: Vec<u32> = a.iter().map(|w| w.rotate_left(7) ^ 0xA5A5_A5A5).collect();
        let want: Vec<u32> = a.iter().zip(&b).map(|(x, y)| x.wrapping_sub(*y)).collect();
        let got = harness().run_wgsl(SUB, "sub", &[&b, &a]);
        check_sub(&want, &got);
    }
);

const EXTRACT: &str = r"
@group(0) @binding(0) var<storage, read> word: array<u32>;
@group(0) @binding(1) var<storage, read> width: array<u32>;
@group(0) @binding(2) var<storage, read_write> out: array<u32>;
@compute @workgroup_size(64)
fn as_i32(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&word)) {
        out[id.x] = bitcast<u32>(extractBits(bitcast<i32>(word[id.x]), 32u - width[id.x], width[id.x]));
    }
}
@compute @workgroup_size(64)
fn as_u32(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&word)) { out[id.x] = extractBits(word[id.x], 32u - width[id.x], width[id.x]); }
}
";

negative_control!(
    qa_gpu_harness_sees_extractbits_sign_extension_at_every_width,
    "with bit 31 clear there is no sign to extend, so the overloads agree and the check must fail",
    expected = "i32 and u32 overloads agree at width",
    {
        let h = harness();
        let low: Vec<u32> = words().iter().map(|w| w & 0x7FFF_FFFF).collect();
        let widths: Vec<u32> = (0..low.len()).map(|i| 1 + (i as u32 % 31)).collect();
        let s = h.run_wgsl(EXTRACT, "as_i32", &[&low, &widths]);
        let u = h.run_wgsl(EXTRACT, "as_u32", &[&low, &widths]);
        for i in 0..low.len() {
            check_overloads_differ(s[i], u[i], widths[i], i);
        }
    }
);

negative_control!(
    qa_gpu_backend_env_governs_harness_new,
    "the platform's own backend opens a device, so the refusal check must fail on it",
    expected = "opened a device",
    {
        let value = Some(platform_default());
        check_refused(value, &open_with(value));
    }
);

negative_control!(
    qa_prop_seed_printed_and_rerun_through_the_environment,
    "two different seeds must fail the same-failing-case check",
    expected = "the printed seed did not reproduce the failing case",
    {
        let draw = |seed: &str| {
            first_draw(&text(&child(
                "qa_child_failing_property",
                &[("PROPTEST_RNG_SEED", Some(seed))],
            )))
        };
        check_same_draw(&draw("1"), &draw("2"));
    }
);

negative_control!(
    qa_prop_shared_config_runs_256_cases_marked_provisional,
    "a runner of 255 cases must fail the 256-case count",
    expected = "prop::run did not run R-203's 256 cases",
    {
        let n = AtomicU32::new(0);
        proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
            cases: 255,
            ..prop::config(1)
        })
        .run(&proptest::prelude::any::<u64>(), |_| {
            n.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })
        .unwrap();
        check_ran_256(n.load(Ordering::Relaxed));
    }
);

negative_control!(
    qa_adapter_info_printout_names_the_backend,
    "a Metal adapter's printout must fail the check that it names Vulkan",
    expected = "adapter info does not name Vulkan",
    {
        check_names(&info(wgpu::Backend::Metal).to_string(), "Vulkan");
    }
);

negative_control!(
    qa_prop_case_count_is_not_replaced_by_proptest_cases,
    "a shared count replaced as the default runner's is by PROPTEST_CASES=3 must fail the 256-case check",
    expected = "PROPTEST_CASES=3 changed the shared case count",
    {
        let measured = counts_with("3");
        let d = measured
            .rsplit_once("default=")
            .map(|(_, d)| d.trim())
            .unwrap_or_else(|| panic!("child printed no default count: {measured}"));
        check_count_kept("3", &format!("QA_CASES shared={d} config={d} default={d}"));
    }
);
