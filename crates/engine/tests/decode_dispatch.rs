//! The decode dispatch's host-side arithmetic (`engine::decode_dispatch`; TASK-M2-06), with no device:
//! - `decode_dispatch_check_samples`: a dispatch covers `n` samples at 64 per workgroup within the device's workgroup
//!   count and storage-binding size, each limit met exactly and refused one past it, and `n` within u32.
//! - `decode_dispatch_uniform_words`: the uniform's words are the chart's six rows, in order, as f32 bits, then the
//!   flags word and its padding, then the energy row (`η_E`'s switch and `E*`'s bits).

use engine::decode_dispatch::{check_samples, uniform_words};
use kernel::chart::Latent;
use kernel::stage::decode::{descriptor_words, DecodeUniforms, UniformChart};
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
    let wide = limits(u32::MAX, u64::MAX);
    assert_eq!(ok(0, &wide), Ok(0), "no samples");
    // Two workgroups: 128 samples fit, 129 need a third.
    let two = limits(2, u64::MAX);
    assert_eq!(ok(128, &two), Ok(128), "128 samples in two workgroups");
    let past = ok(129, &two).expect_err("129 samples in two workgroups");
    assert!(past.contains("3 workgroups"), "{past}");
    // A descriptor buffer of exactly 64 samples' bytes: 64 fit, 65 do not.
    let bytes = (64 * descriptor_words() * 4) as u64;
    let store = limits(u32::MAX, bytes);
    assert_eq!(ok(64, &store), Ok(64), "64 samples in a {bytes} B binding");
    let past = ok(65, &store).expect_err("65 samples in a 64-sample binding");
    assert!(
        past.contains(&format!("over the device's {bytes} B")),
        "{past}"
    );
    // The sample count is a u32: u32::MAX is one, one more is not.
    let max = u32::MAX as usize;
    assert_eq!(ok(max, &wide), Ok(u32::MAX), "u32::MAX samples");
    let past = ok(max + 1, &wide).expect_err("u32::MAX + 1 samples");
    assert!(past.contains("over u32"), "{past}");
}

#[test]
fn decode_dispatch_check_samples() {
    check_counts(check_samples);
}

negative_control!(
    decode_dispatch_check_samples,
    "a check that counts workgroups by truncating division must fail",
    expected = "129 samples in two workgroups",
    check_counts(|n, l| {
        if n / 64 > l.max_compute_workgroups_per_dimension as usize {
            return Err(engine::compute::ComputeError("over".into()));
        }
        check_samples(n.min(64), l).map(|_| n as u32)
    })
);

fn check_words(words: fn(&DecodeUniforms) -> Vec<u32>) {
    let chart = Latent {
        z0: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        q1: [9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0],
        q2: [17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0],
    };
    let mut u = DecodeUniforms {
        chart: chart.uniform(),
        ..DecodeUniforms::default()
    };
    u.flags.x = 1;
    u.energy = kernel::stage::decode::energy(Some(2.5));
    let got = words(&u);
    let mut want: Vec<u32> = chart
        .z0
        .iter()
        .chain(&chart.q1)
        .chain(&chart.q2)
        .map(|x| x.to_bits())
        .collect();
    want.extend([1, 0, 0, 0]);
    want.extend([1, 2.5f32.to_bits(), 0, 0]);
    assert_eq!(got, want, "the uniform's words");
    assert_eq!(
        got.len() * 4,
        size_of::<DecodeUniforms>(),
        "the uniform's size"
    );
}

#[test]
fn decode_dispatch_uniform_words() {
    check_words(uniform_words);
}

negative_control!(
    decode_dispatch_uniform_words,
    "words with the flags before the chart must fail",
    expected = "the uniform's words",
    check_words(|u| {
        let mut w = uniform_words(u);
        w.rotate_right(4);
        w
    })
);
