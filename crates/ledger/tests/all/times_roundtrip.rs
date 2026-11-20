//! `times` as exact u16 macro-step indices (dd_simstate_payload §2, `times`; render contract, "Unpack layer"; R-86;
//! REQ-PAY-040; TASK-M3-06):
//! - `times_roundtrip`: `t_end_step` in bits 0–15 and `t_dmin_step` in bits 16–31 round-trip exactly through the
//!   kernel's pack and the generated accessors, over every u16 value of each, with no fixed point; and a configuration
//!   with `⌈T/dt_macro⌉ > 65535`, more steps than the u16 indices hold, is refused at dispatch, while 65535 is not.

use validation::{
    negative_control, pack_times, tm_t_dmin_step, tm_t_end_step, validate_dispatch, DispatchError,
    SimUniforms,
};

/// `pack` round-trips every u16 `t_end_step` with its complement as `t_dmin_step`, and the four corner pairs, exactly:
/// each field reads back its own bits, in its own half of the word.
fn check_roundtrip(pack: &dyn Fn(u32, u32) -> u32) {
    let pairs = (0..=65535u32).map(|s| (s, 65535 - s)).chain([
        (0, 0),
        (65535, 65535),
        (0, 65535),
        (65535, 0),
    ]);
    for (end, dmin) in pairs {
        let w = pack(end, dmin);
        assert_eq!(
            (tm_t_end_step(w), tm_t_dmin_step(w)),
            (end, dmin),
            "times {w:#010X} does not round-trip ({end}, {dmin})"
        );
        assert_eq!(
            w,
            end | dmin << 16,
            "times {w:#010X}: the halves are not t_end_step low, t_dmin_step high"
        );
    }
}

/// `dispatch` refuses `(T, dt_macro) = (64, 2⁻¹⁰)`, 65536 steps, and accepts `(65535·2⁻¹⁰, 2⁻¹⁰)`, 65535.
fn check_dispatch_limit(dispatch: &dyn Fn(&SimUniforms) -> Result<u32, DispatchError>) {
    let dt = 1.0 / 1024.0;
    let over = SimUniforms {
        t_horizon: 64.0,
        dt_macro: dt,
        ..SimUniforms::default()
    };
    assert!(
        matches!(dispatch(&over), Err(DispatchError::HorizonSteps { .. })),
        "⌈T/dt_macro⌉ = 65536 is not refused at dispatch: {:?}",
        dispatch(&over)
    );
    let edge = SimUniforms {
        t_horizon: 65535.0 * dt,
        ..over
    };
    assert_eq!(
        dispatch(&edge),
        Ok(65535),
        "⌈T/dt_macro⌉ = 65535 is refused at dispatch"
    );
}

#[test]
fn times_roundtrip() {
    check_roundtrip(&pack_times);
    check_dispatch_limit(&|u: &SimUniforms| validate_dispatch(u).map(|v| v.horizon_steps()));
}

negative_control!(
    times_roundtrip,
    "a pack that keeps 15 bits of t_end_step loses step 32768 and above",
    expected = "does not round-trip",
    check_roundtrip(&|end: u32, dmin: u32| pack_times(end & 0x7FFF, dmin))
);

#[test]
fn times_roundtrip_dispatch_refuses_65536() {
    check_dispatch_limit(&|u: &SimUniforms| validate_dispatch(u).map(|v| v.horizon_steps()));
}

negative_control!(
    times_roundtrip_dispatch_refuses_65536,
    "a dispatch without the u16 limit accepts 65536 steps",
    expected = "is not refused at dispatch",
    check_dispatch_limit(&|u: &SimUniforms| Ok((u.t_horizon / u.dt_macro).ceil() as u32))
);
