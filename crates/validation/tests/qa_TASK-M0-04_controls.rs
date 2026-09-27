//! Negative controls for qa's tests in `qa_TASK-M0-04.rs` and `qa_TASK-M0-04_r2.rs` (REQ-VAL-153; R-199, R-209).
//! Each runs its test's check, copied here, on an input the check must reject, so the test can fail (philosophy
//! §4.4). The subprocess bodies are the `qa_child` binary's (R-210).
#![cfg(feature = "controls")]

use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use validation::gpu::{AdapterInfo, GpuHarness, BACKEND_VAR};
use validation::negative_control;
use validation::prop;
use validation::spawn::Spawn;

fn harness() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// Index of the first differing word, lengths included (as `qa_TASK-M0-04.rs`).
fn diff_at(a: &[u32], b: &[u32]) -> Option<usize> {
    if a.len() != b.len() {
        return Some(a.len().min(b.len()));
    }
    (0..a.len()).find(|&i| a[i] != b[i])
}

/// The output of the `qa_child` body `body` with the given environment changes, stdout then stderr.
fn child(body: &str, env: &[(&str, Option<&str>)]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_qa_child"));
    cmd.arg(body);
    for (k, v) in env {
        match v {
            Some(v) => cmd.env(k, v),
            None => cmd.env_remove(k),
        };
    }
    let o = cmd.timed_output().expect("qa_child ran");
    let (out, err) = (
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr),
    );
    format!("{out}{err}")
}

/// The child's marker line, from `tag` to the end of its line.
fn marker(t: &str, tag: &str) -> String {
    let at = t
        .find(tag)
        .unwrap_or_else(|| panic!("child printed no {tag}: {t}"));
    t[at..].lines().next().unwrap_or_default().to_string()
}

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
        let input = words();
        let out = harness().run_wgsl(FAULTY, "flip_low_bit", &[&input]);
        assert_eq!(diff_at(&input, &out), None, "identity is not bit-exact");
    }
);

negative_control!(
    qa_gpu_harness_round_trips_lengths_off_the_workgroup_size,
    "a dispatch that leaves the last word unwritten must fail at a length off the workgroup size",
    expected = "identity failed at length 65",
    {
        let input = &words()[..65];
        let out = harness().run_wgsl(FAULTY, "skip_last", &[input]);
        assert_eq!(diff_at(input, &out), None, "identity failed at length 65");
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
    expected = "a - b is wrong",
    {
        let a = words();
        let b: Vec<u32> = a.iter().map(|w| w.rotate_left(7) ^ 0xA5A5_A5A5).collect();
        let want: Vec<u32> = a.iter().zip(&b).map(|(x, y)| x.wrapping_sub(*y)).collect();
        let got = harness().run_wgsl(SUB, "sub", &[&b, &a]);
        assert_eq!(diff_at(&want, &got), None, "a - b is wrong");
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
    expected = "the overloads agree at width",
    {
        let h = harness();
        let low: Vec<u32> = words().iter().map(|w| w & 0x7FFF_FFFF).collect();
        let widths: Vec<u32> = (0..low.len()).map(|i| 1 + (i as u32 % 31)).collect();
        let s = h.run_wgsl(EXTRACT, "as_i32", &[&low, &widths]);
        let u = h.run_wgsl(EXTRACT, "as_u32", &[&low, &widths]);
        for i in 0..low.len() {
            assert_ne!(
                s[i], u[i],
                "the overloads agree at width {} on word {i}",
                widths[i]
            );
        }
    }
);

negative_control!(
    qa_gpu_backend_env_governs_harness_new,
    "the platform's own backend opens a device, so the refusal check must fail on it",
    expected = "opened a device",
    {
        let value = if cfg!(target_os = "macos") {
            "metal"
        } else {
            "vulkan"
        };
        let line = marker(
            &child("open_harness", &[(BACKEND_VAR, Some(value))]),
            "QA_OPEN_",
        );
        assert!(
            line.starts_with("QA_OPEN_ERR"),
            "{BACKEND_VAR}={value} opened a device: {line}"
        );
    }
);

negative_control!(
    qa_prop_seed_printed_and_rerun_through_the_environment,
    "two different seeds must fail the same-failing-case check",
    expected = "the seed did not reproduce the failing case",
    {
        let draw = |seed: &str| {
            marker(
                &child("failing_property", &[("PROPTEST_RNG_SEED", Some(seed))]),
                "QA_FIRST_DRAW",
            )
        };
        assert_eq!(
            draw("1"),
            draw("2"),
            "the seed did not reproduce the failing case"
        );
    }
);

negative_control!(
    qa_prop_shared_config_runs_256_cases_marked_provisional,
    "a runner of 255 cases must fail the 256-case count",
    expected = "the runner did not run 256 cases",
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
        assert_eq!(
            n.load(Ordering::Relaxed),
            256,
            "the runner did not run 256 cases"
        );
    }
);

negative_control!(
    qa_adapter_info_printout_names_the_backend,
    "a Metal adapter's printout must fail the check that it names Vulkan",
    expected = "adapter info does not name Vulkan",
    {
        let shown = AdapterInfo {
            name: "qa-adapter-name".into(),
            backend: wgpu::Backend::Metal,
            driver: "qa-driver".into(),
            driver_info: "qa-driver-info".into(),
        }
        .to_string();
        assert!(
            shown.contains("Vulkan"),
            "adapter info does not name Vulkan: {shown:?}"
        );
    }
);

negative_control!(
    qa_prop_case_count_is_not_replaced_by_proptest_cases,
    "a shared count replaced as the default runner's is by PROPTEST_CASES=3 must fail the 256-case check",
    expected = "PROPTEST_CASES=3 changed the shared case count",
    {
        let measured = marker(
            &child("count_cases", &[("PROPTEST_CASES", Some("3"))]),
            "QA_CASES ",
        );
        let d = measured
            .rsplit_once("default=")
            .map(|(_, d)| d.trim())
            .unwrap_or_else(|| panic!("child printed no default count: {measured}"));
        let line = format!("QA_CASES shared={d} config={d} default={d}");
        assert!(
            line.contains("shared=256 ") && line.contains("config=256 "),
            "PROPTEST_CASES=3 changed the shared case count: {line}"
        );
    }
);
