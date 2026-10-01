//! QA tests for TASK-M0-14, written from REQ-PAY-017 ("the payload width must be a function of the Real type, never
//! hardcoded to f32"; verify: instantiate for f32 and f64 and the DoubleF64 stub row, widths derive from
//! `size_of::<Real>()`, layout generated per precision) and REQ-PAY-087's definition in dd_simstate_payload §1 ("The
//! payload as a function of `Real`", "The f64 layout", "The DoubleF64 stub row"), with R-86 (padding declared, never
//! implicit). Every offset and size below is §1's text, not the generated file's. Each test registers a negative
//! control (R-176).

use std::any::TypeId;
use std::mem::{align_of, offset_of, size_of, size_of_val};

use kernel::payload::{
    PayloadReal, SimStateBase, SimStateBaseOf, SimStateFTLE, SimStateFTLEOf, PAYLOAD_LAYOUTS,
};
use validation::negative_control;

/// Every member offset of `SimStateFTLEOf<R>`, in declaration order, and the struct's size and alignment.
fn ftle_layout<R: PayloadReal>() -> (Vec<(&'static str, usize)>, usize, usize) {
    type S<R> = SimStateFTLEOf<R>;
    let offsets = vec![
        ("r", offset_of!(S<R>, r)),
        ("p", offset_of!(S<R>, p)),
        ("r_sh", offset_of!(S<R>, r_sh)),
        ("p_sh", offset_of!(S<R>, p_sh)),
        ("S", offset_of!(S<R>, S)),
        ("theta", offset_of!(S<R>, theta)),
        ("mean_y", offset_of!(S<R>, mean_y)),
        ("C_ty", offset_of!(S<R>, C_ty)),
        ("E_0", offset_of!(S<R>, E_0)),
        ("Lz_0", offset_of!(S<R>, Lz_0)),
        ("packed_a", offset_of!(S<R>, packed_a)),
        ("packed_b", offset_of!(S<R>, packed_b)),
        ("times", offset_of!(S<R>, times)),
        ("total_substeps", offset_of!(S<R>, total_substeps)),
        ("closure_min", offset_of!(S<R>, closure_min)),
        ("closure_step", offset_of!(S<R>, closure_step)),
        ("_reserved", offset_of!(S<R>, _reserved)),
    ];
    (offsets, size_of::<S<R>>(), align_of::<S<R>>())
}

/// Every member offset of `SimStateBaseOf<R>`, in declaration order, and the struct's size and alignment.
fn base_layout<R: PayloadReal>() -> (Vec<(&'static str, usize)>, usize, usize) {
    type S<R> = SimStateBaseOf<R>;
    let offsets = vec![
        ("r", offset_of!(S<R>, r)),
        ("p", offset_of!(S<R>, p)),
        ("S", offset_of!(S<R>, S)),
        ("theta", offset_of!(S<R>, theta)),
        ("mean_y", offset_of!(S<R>, mean_y)),
        ("C_ty", offset_of!(S<R>, C_ty)),
        ("E_0", offset_of!(S<R>, E_0)),
        ("Lz_0", offset_of!(S<R>, Lz_0)),
        ("packed_a", offset_of!(S<R>, packed_a)),
        ("packed_b", offset_of!(S<R>, packed_b)),
        ("times", offset_of!(S<R>, times)),
        ("total_substeps", offset_of!(S<R>, total_substeps)),
        ("closure_min", offset_of!(S<R>, closure_min)),
        ("closure_step", offset_of!(S<R>, closure_step)),
        ("_reserved", offset_of!(S<R>, _reserved)),
    ];
    (offsets, size_of::<S<R>>(), align_of::<S<R>>())
}

/// dd_simstate_payload §1's f32 `SimStateFTLE` (the WGSL block: 48 + 48 + 16 + 8 + 12 + 4 + 8 = 144 B, aligned to 8).
const FTLE_F32: ([usize; 17], usize, usize) = (
    [
        0, 24, 48, 72, 96, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 142,
    ],
    144,
    8,
);
/// §1's f32 `SimStateBase`: the same less the 48 B shadow, 96 B.
const BASE_F32: ([usize; 15], usize, usize) = (
    [0, 24, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88, 92, 94],
    96,
    8,
);
/// §1 "The f64 layout": `SimStateFTLE` at `r` 0 … `_reserved` 266, `_tail` 268, 272 B, aligned to 8.
const FTLE_F64: ([usize; 17], usize, usize) = (
    [
        0, 48, 96, 144, 192, 200, 208, 216, 224, 232, 240, 244, 248, 252, 256, 264, 266,
    ],
    272,
    8,
);
/// §1 "The f64 layout": `SimStateBase` "the same less `r_sh` and `p_sh`, 96 B earlier from `S`; `_tail` at 172:
/// 176 B".
const BASE_F64: ([usize; 15], usize, usize) = (
    [
        0, 48, 96, 104, 112, 120, 128, 136, 144, 148, 152, 156, 160, 168, 170,
    ],
    176,
    8,
);

/// `got`'s offsets, size and alignment are `want`'s, for the struct named `what`.
fn check_layout<const N: usize>(
    what: &str,
    got: (Vec<(&'static str, usize)>, usize, usize),
    want: ([usize; N], usize, usize),
) {
    let offsets: Vec<usize> = got.0.iter().map(|(_, o)| *o).collect();
    assert_eq!(
        (offsets.as_slice(), got.1, got.2),
        (&want.0[..], want.1, want.2),
        "{what}'s layout is not dd_simstate_payload §1's (members {:?})",
        got.0
    );
}

/// REQ-PAY-087 / REQ-PAY-017: every member offset, the size and the alignment of both structs at f32 (§1's WGSL
/// block, unchanged by genericity) and at f64 (§1 "The f64 layout").
#[test]
fn qa_payload_layouts_at_f32_and_f64_are_payload_s1() {
    check_layout("SimStateFTLE<f32>", ftle_layout::<f32>(), FTLE_F32);
    check_layout("SimStateBase<f32>", base_layout::<f32>(), BASE_F32);
    check_layout("SimStateFTLE<f64>", ftle_layout::<f64>(), FTLE_F64);
    check_layout("SimStateBase<f64>", base_layout::<f64>(), BASE_F64);
    // The declared tail sits where §1 puts it: after `_reserved`, at 268 and 172 at f64, at the end at f32.
    assert_eq!(
        [
            offset_of!(SimStateFTLEOf<f64>, _tail),
            offset_of!(SimStateBaseOf<f64>, _tail),
            offset_of!(SimStateFTLEOf<f32>, _tail),
            offset_of!(SimStateBaseOf<f32>, _tail),
        ],
        [268, 172, 144, 96],
        "the declared tail is not where dd_simstate_payload §1 puts it"
    );
}

negative_control!(
    qa_payload_layouts_at_f32_and_f64_are_payload_s1,
    "the f64 instantiation held to the f32 layout, as a payload hardcoded to f32 would be",
    expected = "layout is not dd_simstate_payload §1's",
    check_layout("SimStateFTLE<f64>", ftle_layout::<f64>(), FTLE_F32)
);

/// Each member's byte size in `SimStateFTLEOf<R>`, `_tail` included, in declaration order.
fn ftle_member_sizes<R: PayloadReal>() -> Vec<(&'static str, usize)> {
    let s = SimStateFTLEOf::<R>::default();
    vec![
        ("r", size_of_val(&s.r)),
        ("p", size_of_val(&s.p)),
        ("r_sh", size_of_val(&s.r_sh)),
        ("p_sh", size_of_val(&s.p_sh)),
        ("S", size_of_val(&s.S)),
        ("theta", size_of_val(&s.theta)),
        ("mean_y", size_of_val(&s.mean_y)),
        ("C_ty", size_of_val(&s.C_ty)),
        ("E_0", size_of_val(&s.E_0)),
        ("Lz_0", size_of_val(&s.Lz_0)),
        ("packed_a", size_of_val(&s.packed_a)),
        ("packed_b", size_of_val(&s.packed_b)),
        ("times", size_of_val(&s.times)),
        ("total_substeps", size_of_val(&s.total_substeps)),
        ("closure_min", size_of_val(&s.closure_min)),
        ("closure_step", size_of_val(&s.closure_step)),
        ("_reserved", size_of_val(&s._reserved)),
        ("_tail", size_of_val(&s._tail)),
    ]
}

/// Each member's byte size in `SimStateBaseOf<R>`, `_tail` included, in declaration order.
fn base_member_sizes<R: PayloadReal>() -> Vec<(&'static str, usize)> {
    let s = SimStateBaseOf::<R>::default();
    vec![
        ("r", size_of_val(&s.r)),
        ("p", size_of_val(&s.p)),
        ("S", size_of_val(&s.S)),
        ("theta", size_of_val(&s.theta)),
        ("mean_y", size_of_val(&s.mean_y)),
        ("C_ty", size_of_val(&s.C_ty)),
        ("E_0", size_of_val(&s.E_0)),
        ("Lz_0", size_of_val(&s.Lz_0)),
        ("packed_a", size_of_val(&s.packed_a)),
        ("packed_b", size_of_val(&s.packed_b)),
        ("times", size_of_val(&s.times)),
        ("total_substeps", size_of_val(&s.total_substeps)),
        ("closure_min", size_of_val(&s.closure_min)),
        ("closure_step", size_of_val(&s.closure_step)),
        ("_reserved", size_of_val(&s._reserved)),
        ("_tail", size_of_val(&s._tail)),
    ]
}

/// R-86: the members, the declared tail included, fill the struct: no implicit padding at `what`'s precision.
fn check_no_implicit_padding(what: &str, members: &[(&'static str, usize)], size: usize) {
    let sum: usize = members.iter().map(|(_, s)| s).sum();
    assert_eq!(
        sum,
        size,
        "{what} has {} B of implicit padding (members {members:?})",
        size as isize - sum as isize
    );
}

/// R-86 and §1 "Layout at a `Real` of `w` bytes": the tail padding each width needs is declared, `(size − end)/4`
/// u32s, empty at f32; nothing is implicit at either instantiation.
#[test]
fn qa_payload_padding_is_declared_at_every_instantiation() {
    check_no_implicit_padding(
        "SimStateFTLE<f32>",
        &ftle_member_sizes::<f32>(),
        size_of::<SimStateFTLEOf<f32>>(),
    );
    check_no_implicit_padding(
        "SimStateBase<f32>",
        &base_member_sizes::<f32>(),
        size_of::<SimStateBaseOf<f32>>(),
    );
    check_no_implicit_padding(
        "SimStateFTLE<f64>",
        &ftle_member_sizes::<f64>(),
        size_of::<SimStateFTLEOf<f64>>(),
    );
    check_no_implicit_padding(
        "SimStateBase<f64>",
        &base_member_sizes::<f64>(),
        size_of::<SimStateBaseOf<f64>>(),
    );
    let tails = [
        size_of_val(&SimStateFTLEOf::<f32>::default()._tail),
        size_of_val(&SimStateBaseOf::<f32>::default()._tail),
        size_of_val(&SimStateFTLEOf::<f64>::default()._tail),
        size_of_val(&SimStateBaseOf::<f64>::default()._tail),
    ];
    assert_eq!(
        tails,
        [0, 0, 4, 4],
        "the declared tails are not §1's (empty at f32, one u32 at f64)"
    );
}

negative_control!(
    qa_payload_padding_is_declared_at_every_instantiation,
    "the f64 members without their declared tail leave 4 B unaccounted",
    expected = "B of implicit padding",
    {
        let members: Vec<_> = ftle_member_sizes::<f64>()
            .into_iter()
            .filter(|(n, _)| *n != "_tail")
            .collect();
        check_no_implicit_padding(
            "SimStateFTLE<f64>",
            &members,
            size_of::<SimStateFTLEOf<f64>>(),
        )
    }
);

/// §1 "Which fields follow `Real`": the members that are f32 in §1's block. Every other member keeps its width.
const FOLLOWS_REAL: [&str; 11] = [
    "r",
    "p",
    "r_sh",
    "p_sh",
    "S",
    "theta",
    "mean_y",
    "C_ty",
    "E_0",
    "Lz_0",
    "closure_min",
];

/// Between the instantiation `narrow` (`wn`-byte `Real`) and `wide` (`ww`-byte), each member that follows the `Real`
/// scales by `ww / wn` exactly and each other one (but the tail) keeps its width.
fn check_scaling(
    narrow: &[(&'static str, usize)],
    wn: usize,
    wide: &[(&'static str, usize)],
    ww: usize,
) {
    assert_eq!(narrow.len(), wide.len());
    for (&(name, n), &(_, w)) in narrow.iter().zip(wide) {
        if name == "_tail" {
            continue;
        }
        if FOLLOWS_REAL.contains(&name) {
            assert_eq!(
                w * wn,
                n * ww,
                "`{name}` is {n} B at a {wn}-B Real and {w} B at a {ww}-B Real: it does not follow the Real"
            );
        } else {
            assert_eq!(w, n, "`{name}` changed width with the Real ({n} B, {w} B)");
        }
    }
}

/// REQ-PAY-017: the width is a function of the `Real`: from f32 to f64 exactly §1's float members double and the
/// packed words, counter and u16s keep their widths; the f32 structs are the generic ones at f32, not a separate
/// hardcoded declaration.
#[test]
fn qa_payload_width_follows_the_real() {
    check_scaling(
        &ftle_member_sizes::<f32>(),
        size_of::<f32>(),
        &ftle_member_sizes::<f64>(),
        size_of::<f64>(),
    );
    check_scaling(
        &base_member_sizes::<f32>(),
        size_of::<f32>(),
        &base_member_sizes::<f64>(),
        size_of::<f64>(),
    );
    // The fixed-width members hold §1's widths: u32 words and counter, u16 step index and reserve.
    let fixed: Vec<(&str, usize)> = ftle_member_sizes::<f64>()
        .into_iter()
        .filter(|(n, _)| !FOLLOWS_REAL.contains(n) && *n != "_tail")
        .collect();
    assert_eq!(
        fixed,
        [
            ("packed_a", 4),
            ("packed_b", 4),
            ("times", 4),
            ("total_substeps", 4),
            ("closure_step", 2),
            ("_reserved", 2)
        ],
        "the fixed-width members are not §1's"
    );
    assert_eq!(
        (TypeId::of::<SimStateFTLE>(), TypeId::of::<SimStateBase>()),
        (
            TypeId::of::<SimStateFTLEOf<f32>>(),
            TypeId::of::<SimStateBaseOf<f32>>()
        ),
        "the f32 structs are not the generic structs at f32"
    );
}

negative_control!(
    qa_payload_width_follows_the_real,
    "the f64 member sizes given as the narrow side against themselves at twice the width",
    expected = "does not follow the Real",
    check_scaling(
        &ftle_member_sizes::<f64>(),
        size_of::<f32>(),
        &ftle_member_sizes::<f64>(),
        size_of::<f64>(),
    )
);

/// §1 "Layout at a `Real` of `w` bytes": a struct storing `reals` `Real`s ends at `reals·w + 20` and is aligned to 8
/// or to the `Real`'s alignment if greater.
fn s1_size(reals: usize, w: usize, align: usize) -> usize {
    (reals * w + 20).next_multiple_of(align.max(8))
}

/// A generated row: the `Real`'s name, size and alignment, and each struct's `(name, size, alignment)` at it.
type Row = (
    &'static str,
    usize,
    usize,
    [(&'static str, usize, usize); 2],
);

/// Each row of the generated table is §1's width function at the row's `Real` size and alignment.
fn check_rows(rows: &[Row]) {
    for (real, w, a, structs) in rows {
        let want = [
            ("SimStateFTLE", s1_size(31, *w, *a), *a.max(&8)),
            ("SimStateBase", s1_size(19, *w, *a), *a.max(&8)),
        ];
        assert_eq!(
            structs, &want,
            "the {real} row is not dd_simstate_payload §1's width function at w = {w}"
        );
    }
}

/// REQ-PAY-017 / REQ-PAY-087: the generated precision rows are f32, f64 and the DoubleF64 stub (a pair of f64s, 16 B
/// aligned to 8; §1 "The DoubleF64 stub row"); each row's sizes are §1's function of its width (520 B and 328 B for
/// DoubleF64); and the instantiated rows are the instantiated types' sizes.
#[test]
fn qa_payload_rows_are_the_width_function() {
    let rows: Vec<Row> = PAYLOAD_LAYOUTS
        .iter()
        .map(|r| (r.real, r.real_size, r.real_align, r.structs))
        .collect();
    let heads: Vec<_> = PAYLOAD_LAYOUTS
        .iter()
        .map(|r| (r.real, r.real_size, r.real_align, r.instantiated))
        .collect();
    assert_eq!(
        heads,
        [
            ("f32", size_of::<f32>(), align_of::<f32>(), true),
            ("f64", size_of::<f64>(), align_of::<f64>(), true),
            ("DoubleF64", 2 * size_of::<f64>(), align_of::<f64>(), false),
        ],
        "the precision rows are not f32, f64 and the DoubleF64 stub"
    );
    check_rows(&rows);
    assert_eq!(
        (rows[2].3[0].1, rows[2].3[1].1),
        (520, 328),
        "the DoubleF64 row is not §1's 520 B and 328 B"
    );
    assert_eq!(
        [rows[0].3, rows[1].3],
        [
            [
                (
                    "SimStateFTLE",
                    size_of::<SimStateFTLEOf<f32>>(),
                    align_of::<SimStateFTLEOf<f32>>()
                ),
                (
                    "SimStateBase",
                    size_of::<SimStateBaseOf<f32>>(),
                    align_of::<SimStateBaseOf<f32>>()
                ),
            ],
            [
                (
                    "SimStateFTLE",
                    size_of::<SimStateFTLEOf<f64>>(),
                    align_of::<SimStateFTLEOf<f64>>()
                ),
                (
                    "SimStateBase",
                    size_of::<SimStateBaseOf<f64>>(),
                    align_of::<SimStateBaseOf<f64>>()
                ),
            ],
        ],
        "an instantiated row differs from the type it describes"
    );
}

negative_control!(
    qa_payload_rows_are_the_width_function,
    "the DoubleF64 row's sizes claimed at an f64's width, as a width hardcoded below the Real would give",
    expected = "is not dd_simstate_payload §1's width function",
    {
        let r = &PAYLOAD_LAYOUTS[2];
        check_rows(&[(r.real, 8, r.real_align, r.structs)])
    }
);
