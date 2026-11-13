//! The bring-up check and its f64 narrowing, on the kernel's native writer (colour_composition Appendix A;
//! REQ-TOOL-015; R-973's `coll_floor_bits`, which the pattern leaves zero). The GPU readback (`tests/bringup.rs`) needs
//! the built kernel; this runs `validation::bringup::check` and `narrow` on the CPU, so a fault in either fails here
//! without a GPU.

use kernel::bringup::pattern;
use kernel::payload::SimStateFTLE;
use validation::bringup::{check, narrow};
use validation::negative_control;

/// The samples checked: the first, one per `packed_a` field's period, and one past 2¹⁶ (the 16-bit step fields wrap).
const SAMPLES: [u32; 6] = [0, 1, 7, 17, 65, 70_001];

/// Each sample of `f` checked against Appendix A's pattern.
fn check_all(f: impl Fn(u32) -> SimStateFTLE) {
    for i in SAMPLES {
        check(i, &f(i)).unwrap_or_else(|e| panic!("CPU: {e}"));
    }
}

#[test]
fn bringup_check_native() {
    check_all(pattern::<f32>);
}

#[test]
fn bringup_check_narrowed() {
    check_all(|i| narrow(&pattern::<f64>(i)));
}

negative_control!(
    bringup_check_native,
    "a pattern with a collision-floor code written (R-973) must fail the check, which reads the member as stored",
    expected = "`coll_floor_bits` is 1",
    check_all(|i| SimStateFTLE {
        coll_floor_bits: 1,
        ..pattern::<f32>(i)
    })
);

#[cfg(feature = "controls")]
mod narrowed_control {
    use super::*;

    negative_control!(
        bringup_check_narrowed,
        "a narrowing that writes a default payload in place of the f64 pattern must fail the check",
        expected = "not the bring-up pattern's",
        check_all(|_| SimStateFTLE::default())
    );
}

/// `s` at f64 with collision-floor code `code` written (R-973), narrowed; its code as narrowed.
fn narrowed_code(
    code: u32,
    narrowing: impl Fn(&kernel::payload::SimStateFTLEOf<f64>) -> SimStateFTLE,
) -> u32 {
    let mut s = pattern::<f64>(3);
    s.coll_floor_bits = code;
    narrowing(&s).coll_floor_bits
}

/// The narrowing carries the collision-floor member as stored, every code 0..=255 of its byte and a reserved bit.
fn assert_carries(
    narrowing: impl Fn(&kernel::payload::SimStateFTLEOf<f64>) -> SimStateFTLE + Copy,
) {
    for code in (0..=255).chain([0x100, 0x8000_0091]) {
        let got = narrowed_code(code, narrowing);
        assert_eq!(
            got, code,
            "the narrowing carries `coll_floor_bits` {code:#x} as {got:#x}"
        );
    }
}

#[test]
fn bringup_narrow_carries_coll_floor() {
    assert_carries(narrow);
}

#[cfg(feature = "controls")]
mod carry_control {
    use super::*;

    negative_control!(
        bringup_narrow_carries_coll_floor,
        "a narrowing that zeroes `coll_floor_bits` must fail",
        expected = "the narrowing carries `coll_floor_bits` 0x1 as 0x0",
        assert_carries(|s| SimStateFTLE {
            coll_floor_bits: 0,
            ..narrow(s)
        })
    );
}
