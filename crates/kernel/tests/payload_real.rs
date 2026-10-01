//! The payload as a function of the `Real` (REQ-PAY-017; philosophy §7.1, §7.7; dd_simstate_payload §1;
//! dd_generation_root §3.6; R-313): the `SimState` structs and `ICDescriptor` instantiated at f32 and f64, every width
//! derived from `size_of::<Real>()`, and the layout generated per precision, the DoubleF64 stub row included.

use std::mem::{align_of, offset_of, size_of, size_of_val};

use kernel::payload::{
    ICDescriptor, ICDescriptorOf, PayloadLayout, PayloadReal, SimStateBase, SimStateBaseOf,
    SimStateFTLE, SimStateFTLEOf, PAYLOAD_LAYOUTS,
};
use validation::negative_control;

/// The generated precision row named `real`.
fn row(real: &str) -> PayloadLayout {
    *PAYLOAD_LAYOUTS
        .iter()
        .find(|r| r.real == real)
        .unwrap_or_else(|| panic!("no precision row `{real}`"))
}

/// A `SimState` struct's size at a `Real` of `w` bytes, from dd_simstate_payload §1's member list: `reals` members
/// of the `Real` (6 per vec2 group, one per f32 scalar), then 16 B of packed words and counter and 4 B of u16s, the
/// whole rounded up to 8.
fn width(reals: usize, w: usize) -> usize {
    (reals * w + 16 + 4).next_multiple_of(8)
}

/// `ICDescriptor`'s size at a `Real` of `w` bytes aligned to `a`, from dd_generation_root §3.6: twelve `Real`s, then
/// 16 B of declared padding, the whole rounded up to `a`.
fn descriptor_width(w: usize, a: usize) -> usize {
    (12 * w + 16).next_multiple_of(a)
}

/// `SimStateFTLE` stores 31 `Real`s (four vec2 groups, six accumulators and drift references, `closure_min`),
/// `SimStateBase` 19 (no shadow).
const FTLE_REALS: usize = 31;
const BASE_REALS: usize = 19;

/// Each widened member of `SimStateFTLEOf<R>` is a multiple of `size_of::<R>()`, each other member keeps its width,
/// and both structs' sizes and alignments are `row`'s and [`width`]'s at `size_of::<R>()`.
fn check_instantiated<R: PayloadReal>(row: &PayloadLayout) {
    let w = size_of::<R>();
    assert_eq!(
        (row.real, row.real_size, row.real_align),
        (R::NAME, w, align_of::<R>()),
        "{}'s precision row differs from the Real it instantiates",
        R::NAME
    );
    let s = SimStateFTLEOf::<R>::default();
    for (name, size, reals) in [
        ("r", size_of_val(&s.r), 6),
        ("p", size_of_val(&s.p), 6),
        ("r_sh", size_of_val(&s.r_sh), 6),
        ("p_sh", size_of_val(&s.p_sh), 6),
        ("S", size_of_val(&s.S), 1),
        ("theta", size_of_val(&s.theta), 1),
        ("mean_y", size_of_val(&s.mean_y), 1),
        ("C_ty", size_of_val(&s.C_ty), 1),
        ("E_0", size_of_val(&s.E_0), 1),
        ("Lz_0", size_of_val(&s.Lz_0), 1),
        ("closure_min", size_of_val(&s.closure_min), 1),
    ] {
        assert_eq!(size, reals * w, "`{name}` does not widen with {}", R::NAME);
    }
    for (name, size, fixed) in [
        ("packed_a", size_of_val(&s.packed_a), 4),
        ("packed_b", size_of_val(&s.packed_b), 4),
        ("times", size_of_val(&s.times), 4),
        ("total_substeps", size_of_val(&s.total_substeps), 4),
        ("closure_step", size_of_val(&s.closure_step), 2),
        ("_reserved", size_of_val(&s._reserved), 2),
    ] {
        assert_eq!(size, fixed, "`{name}` changed width at {}", R::NAME);
    }
    let sizes = [
        (
            size_of::<SimStateFTLEOf<R>>(),
            align_of::<SimStateFTLEOf<R>>(),
        ),
        (
            size_of::<SimStateBaseOf<R>>(),
            align_of::<SimStateBaseOf<R>>(),
        ),
    ];
    let generated = [
        (row.structs[0].1, row.structs[0].2),
        (row.structs[1].1, row.structs[1].2),
    ];
    assert_eq!(
        sizes,
        generated,
        "{}'s layout differs from its generated row",
        R::NAME
    );
    assert_eq!(
        [sizes[0].0, sizes[1].0],
        [width(FTLE_REALS, w), width(BASE_REALS, w)],
        "{}'s sizes do not derive from size_of::<Real>()",
        R::NAME
    );
}

/// `ICDescriptorOf<R>`'s twelve fields are each one `R`, its declared padding keeps its 16 B, and its size and
/// alignment are `row`'s `descriptor` and §3.6's width at `size_of::<R>()` (R-313).
fn check_descriptor<R: PayloadReal>(row: &PayloadLayout) {
    let w = size_of::<R>();
    let d = ICDescriptorOf::<R>::default();
    let fields = [
        ("m0", size_of_val(&d.m0)),
        ("m1", size_of_val(&d.m1)),
        ("m2", size_of_val(&d.m2)),
        ("q_mass", size_of_val(&d.q_mass)),
        ("rho_mag", size_of_val(&d.rho_mag)),
        ("lambda_mag", size_of_val(&d.lambda_mag)),
        ("rho_ratio", size_of_val(&d.rho_ratio)),
        ("rho_angle", size_of_val(&d.rho_angle)),
        ("K_0", size_of_val(&d.K_0)),
        ("V_0", size_of_val(&d.V_0)),
        ("virial_ratio", size_of_val(&d.virial_ratio)),
        ("r_min_pair_0", size_of_val(&d.r_min_pair_0)),
    ];
    for (name, size) in fields {
        assert_eq!(
            size,
            w,
            "ICDescriptor's `{name}` does not widen with {}",
            R::NAME
        );
    }
    assert_eq!(
        (size_of_val(&d._pad), offset_of!(ICDescriptorOf<R>, _pad)),
        (16, 12 * w),
        "ICDescriptor's declared padding is not 16 B after its twelve fields at {}",
        R::NAME
    );
    let actual = (
        "ICDescriptor",
        size_of::<ICDescriptorOf<R>>(),
        align_of::<ICDescriptorOf<R>>(),
    );
    assert_eq!(
        actual,
        row.descriptor,
        "{}'s ICDescriptor differs from its generated row",
        R::NAME
    );
    assert_eq!(
        actual.1,
        descriptor_width(w, align_of::<R>()),
        "{}'s ICDescriptor size does not derive from size_of::<Real>()",
        R::NAME
    );
}

#[test]
fn payload_real_generic() {
    // The rows, f32 first, and which are instantiated: f32 and f64 only (R-265); DoubleF64 is a stub row.
    let rows: Vec<(&str, bool)> = PAYLOAD_LAYOUTS
        .iter()
        .map(|r| (r.real, r.instantiated))
        .collect();
    assert_eq!(
        rows,
        [("f32", true), ("f64", true), ("DoubleF64", false)],
        "the precision rows differ from dd_simstate_payload §1's"
    );
    check_instantiated::<f32>(&row("f32"));
    check_instantiated::<f64>(&row("f64"));
    // The f32 instantiation is payload §1's layout, and keeps the struct's name.
    assert_eq!(
        (size_of::<SimStateFTLE>(), size_of::<SimStateBase>()),
        (144, 96)
    );
    // dd_simstate_payload §1's f64 layout: members at their offsets, then 4 B of declared tail padding.
    let f64_offsets = [
        offset_of!(SimStateFTLEOf<f64>, S),
        offset_of!(SimStateFTLEOf<f64>, packed_a),
        offset_of!(SimStateFTLEOf<f64>, closure_min),
        offset_of!(SimStateFTLEOf<f64>, closure_step),
        offset_of!(SimStateFTLEOf<f64>, _tail),
    ];
    assert_eq!(
        f64_offsets,
        [192, 240, 256, 264, 268],
        "the f64 SimStateFTLE offsets differ from payload §1"
    );
    // The DoubleF64 stub: a pair of f64s, 16 B aligned to 8, laid out by the width function alone.
    let dd = row("DoubleF64");
    assert_eq!(
        (dd.real_size, dd.real_align),
        (16, 8),
        "DoubleF64 is not a pair of f64s"
    );
    assert_eq!(
        [dd.structs[0].1, dd.structs[1].1],
        [
            width(FTLE_REALS, dd.real_size),
            width(BASE_REALS, dd.real_size)
        ],
        "the DoubleF64 row's sizes do not derive from its width"
    );
    assert_eq!([dd.structs[0].1, dd.structs[1].1], [520, 328]);
}

/// `ICDescriptor` as a function of the `Real` (R-313; dd_generation_root §3.6), run with `payload_real_generic` by the
/// acceptance filter: instantiated at f32 and f64, every width from `size_of::<Real>()`, the DoubleF64 stub row by the
/// width function alone.
#[test]
fn payload_real_generic_descriptor() {
    check_descriptor::<f32>(&row("f32"));
    check_descriptor::<f64>(&row("f64"));
    let dd = row("DoubleF64");
    // ICDescriptor (dd_generation_root §3.6; R-313): 64 B at f32 (R-86), 112 B at f64, 208 B at the DoubleF64 stub.
    assert_eq!(
        size_of::<ICDescriptor>(),
        64,
        "ICDescriptor at f32 is not 64 B (R-86)"
    );
    assert_eq!(
        [row("f32").descriptor, row("f64").descriptor, dd.descriptor],
        [
            ("ICDescriptor", 64, 4),
            ("ICDescriptor", 112, 8),
            (
                "ICDescriptor",
                descriptor_width(dd.real_size, dd.real_align),
                8
            )
        ],
        "the ICDescriptor rows differ from dd_generation_root §3.6"
    );
    assert_eq!(dd.descriptor.1, 208);
}

negative_control!(
    payload_real_generic,
    "the f64 instantiation checked against the f32 row must fail",
    expected = "precision row differs from the Real it instantiates",
    check_instantiated::<f64>(&row("f32"))
);

negative_control!(
    payload_real_generic_descriptor,
    "the f64 ICDescriptor checked against the f32 row must fail",
    expected = "ICDescriptor differs from its generated row",
    check_descriptor::<f64>(&row("f32"))
);
