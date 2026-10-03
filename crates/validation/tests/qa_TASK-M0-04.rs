//! QA tests for TASK-M0-04, written from REQ-SYS-065, REQ-VAL-151, the task's acceptance lines, parity_contract §6
//! and pitfalls §9. Each test carries an inline negative control (R-176; the registry is TASK-M0-21/M0-22, R-198).
//!
//! Tests that must change process state (the `PRIN_GPU_BACKEND` and `PROPTEST_RNG_SEED` variables, both read once or
//! at construction) run a body of the `qa_child` binary as a child process (R-210), so the parent's environment is
//! never mutated.

#[path = "support/qa_m0_04.rs"]
mod qa_m0_04;

use qa_m0_04::*;
use validation::gpu::BACKEND_VAR;
use validation::prop;

/// A 2^16-word fixture independent of the implementation's: 0, all ones, every single-bit word, every
/// single-bit-cleared word, then xorshift32 words. Every bit position is exercised alone in both states.
fn qa_fixture() -> Vec<u32> {
    let mut v = vec![0u32, u32::MAX];
    v.extend((0..32).map(|b| 1u32 << b));
    v.extend((0..32).map(|b| !(1u32 << b)));
    let mut x = 0x1234_5678u32;
    while v.len() < 1 << 16 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        v.push(x);
    }
    v
}

/// REQ-SYS-065 / acceptance `gpu_harness`: a WGSL identity dispatch round-trips 2^16 u32 words bit-exact.
#[test]
fn qa_gpu_harness_identity_round_trips_2_16_words_bit_exact() {
    let h = harness();
    let input = qa_fixture();
    assert_eq!(input.len(), 1 << 16);
    let out = h.run_wgsl(IDENTITY, "identity", &[&input]);
    check_bit_exact(&input, &out, h.adapter_info());
    // Control: a dispatch that flips one low bit must be seen at that word.
    let bad = h.run_wgsl(IDENTITY, "flip_low_bit", &[&input]);
    assert_eq!(
        diff_at(&input, &bad),
        Some(12345),
        "control: a one-bit fork went unseen"
    );
}

/// The harness dispatches over the whole buffer, not only whole workgroups: lengths off the workgroup size.
#[test]
fn qa_gpu_harness_round_trips_lengths_off_the_workgroup_size() {
    let h = harness();
    let fixture = qa_fixture();
    for len in [1usize, 63, 64, 65, 1000, (1 << 16) - 1] {
        let input: Vec<u32> = fixture.iter().rev().take(len).map(|w| w | 1).collect();
        let out = h.run_wgsl(IDENTITY, "identity", &[&input]);
        check_length(len, &input, &out);
        // Control: a kernel that leaves the last word unwritten must differ there (every input word is nonzero).
        let bad = h.run_wgsl(IDENTITY, "skip_last", &[&input]);
        assert_eq!(
            diff_at(&input, &bad),
            Some(len - 1),
            "control: unwritten tail unseen at length {len}"
        );
    }
}

/// `run_wgsl(module, entry, inputs)` takes several storage buffers; input k is binding k, output the next.
#[test]
fn qa_gpu_harness_binds_several_inputs_in_order() {
    let h = harness();
    let a = qa_fixture();
    let b: Vec<u32> = a.iter().map(|w| w.rotate_left(7) ^ 0xA5A5_A5A5).collect();
    let want: Vec<u32> = a.iter().zip(&b).map(|(x, y)| x.wrapping_sub(*y)).collect();
    let got = h.run_wgsl(SUB, "sub", &[&a, &b]);
    check_sub(&want, &got);
    // Control: the swapped order gives b - a, which must differ, so the check above can tell the order.
    let swapped = h.run_wgsl(SUB, "sub", &[&b, &a]);
    assert!(
        diff_at(&want, &swapped).is_some(),
        "control: swapping the inputs changed nothing"
    );
}

/// Pitfalls §9 / acceptance `gpu_harness_can_fire`: on words with bit 31 set, the i32 `extractBits` overload
/// sign-extends and the u32 one does not, for every field width ending at bit 31 below 32; at width 32 the two agree
/// (nothing to extend), and with bit 31 clear they agree at every width. The harness's reachable output includes the
/// sign-extension failure.
#[test]
fn qa_gpu_harness_sees_extractbits_sign_extension_at_every_width() {
    let h = harness();
    let base = qa_fixture();
    let widths: Vec<u32> = (0..base.len()).map(|i| 1 + (i as u32 % 32)).collect();
    let high: Vec<u32> = base.iter().map(|w| w | 0x8000_0000).collect();
    let s = h.run_wgsl(EXTRACT, "as_i32", &[&high, &widths]);
    let u = h.run_wgsl(EXTRACT, "as_u32", &[&high, &widths]);
    for i in 0..high.len() {
        let w = widths[i];
        let want_u = if w == 32 {
            high[i]
        } else {
            high[i] >> (32 - w)
        };
        let want_s = ((high[i] as i32) >> (32 - w)) as u32;
        assert_eq!(u[i], want_u, "u32 extractBits wrong at word {i} width {w}");
        assert_eq!(s[i], want_s, "i32 extractBits wrong at word {i} width {w}");
        if w < 32 {
            check_overloads_differ(s[i], u[i], w, i);
        } else {
            assert_eq!(
                s[i], u[i],
                "control: at width 32 there is nothing to extend (word {i})"
            );
        }
    }
    // Control: bit 31 clear, the overloads agree at every width.
    let low: Vec<u32> = base.iter().map(|w| w & 0x7FFF_FFFF).collect();
    let s = h.run_wgsl(EXTRACT, "as_i32", &[&low, &widths]);
    let u = h.run_wgsl(EXTRACT, "as_u32", &[&low, &widths]);
    assert_eq!(
        diff_at(&s, &u),
        None,
        "control: overloads differ with bit 31 clear"
    );
}

/// REQ-SYS-065 / acceptance `gpu_backend_env`: `GpuHarness::new()` itself (not only the parser) fails naming
/// `PRIN_GPU_BACKEND` when it is `dx12`, empty or a wrong-case name; unset opens the platform's backend and logs that
/// it was the default (R-206); `metal` and `vulkan` select that backend and never another.
#[test]
fn qa_gpu_backend_env_governs_harness_new() {
    for value in [Some("dx12"), Some(""), Some("Metal"), Some("gl")] {
        let line = open_with(value);
        check_refused(value, &line);
        assert!(
            line.contains(BACKEND_VAR),
            "{BACKEND_VAR}={value:?}: error does not name the variable: {line}"
        );
    }
    for (value, backend) in [("metal", "Metal"), ("vulkan", "Vulkan")] {
        let line = open_with(Some(value));
        if line.starts_with("QA_OPEN_OK") {
            assert_eq!(
                line,
                format!("QA_OPEN_OK backend={backend}"),
                "{BACKEND_VAR}={value} gave another backend"
            );
        }
    }
    let unset = text(&child("qa_child_open_harness", &[(BACKEND_VAR, None)]));
    let default = platform_default();
    let shown = if default == "metal" {
        "Metal"
    } else {
        "Vulkan"
    };
    assert_eq!(
        marker(&unset, "QA_OPEN_"),
        format!("QA_OPEN_OK backend={shown}"),
        "unset {BACKEND_VAR} did not open the platform's backend"
    );
    assert!(
        unset.contains(&format!(
            "gpu backend: {default} (platform default, {BACKEND_VAR} unset)"
        )),
        "unset {BACKEND_VAR}: the harness did not log the default it chose: {unset}"
    );
    // Control: the backend this run was given (or the platform default, R-206) opens a device on exactly that
    // backend, so the refusals above are not unconditional.
    let current = std::env::var(BACKEND_VAR).unwrap_or_else(|_| default.to_owned());
    let want = match current.as_str() {
        "metal" => "Metal",
        "vulkan" => "Vulkan",
        other => panic!("unexpected {BACKEND_VAR}={other}"),
    };
    assert_eq!(
        open_with(Some(&current)),
        format!("QA_OPEN_OK backend={want}"),
        "control: the configured backend did not open"
    );
}

fn seed_in(t: &str) -> u64 {
    let at = t
        .find("PROPTEST_RNG_SEED=")
        .unwrap_or_else(|| panic!("no seed printed on failure: {t}"));
    t[at + "PROPTEST_RNG_SEED=".len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| panic!("seed is not a number: {t}"))
}

/// Acceptance `prop_seed`: a property made to fail prints the seed it failed on, and re-running with that seed the
/// way the message says (`PROPTEST_RNG_SEED=<seed>`, a fresh process) fails on the same case.
#[test]
fn qa_prop_seed_printed_and_rerun_through_the_environment() {
    let first = child("qa_child_failing_property", &[("PROPTEST_RNG_SEED", None)]);
    let t1 = text(&first);
    assert!(!first.status.success(), "the failing property passed: {t1}");
    let seed = seed_in(&t1);
    let again = child(
        "qa_child_failing_property",
        &[("PROPTEST_RNG_SEED", Some(&seed.to_string()))],
    );
    let t2 = text(&again);
    assert!(
        !again.status.success(),
        "re-run with the printed seed passed: {t2}"
    );
    assert_eq!(seed_in(&t2), seed, "the re-run reports a different seed");
    check_same_draw(&first_draw(&t1), &first_draw(&t2));
    // Control: another seed draws a different first failing case, so the match above is the seed's doing.
    let other = child(
        "qa_child_failing_property",
        &[("PROPTEST_RNG_SEED", Some(&(seed ^ 0xDEAD_BEEF).to_string()))],
    );
    assert_ne!(
        first_draw(&t1),
        first_draw(&text(&other)),
        "control: two seeds drew the same failing case"
    );
}

/// REQ-VAL-151 / R-376: 256 is marked confirmed, not provisional, and the printed status says so. "confirmed" alone
/// would also match the provisional form ("provisional until confirmed at the M0 gate"), so the status must name R-376
/// and must not say provisional.
fn check_marked_confirmed(provisional: bool, status: &str) {
    assert!(
        !provisional,
        "256 was confirmed at the M0 gate (R-376), yet CASES is still marked provisional"
    );
    assert!(
        status.contains("256")
            && status.contains("confirmed")
            && status.contains("R-376")
            && !status.contains("provisional"),
        "status hides value or its confirmed status (R-376): {status}"
    );
}

#[cfg(feature = "controls")]
mod marked_confirmed {
    use super::*;
    validation::negative_control!(
        qa_prop_shared_config_runs_256_cases_marked_provisional,
        "the status line prop.rs prints while CASES is provisional, with the flag off",
        expected = "status hides value or its confirmed status (R-376)",
        check_marked_confirmed(
            false,
            "prop::CASES = 256 (provisional until confirmed at the M0 gate; REQ-VAL-151, R-203, R-376)"
        )
    );
}

#[cfg(feature = "controls")]
mod flag_provisional {
    use super::*;
    validation::negative_control!(
        qa_prop_shared_config_runs_256_cases_marked_provisional,
        "CASES_PROVISIONAL still true",
        expected =
            "256 was confirmed at the M0 gate (R-376), yet CASES is still marked provisional",
        check_marked_confirmed(true, &prop::cases_status())
    );
}

/// REQ-VAL-151 / R-203: the shared config runs 256 cases per property, and says the value is confirmed: the human
/// confirmed 256 at the M0 gate (R-376), so it is no longer marked provisional (R-182). The test keeps its name, which
/// `qa_TASK-M0-04_controls.rs` and `qa_TASK-M0-25.rs` cite; what it asserts is R-376's.
#[test]
fn qa_prop_shared_config_runs_256_cases_marked_provisional() {
    use std::sync::atomic::{AtomicU32, Ordering};
    let n = AtomicU32::new(0);
    prop::run(&proptest::prelude::any::<u64>(), |_| {
        n.fetch_add(1, Ordering::Relaxed);
        Ok(())
    });
    check_ran_256(n.load(Ordering::Relaxed));
    assert_eq!(
        prop::config(1).cases,
        256,
        "the shared config's case count is not R-203's 256"
    );
    let status = prop::cases_status();
    println!("{status}");
    check_marked_confirmed(std::hint::black_box(prop::CASES_PROVISIONAL), &status);
    // Control: the counter sees a different case count when the config differs, so 256 above was measured.
    let m = AtomicU32::new(0);
    proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: 255,
        ..prop::config(1)
    })
    .run(&proptest::prelude::any::<u64>(), |_| {
        m.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })
    .unwrap();
    assert_ne!(
        m.load(Ordering::Relaxed),
        256,
        "control: the counter cannot see the case count"
    );
}
