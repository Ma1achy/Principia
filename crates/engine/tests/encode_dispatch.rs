//! The encode dispatches' host-side check (`engine::decode_dispatch::check_output_words`; TASK-M2-15):
//! - `encode_dispatch_check_words`: a dispatch covers `n` samples within the device's workgroup count and an output
//!   buffer of `words` words a sample within the binding size, each limit met exactly and refused one past it, for the
//!   encode's 12 words a sample and `β`'s one.

use engine::decode_dispatch::check_output_words;
use kernel::stage::encode::encode_words;
use validation::negative_control;

/// Limits with `groups` workgroups per dimension and `bytes` per storage binding, the rest wgpu's defaults.
fn limits(groups: u32, bytes: u64) -> wgpu::Limits {
    wgpu::Limits {
        max_compute_workgroups_per_dimension: groups,
        max_storage_buffer_binding_size: bytes,
        ..wgpu::Limits::default()
    }
}

/// `check` for an output of `words` words a sample: no samples, a workgroup count one past the limit, and a binding of
/// exactly 64 samples' words, which 64 fit and 65 do not.
fn check_counts(check: fn(usize, usize, &wgpu::Limits) -> Result<u32, String>, words: usize) {
    assert_eq!(
        check(0, words, &limits(u32::MAX, u64::MAX)),
        Ok(0),
        "no samples"
    );
    let past = check(129, words, &limits(2, u64::MAX)).expect_err("129 samples in two workgroups");
    assert!(past.contains("3 workgroups"), "{past}");
    let bytes = (64 * words * 4) as u64;
    let store = limits(u32::MAX, bytes);
    assert_eq!(
        check(64, words, &store),
        Ok(64),
        "64 samples of {words} words in a binding of {bytes} B"
    );
    let past = check(65, words, &store).expect_err("65 samples in a 64-sample binding");
    assert!(past.contains("output buffer"), "{past}");
}

fn checked(n: usize, words: usize, l: &wgpu::Limits) -> Result<u32, String> {
    check_output_words(n, words, l).map_err(|e| e.0)
}

#[test]
fn encode_dispatch_check_words() {
    assert_eq!(
        encode_words(),
        12,
        "a sample's words: z's 8, α, β, λ and the notices"
    );
    check_counts(checked, encode_words());
    check_counts(checked, 1);
}

negative_control!(
    encode_dispatch_check_words,
    "a check that sizes the output by the descriptors' 16 words a sample must fail",
    expected = "samples of 12 words in a binding",
    check_counts(
        |n, _, l| engine::decode_dispatch::check_samples(n, l).map_err(|e| e.0),
        encode_words()
    )
);
