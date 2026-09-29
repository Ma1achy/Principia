//! The generated payload structs against dd_simstate_payload §1 (REQ-PAY-003, REQ-PAY-008, REQ-PAY-009, REQ-PAY-020):
//! sizes, alignments and every field offset; the closure fields in both variants; `ICDescriptor`'s names. It prints
//! the recomputed widths and payload §7's memory rows (R-40, R-59 D6).

use std::mem::{align_of, offset_of, size_of};

use kernel::payload::{FreeGroupWord, ICDescriptor, SimStateBase, SimStateFTLE};
use validation::negative_control;

/// Payload §1's `SimStateFTLE` offsets; `SimStateBase` is the same less `r_sh` and `p_sh`, 48 B earlier from `S`.
const FTLE: [(&str, usize); 17] = [
    ("r", 0),
    ("p", 24),
    ("r_sh", 48),
    ("p_sh", 72),
    ("S", 96),
    ("theta", 100),
    ("mean_y", 104),
    ("C_ty", 108),
    ("E_0", 112),
    ("Lz_0", 116),
    ("packed_a", 120),
    ("packed_b", 124),
    ("times", 128),
    ("total_substeps", 132),
    ("closure_min", 136),
    ("closure_step", 140),
    ("_reserved", 142),
];

fn ftle_offsets() -> Vec<(&'static str, usize)> {
    macro_rules! at {
        ($($f:ident),*) => { vec![$((stringify!($f), offset_of!(SimStateFTLE, $f))),*] };
    }
    at!(
        r,
        p,
        r_sh,
        p_sh,
        S,
        theta,
        mean_y,
        C_ty,
        E_0,
        Lz_0,
        packed_a,
        packed_b,
        times,
        total_substeps,
        closure_min,
        closure_step,
        _reserved
    )
}

fn base_offsets() -> Vec<(&'static str, usize)> {
    macro_rules! at {
        ($($f:ident),*) => { vec![$((stringify!($f), offset_of!(SimStateBase, $f))),*] };
    }
    at!(
        r,
        p,
        S,
        theta,
        mean_y,
        C_ty,
        E_0,
        Lz_0,
        packed_a,
        packed_b,
        times,
        total_substeps,
        closure_min,
        closure_step,
        _reserved
    )
}

/// `actual` offsets equal `expected`, field by field.
fn check_offsets(actual: &[(&str, usize)], expected: &[(&str, usize)]) {
    assert_eq!(actual, expected, "field offsets differ from payload §1");
}

/// Payload §1's table for `SimStateBase`: `FTLE` without the shadow, shifted down 48 B from `S`.
fn base_expected() -> Vec<(&'static str, usize)> {
    let shadow = |n: &str| n == "r_sh" || n == "p_sh";
    let shift = |(n, at): (&'static str, usize)| (n, if at >= 96 { at - 48 } else { at });
    FTLE.into_iter()
        .filter(|(n, _)| !shadow(n))
        .map(shift)
        .collect()
}

#[test]
fn payload_layout_offsets_match_payload_section_1() {
    check_offsets(&ftle_offsets(), &FTLE);
    check_offsets(&base_offsets(), &base_expected());
}

negative_control!(
    payload_layout_offsets_match_payload_section_1,
    "SimStateBase's offsets are not SimStateFTLE's, so the offset check must fail on them",
    expected = "field offsets differ from payload §1",
    check_offsets(&base_offsets(), &FTLE)
);

/// `(size, align)` is `expected` for `name`.
fn check_size(name: &str, actual: (usize, usize), expected: (usize, usize)) {
    assert_eq!(actual, expected, "{name}: (size_of, align_of)");
}

#[test]
fn payload_layout_sizes_are_144_96_and_8_aligned() {
    let ftle = (size_of::<SimStateFTLE>(), align_of::<SimStateFTLE>());
    let base = (size_of::<SimStateBase>(), align_of::<SimStateBase>());
    println!(
        "recomputed widths: SimStateFTLE {ftle:?}, SimStateBase {base:?} (size_of, align_of) B"
    );
    check_size("SimStateFTLE", ftle, (144, 8));
    check_size("SimStateBase", base, (96, 8));
    let word = (size_of::<FreeGroupWord>(), align_of::<FreeGroupWord>());
    check_size("FreeGroupWord", word, (16, 16));
    let ic = (size_of::<ICDescriptor>(), align_of::<ICDescriptor>());
    check_size("ICDescriptor", ic, (64, 4));
}

negative_control!(
    payload_layout_sizes_are_144_96_and_8_aligned,
    "136 B is the pre-closure width (D6), so the size check must fail on it",
    expected = "SimStateFTLE: (size_of, align_of)",
    check_size("SimStateFTLE", (size_of::<SimStateFTLE>(), 8), (136, 8))
);

/// Payload §7's hot and word memory, decimal GB to three places, for `bytes × copies × pixels`.
fn gb(bytes: usize, copies: usize, (w, h): (usize, usize)) -> String {
    format!("{:.3}", (bytes * copies * w * h) as f64 / 1e9)
}

/// The memory rows computed from the generated widths equal payload §7's quotes.
fn check_memory(ftle: usize, base: usize) {
    let (hd, uhd) = ((1920, 1080), (3840, 2160));
    let word = size_of::<FreeGroupWord>();
    let rows = [
        (
            "1080p E=3 FTLE-on",
            gb(ftle, 4, hd),
            gb(word, 4, hd),
            "1.194",
            "0.133",
        ),
        (
            "1080p E=3 FTLE-off",
            gb(base, 4, hd),
            gb(word, 4, hd),
            "0.796",
            "0.133",
        ),
        (
            "1080p E=1 FTLE-off",
            gb(base, 2, hd),
            gb(word, 2, hd),
            "0.398",
            "0.066",
        ),
        (
            "4K E=3 FTLE-on",
            gb(ftle, 4, uhd),
            gb(word, 4, uhd),
            "4.778",
            "0.531",
        ),
        (
            "4K E=1 FTLE-off",
            gb(base, 2, uhd),
            gb(word, 2, uhd),
            "1.593",
            "0.265",
        ),
    ];
    for (config, hot, word, quoted_hot, quoted_word) in rows {
        println!("payload §7 {config}: hot {hot} GB, word {word} GB");
        assert_eq!(
            (hot.as_str(), word.as_str()),
            (quoted_hot, quoted_word),
            "payload §7 row {config}"
        );
    }
}

#[test]
fn payload_layout_memory_rows_match_payload_section_7() {
    check_memory(size_of::<SimStateFTLE>(), size_of::<SimStateBase>());
}

negative_control!(
    payload_layout_memory_rows_match_payload_section_7,
    "at the old 136 B width the rows are 1.128 GB, so the memory check must fail",
    expected = "payload §7 row 1080p E=3 FTLE-on",
    check_memory(136, 88)
);

/// `closure_min` is an f32 and `closure_step` a u16 in `(closure_min, closure_step)`.
fn check_closure(min: &str, step: &str) {
    assert_eq!((min, step), ("f32", "u16"), "closure field types");
}

fn type_of<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

#[test]
fn closure_fields_are_f32_and_u16_in_both_variants() {
    let (f, b) = (SimStateFTLE::default(), SimStateBase::default());
    check_closure(type_of(&f.closure_min), type_of(&f.closure_step));
    check_closure(type_of(&b.closure_min), type_of(&b.closure_step));
}

negative_control!(
    closure_fields_are_f32_and_u16_in_both_variants,
    "`_reserved` is the u16 beside closure_step, not an f32, so the type check must fail on it",
    expected = "closure field types",
    check_closure(type_of(&SimStateFTLE::default()._reserved), "u16")
);

/// `names` holds `rho_mag` and `lambda_mag` and neither `rho0_mag` nor `rho1_mag`.
fn check_ic_names(names: &str) {
    let has = |n: &str| names.contains(&format!(" {n}: "));
    assert!(
        has("rho_mag") && has("lambda_mag"),
        "ICDescriptor lacks rho_mag or lambda_mag: {names}"
    );
    assert!(
        !has("rho0_mag") && !has("rho1_mag"),
        "ICDescriptor has rho0_mag or rho1_mag: {names}"
    );
}

#[test]
fn icdescriptor_names_are_rho_mag_and_lambda_mag() {
    check_ic_names(&format!("{:?}", ICDescriptor::default()));
}

negative_control!(
    icdescriptor_names_are_rho_mag_and_lambda_mag,
    "R-22's superseded names must fail the name check",
    expected = "ICDescriptor has rho0_mag or rho1_mag",
    check_ic_names("ICDescriptor { rho_mag: 0.0, lambda_mag: 0.0, rho0_mag: 0.0 }")
);
