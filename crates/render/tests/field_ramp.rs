//! The minimal field ramp (`render::colour::field_ramp`; colour_composition §1.2, §3; RQ-232; TASK-M1-09), REQ-COL-001
//! on the CPU side: each `ScalarField` returns `(value, valid)` with the ledger's validity, a sentinel and the absence
//! NaN invalid, `d_min`'s unset value its own case (`field_ramp_scalar_*`); the `lin` compaction clamps to its range
//! (`field_ramp_compaction_*`); the ramp's WGSL declares its invalid lane's uniforms, the override off and R-16's
//! magenta by default (`field_ramp_declares_*`), and its CPU twin orders the unset grey before the invalid lane
//! (`field_ramp_shown_*`). The renders are `numeric_views.rs`'s.
//!
//! Each test registers its negative control (R-176).

use render::assemble::{Declaration, UniformType};
use render::colour::field_ramp::{override_default, Compaction, FieldRamp, ScalarField, Shown};
use validation::negative_control;

fn field(name: &str) -> ScalarField {
    ScalarField::payload(name).unwrap_or_else(|e| panic!("{e}"))
}

/// Checks `length`'s validity: 127, its stored sentinel, is invalid; its neighbours and 0 are valid; a failed gate
/// makes any value invalid.
fn check_length(f: &ScalarField) {
    assert!(
        f.valid.contains("v != 127.0"),
        "`length`'s predicate: {}",
        f.valid
    );
    assert_eq!(f.read(127.0, true), (127.0, false), "127 is invalid");
    for x in [0.0, 76.0, 126.0, 128.0] {
        assert_eq!(f.read(x, true), (x, true), "{x} is valid");
    }
    assert_eq!(f.read(3.0, false), (3.0, false), "a failed gate is invalid");
    assert!(
        f.unset.is_none() && !f.is_unset(f32::INFINITY),
        "`length` has no unset value"
    );
}

#[test]
fn field_ramp_scalar_length_sentinel_is_invalid() {
    check_length(&field("length"));
}

negative_control!(
    field_ramp_scalar_length_sentinel_is_invalid,
    "`d_min`'s field, which has no finite sentinel, takes 127 as valid",
    expected = "`length`'s predicate",
    check_length(&field("d_min"))
);

/// Checks `d_min`'s validity: the canonical NaN alone is invalid, by its bits; +∞ is its unset value, by its bits.
fn check_d_min(f: &ScalarField) {
    assert!(
        f.valid.contains("bitcast<u32>(v) != 0x7fc00000u"),
        "`d_min`'s NaN test: {}",
        f.valid
    );
    let nan = f32::from_bits(0x7fc0_0000);
    assert!(!f.read(nan, true).1, "the canonical NaN is invalid");
    assert!(
        f.read(f32::from_bits(0x7fc0_0001), true).1,
        "another NaN's bits are not the guard's"
    );
    assert!(
        f.read(0.0, true).1 && f.read(1.5, true).1,
        "values are valid"
    );
    assert_eq!(
        f.unset.as_deref(),
        Some("bitcast<u32>(v) == 0x7f800000u"),
        "the unset test is by its bits"
    );
    assert!(f.is_unset(f32::INFINITY), "+inf is unset");
    for x in [f32::MAX, f32::NEG_INFINITY, 0.0] {
        assert!(!f.is_unset(x), "{x} is not unset");
    }
}

#[test]
fn field_ramp_scalar_d_min_nan_invalid_inf_unset() {
    check_d_min(&field("d_min"));
}

negative_control!(
    field_ramp_scalar_d_min_nan_invalid_inf_unset,
    "the word `length`, a u-bits field, has no NaN test",
    expected = "`d_min`'s NaN test",
    check_d_min(&field("length"))
);

#[test]
fn field_ramp_scalar_gates_and_refusals() {
    let ftle = field("ftle");
    assert!(
        ftle.valid.contains("ctx.sample.ftle_valid"),
        "{}",
        ftle.valid
    );
    let diffusion = field("diffusion");
    assert!(
        diffusion.valid.contains("ctx.sample.diffusion_slope_valid"),
        "{}",
        diffusion.valid
    );
    assert_eq!(
        field("t_end_step").valid,
        "true",
        "a u-bits field with no gate or sentinel"
    );
    check_refused("r", "is not a scalar field");
    check_refused("saturated", "is not a scalar field");
    check_refused("no_such_field", "is no ledger field");
}

/// Checks that `name` is refused as a scalar field, with a message containing `why`.
fn check_refused(name: &str, why: &str) {
    match ScalarField::payload(name) {
        Ok(_) => panic!("`{name}` is a scalar field"),
        Err(e) => assert!(e.contains(why), "`{name}`: {e}"),
    }
}

negative_control!(
    field_ramp_scalar_gates_and_refusals,
    "`ftle`, a scalar, is accepted",
    expected = "`ftle` is a scalar field",
    check_refused("ftle", "is not a scalar field")
);

/// Checks the `lin` compaction `c` over `[0, 4]`: 0 at lo, 1 at hi, clamped beyond.
fn check_lin(c: Compaction) {
    for (x, t) in [(0.0, 0.0), (1.0, 0.25), (4.0, 1.0), (-1.0, 0.0), (9.0, 1.0)] {
        assert_eq!(c.place(x), t, "{x} places at {}, not {t}", c.place(x));
    }
    assert_eq!(
        c.wgsl("v"),
        "range_norm(v, 0.0, 4.0, false, vec2<f32>(0.0, 0.0))"
    );
}

#[test]
fn field_ramp_compaction_lin_clamps_to_its_range() {
    check_lin(Compaction::Lin { lo: 0.0, hi: 4.0 });
}

negative_control!(
    field_ramp_compaction_lin_clamps_to_its_range,
    "a compaction over [0, 8] places 4 at 0.5",
    expected = "places at",
    check_lin(Compaction::Lin { lo: 0.0, hi: 8.0 })
);

/// Checks that `wgsl` declares `INVALID_OVERRIDE: u32 = override_on [0, 1]` and `INVALID_COLOUR`, R-16's magenta.
fn check_declares(wgsl: &str, override_on: f64) {
    let d = Declaration::parse(wgsl).unwrap_or_else(|e| panic!("{e}"));
    let names: Vec<&str> = d.uniforms.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(names, ["INVALID_OVERRIDE", "INVALID_COLOUR"]);
    let o = &d.uniforms[0];
    assert_eq!((o.ty, o.range), (UniformType::U32, Some((0.0, 1.0))));
    assert_eq!(o.default, vec![override_on], "the override's default");
    let c = &d.uniforms[1];
    assert_eq!(c.ty, UniformType::Vec3);
    assert_eq!(
        c.default,
        override_default().to_vec(),
        "the override colour"
    );
    assert_eq!(override_default(), [1.0, 0.0, 1.0], "#FF00FF, linear");
}

#[test]
fn field_ramp_declares_its_invalid_lane() {
    for ramp in [FieldRamp::length(), FieldRamp::d_min()] {
        let ramp = ramp.unwrap_or_else(|e| panic!("{e}"));
        let wgsl = ramp.wgsl();
        check_declares(&wgsl, 0.0);
        assert!(
            wgsl.contains(
                "return select(debug_invalid(ctx.frag_xy), uniforms.INVALID_COLOUR, uniforms.INVALID_OVERRIDE != 0u);"
            ),
            "{wgsl}"
        );
    }
}

negative_control!(
    field_ramp_declares_its_invalid_lane,
    "a ramp with the override on by default is not the pattern's",
    expected = "the override's default",
    check_declares(
        &FieldRamp::length().unwrap_or_else(|e| panic!("{e}")).wgsl(),
        1.0
    )
);

/// Checks the CPU twin of `d_min`'s ramp `r`: the unset value grey, before the invalid lane; NaN invalid; a value at
/// its place on `[0, 2]`.
fn check_shown(r: &FieldRamp) {
    assert_eq!(
        r.shown(f32::INFINITY, false),
        Shown::NotYet,
        "unset, even with a failed gate"
    );
    assert_eq!(r.shown(f32::from_bits(0x7fc0_0000), true), Shown::Invalid);
    assert_eq!(r.shown(0.5, true), Shown::Ramp(0.25));
    assert_eq!(r.shown(0.5, false), Shown::Invalid, "a failed gate");
    assert_eq!(r.shown(3.0, true), Shown::Ramp(1.0), "clamped");
}

#[test]
fn field_ramp_shown_orders_unset_before_invalid() {
    check_shown(&FieldRamp::d_min().unwrap_or_else(|e| panic!("{e}")));
    let length = FieldRamp::length().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(length.shown(127.0, true), Shown::Invalid);
    assert_eq!(length.shown(38.0, true), Shown::Ramp(0.5));
}

negative_control!(
    field_ramp_shown_orders_unset_before_invalid,
    "the length ramp has no unset value, so +inf is not grey",
    expected = "unset, even with a failed gate",
    check_shown(&FieldRamp::length().unwrap_or_else(|e| panic!("{e}")))
);
