//! QA tests for TASK-M0-14's generic `ICDescriptor`, written from REQ-PAY-017 ("the payload width must be a function
//! of the Real type, never hardcoded to f32: `SimState` and `ICDescriptor` both (R-313; R-86's 64 B is
//! `ICDescriptor`'s f32 instantiation)"; verify: instantiate for f32 and f64 and the DoubleF64 stub row, widths from
//! `size_of::<Real>()`, the layout generated per precision, 64 B with declared padding at f32) and REQ-PAY-087's
//! definition in dd_generation_root §3.6 ("Layout at a `Real` of `w` bytes": the twelve fields, in §3.6's order, are
//! `Real`s at `k·w`; `_pad`, four u32s, 16 B, at `12w`; aligned to `Real`'s alignment; size `12w + 16`, the declared
//! `_tail` empty; the table's 64 B / 112 B / 208 B rows). Every offset and size below is §3.6's text, not the
//! generated file's. Each test registers a negative control (R-176).

use std::any::{Any, TypeId};
use std::mem::{align_of, offset_of, size_of, size_of_val};

use kernel::payload::{ICDescriptor, ICDescriptorOf, PayloadReal, PAYLOAD_LAYOUTS};
use validation::negative_control;

/// §3.6's twelve fields, in its order.
const FIELDS: [&str; 12] = [
    "m0",
    "m1",
    "m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
];

/// `ICDescriptorOf<R>`'s layout as built: each member's (name, offset, size, type), `_pad` and `_tail` included, in
/// declaration order, and the struct's size and alignment.
type Built = (Vec<(&'static str, usize, usize, TypeId)>, usize, usize);

fn type_of<T: Any>(_: &T) -> TypeId {
    TypeId::of::<T>()
}

fn built<R: PayloadReal + Any>() -> Built {
    type D<R> = ICDescriptorOf<R>;
    let d = D::<R>::default();
    let members = vec![
        (
            "m0",
            offset_of!(D<R>, m0),
            size_of_val(&d.m0),
            type_of(&d.m0),
        ),
        (
            "m1",
            offset_of!(D<R>, m1),
            size_of_val(&d.m1),
            type_of(&d.m1),
        ),
        (
            "m2",
            offset_of!(D<R>, m2),
            size_of_val(&d.m2),
            type_of(&d.m2),
        ),
        (
            "q_mass",
            offset_of!(D<R>, q_mass),
            size_of_val(&d.q_mass),
            type_of(&d.q_mass),
        ),
        (
            "rho_mag",
            offset_of!(D<R>, rho_mag),
            size_of_val(&d.rho_mag),
            type_of(&d.rho_mag),
        ),
        (
            "lambda_mag",
            offset_of!(D<R>, lambda_mag),
            size_of_val(&d.lambda_mag),
            type_of(&d.lambda_mag),
        ),
        (
            "rho_ratio",
            offset_of!(D<R>, rho_ratio),
            size_of_val(&d.rho_ratio),
            type_of(&d.rho_ratio),
        ),
        (
            "rho_angle",
            offset_of!(D<R>, rho_angle),
            size_of_val(&d.rho_angle),
            type_of(&d.rho_angle),
        ),
        (
            "K_0",
            offset_of!(D<R>, K_0),
            size_of_val(&d.K_0),
            type_of(&d.K_0),
        ),
        (
            "V_0",
            offset_of!(D<R>, V_0),
            size_of_val(&d.V_0),
            type_of(&d.V_0),
        ),
        (
            "virial_ratio",
            offset_of!(D<R>, virial_ratio),
            size_of_val(&d.virial_ratio),
            type_of(&d.virial_ratio),
        ),
        (
            "r_min_pair_0",
            offset_of!(D<R>, r_min_pair_0),
            size_of_val(&d.r_min_pair_0),
            type_of(&d.r_min_pair_0),
        ),
        (
            "_pad",
            offset_of!(D<R>, _pad),
            size_of_val(&d._pad),
            type_of(&d._pad),
        ),
        (
            "_tail",
            offset_of!(D<R>, _tail),
            size_of_val(&d._tail),
            TypeId::of::<()>(),
        ),
    ];
    (members, size_of::<D<R>>(), align_of::<D<R>>())
}

/// §3.6 "Layout at a `Real` of `w` bytes" at a `Real` of `w` bytes aligned to `a`, with the `Real`'s type `real`:
/// field `k` a `Real` at `k·w`; `_pad` four u32s at `12w`; `_tail` empty at `12w + 16`; size `12w + 16`; alignment
/// `a`.
fn s3_6(w: usize, a: usize, real: TypeId) -> Built {
    let mut members: Vec<(&'static str, usize, usize, TypeId)> = FIELDS
        .iter()
        .enumerate()
        .map(|(k, &n)| (n, k * w, w, real))
        .collect();
    members.push(("_pad", 12 * w, 16, TypeId::of::<[u32; 4]>()));
    members.push(("_tail", 12 * w + 16, 0, TypeId::of::<()>()));
    (members, 12 * w + 16, a)
}

fn check_layout(what: &str, got: Built, want: Built) {
    assert_eq!(
        got, want,
        "{what}'s layout is not dd_generation_root §3.6's"
    );
}

/// REQ-PAY-087 / REQ-PAY-017 / R-313: every member of `ICDescriptor` at f32 and at f64 sits where §3.6 puts it, each
/// field a `Real` of that width, the padding four u32s, the tail empty; 64 B aligned to 4 at f32 (R-86), 112 B
/// aligned to 8 at f64.
#[test]
fn qa_descriptor_layout_is_generation_root_3_6() {
    check_layout(
        "ICDescriptor<f32>",
        built::<f32>(),
        s3_6(4, 4, TypeId::of::<f32>()),
    );
    check_layout(
        "ICDescriptor<f64>",
        built::<f64>(),
        s3_6(8, 8, TypeId::of::<f64>()),
    );
    // §3.6's table, as written.
    assert_eq!(
        [
            (
                size_of::<ICDescriptorOf<f32>>(),
                align_of::<ICDescriptorOf<f32>>()
            ),
            (
                size_of::<ICDescriptorOf<f64>>(),
                align_of::<ICDescriptorOf<f64>>()
            ),
        ],
        [(64, 4), (112, 8)],
        "ICDescriptor is not §3.6's 64 B at f32 and 112 B at f64"
    );
}

negative_control!(
    qa_descriptor_layout_is_generation_root_3_6,
    "the f64 descriptor held to the f32 layout, as a descriptor hardcoded to f32 would be",
    expected = "layout is not dd_generation_root §3.6's",
    check_layout(
        "ICDescriptor<f64>",
        built::<f64>(),
        s3_6(4, 4, TypeId::of::<f32>())
    )
);

/// R-86: the members, `_pad` and `_tail` included, fill the struct: no implicit padding at either instantiation.
fn check_no_implicit_padding(what: &str, b: &Built) {
    let sum: usize = b.0.iter().map(|m| m.2).sum();
    assert_eq!(
        sum,
        b.1,
        "{what} has {} B of implicit padding",
        b.1 as isize - sum as isize
    );
}

#[test]
fn qa_descriptor_padding_is_declared_at_every_instantiation() {
    check_no_implicit_padding("ICDescriptor<f32>", &built::<f32>());
    check_no_implicit_padding("ICDescriptor<f64>", &built::<f64>());
}

negative_control!(
    qa_descriptor_padding_is_declared_at_every_instantiation,
    "the f64 descriptor's members without its declared `_pad` leave 16 B unaccounted",
    expected = "B of implicit padding",
    {
        let (m, size, align) = built::<f64>();
        let m = m.into_iter().filter(|x| x.0 != "_pad").collect();
        check_no_implicit_padding("ICDescriptor<f64>", &(m, size, align))
    }
);

/// R-313 "R-86's 64 B is its f32 instantiation": the f32 `ICDescriptor` is the generic struct at f32, not a separate
/// declaration, and an f64 value not representable at f32 survives in the f64 descriptor's fields (they are stored at
/// the `Real`'s width, so `E₀ = K_0 + V_0` can be formed at it).
fn check_f32_is_the_instantiation(f32_named: TypeId) {
    assert_eq!(
        f32_named,
        TypeId::of::<ICDescriptorOf<f32>>(),
        "ICDescriptor is not ICDescriptorOf<f32>"
    );
}

#[test]
fn qa_descriptor_f32_is_the_generic_at_f32() {
    check_f32_is_the_instantiation(TypeId::of::<ICDescriptor>());
    assert_eq!(
        size_of::<ICDescriptor>(),
        64,
        "ICDescriptor at f32 is not 64 B (R-86)"
    );
    let fine = 1.0 + f64::EPSILON;
    assert_ne!(fine as f32 as f64, fine);
    let d = ICDescriptorOf::<f64> {
        K_0: fine,
        V_0: -fine,
        ..Default::default()
    };
    assert_eq!(
        (d.K_0, d.V_0, d.K_0 + d.V_0),
        (fine, -fine, 0.0),
        "the f64 descriptor does not store its fields at f64"
    );
}

negative_control!(
    qa_descriptor_f32_is_the_generic_at_f32,
    "the f64 instantiation offered as the f32 ICDescriptor",
    expected = "ICDescriptor is not ICDescriptorOf<f32>",
    check_f32_is_the_instantiation(TypeId::of::<ICDescriptorOf<f64>>())
);

/// §3.6's size at a `Real` of `w` bytes aligned to `a`: `12w + 16`, rounded up to `a` (§3.6: a multiple of it at
/// every row, so the rounding adds nothing).
fn s3_6_size(w: usize, a: usize) -> usize {
    (12 * w + 16).next_multiple_of(a)
}

/// Each generated row's `descriptor` is §3.6's width function at the row's `Real` size and alignment.
/// A generated row: the `Real`'s name, size and alignment, and its `descriptor` as `(name, size, alignment)`.
type Row = (&'static str, usize, usize, (&'static str, usize, usize));

fn check_rows(rows: &[Row]) {
    for (real, w, a, d) in rows {
        assert_eq!(
            *d,
            ("ICDescriptor", s3_6_size(*w, *a), *a),
            "the {real} row's ICDescriptor is not dd_generation_root §3.6's width function at w = {w}"
        );
    }
}

/// REQ-PAY-017 / REQ-PAY-087: the generated rows carry `ICDescriptor`'s layout per precision, f32, f64 and the
/// DoubleF64 stub (16 B aligned to 8): §3.6's table, 64 B, 112 B and 208 B; the instantiated rows are the types'.
#[test]
fn qa_descriptor_rows_are_the_width_function() {
    let rows: Vec<_> = PAYLOAD_LAYOUTS
        .iter()
        .map(|r| (r.real, r.real_size, r.real_align, r.descriptor))
        .collect();
    assert_eq!(
        rows.iter().map(|r| (r.0, r.1, r.2)).collect::<Vec<_>>(),
        [("f32", 4, 4), ("f64", 8, 8), ("DoubleF64", 16, 8)],
        "the precision rows are not f32, f64 and the DoubleF64 stub"
    );
    check_rows(&rows);
    assert_eq!(
        rows.iter().map(|r| r.3).collect::<Vec<_>>(),
        [
            ("ICDescriptor", 64, 4),
            ("ICDescriptor", 112, 8),
            ("ICDescriptor", 208, 8)
        ],
        "the ICDescriptor rows are not §3.6's table"
    );
    assert_eq!(
        [rows[0].3, rows[1].3],
        [
            (
                "ICDescriptor",
                size_of::<ICDescriptorOf<f32>>(),
                align_of::<ICDescriptorOf<f32>>()
            ),
            (
                "ICDescriptor",
                size_of::<ICDescriptorOf<f64>>(),
                align_of::<ICDescriptorOf<f64>>()
            ),
        ],
        "an instantiated row's ICDescriptor differs from the type it describes"
    );
}

negative_control!(
    qa_descriptor_rows_are_the_width_function,
    "the DoubleF64 row's descriptor claimed at an f64's width, as a width hardcoded below the Real would give",
    expected = "is not dd_generation_root §3.6's width function",
    {
        let r = &PAYLOAD_LAYOUTS[2];
        check_rows(&[(r.real, 8, r.real_align, r.descriptor)])
    }
);
