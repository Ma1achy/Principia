//! The canonicalise dispatch's host-side check (`engine::decode_dispatch::check_canon_samples`; TASK-M2-07):
//! - `canon_dispatch_check_samples`: a dispatch covers `n` states within the device's workgroup count and the output
//!   buffer's binding size, `canon_words()` per state, each limit met exactly and refused one past it.

use engine::decode_dispatch::check_canon_samples;
use kernel::stage::canon::canon_words;
use validation::negative_control;

/// Limits with `groups` workgroups per dimension and `bytes` per storage binding, the rest wgpu's defaults.
fn limits(groups: u32, bytes: u64) -> wgpu::Limits {
    wgpu::Limits {
        max_compute_workgroups_per_dimension: groups,
        max_storage_buffer_binding_size: bytes,
        ..wgpu::Limits::default()
    }
}

type Check = fn(usize, &wgpu::Limits) -> Result<u32, engine::compute::ComputeError>;

fn check_counts(check: Check) {
    let ok = |n: usize, l: &wgpu::Limits| check(n, l).map_err(|e| e.0);
    assert_eq!(ok(0, &limits(u32::MAX, u64::MAX)), Ok(0), "no states");
    let past = ok(129, &limits(2, u64::MAX)).expect_err("129 states in two workgroups");
    assert!(past.contains("3 workgroups"), "{past}");
    // An output buffer of exactly 64 states' words: 64 fit, 65 do not. It is larger than the descriptors' buffer
    // `check_samples` checks, so it binds first.
    let bytes = (64 * canon_words() * 4) as u64;
    let store = limits(u32::MAX, bytes);
    assert_eq!(ok(64, &store), Ok(64), "64 states in a {bytes} B binding");
    let past = ok(65, &store).expect_err("65 states in a 64-state binding");
    assert!(past.contains("output buffer"), "{past}");
}

#[test]
fn canon_dispatch_check_samples() {
    assert_eq!(
        canon_words(),
        20,
        "a state's words: 15 for (m, r, p), I_in, λ, φ, λ̃_y and the notices"
    );
    check_counts(check_canon_samples);
}

negative_control!(
    canon_dispatch_check_samples,
    "a check that sizes the output by the descriptors' 16 words a state must fail",
    expected = "65 states in a 64-state binding",
    check_counts(|n, l| engine::decode_dispatch::check_samples(n, l))
);
