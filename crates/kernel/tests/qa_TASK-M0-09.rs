//! QA tests for TASK-M0-09, written from REQ-PAY-003, REQ-PAY-008, REQ-PAY-009 and REQ-PAY-020 against their sources
//! (dd_simstate_payload §0, §1, §7; dd_generation_root §3.3a, §3.6; render contract Part 1; R-22, R-62, R-86), not
//! from the implementation. Offsets are computed from payload §1's own group widths; the corpus's quoted widths are
//! read from the docs. Each test has a registered negative control (R-176).

use std::any::type_name_of_val as ty;
use std::mem::{align_of, offset_of, size_of};

use kernel::payload::{FreeGroupWord, ICDescriptor, SimStateBase, SimStateFTLE};
use validation::negative_control;

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-008: every offset, from payload §1's field widths (phase 48 B, shadow 48 B, accumulators 16 B, drift refs
// 8 B, packed words 12 B, total_substeps 4 B, the budget word and the regularised drift 8 B: R-603, R-605,
// TASK-M3-41, closure 8 B, the collision floor's record 4 B: R-973, TASK-M3-71, and
// the escape rule's lagged n̂ register 12 B, three Reals in place of the record's 4 B of padding: TASK-M3-12), and the
// type of each field.

/// `(name, width in bytes, Rust type)` of payload §1, in order; the shadow rows only when `ftle`.
fn section_1(ftle: bool) -> Vec<(&'static str, usize, &'static str)> {
    let v = "[[f32; 2]; 3]";
    let mut rows = vec![("r", 24, v), ("p", 24, v)];
    if ftle {
        rows.extend([("r_sh", 24, v), ("p_sh", 24, v)]);
    }
    for f in ["S", "theta", "mean_y", "C_ty", "E_0", "Lz_0"] {
        rows.push((f, 4, "f32"));
    }
    for u in [
        "packed_a",
        "packed_b",
        "times",
        "total_substeps",
        "budget_bits",
        "dE_reg",
    ] {
        rows.push((u, 4, "u32"));
    }
    rows.extend([
        ("closure_min", 4, "f32"),
        ("closure_step", 2, "u16"),
        ("_reserved", 2, "u16"),
        // the collision floor's record (payload §1, R-973; TASK-M3-71), then the lagged n̂ register (TASK-M3-12)
        ("coll_floor_bits", 4, "u32"),
        ("_nhat_lag_0", 4, "f32"),
        ("_nhat_lag_1", 4, "f32"),
        ("_nhat_lag_2", 4, "f32"),
        // the re-boot's two registers (payload §1, "Payload purity"; TASK-M3-21)
        ("_dmin_d2", 4, "u32"),
        ("_march_bits", 4, "u32"),
    ]);
    rows
}

/// Payload §1's struct size: the sum of its field widths (it packs exactly at f32).
fn section_1_bytes(ftle: bool) -> usize {
    section_1(ftle).iter().map(|r| r.1).sum()
}

/// Payload §1's offsets: each field directly after the last, no padding (the struct packs exactly).
fn expected(ftle: bool) -> Vec<(&'static str, usize, &'static str)> {
    let mut at = 0;
    section_1(ftle)
        .into_iter()
        .map(|(n, w, t)| {
            let row = (n, at, t);
            at += w;
            row
        })
        .collect()
}

fn ftle_actual() -> Vec<(&'static str, usize, &'static str)> {
    let s = SimStateFTLE::default();
    macro_rules! rows {
        ($($f:ident),*) => { vec![$((stringify!($f), offset_of!(SimStateFTLE, $f), ty(&s.$f))),*] };
    }
    rows!(
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
        budget_bits,
        dE_reg,
        closure_min,
        closure_step,
        _reserved,
        coll_floor_bits,
        _nhat_lag_0,
        _nhat_lag_1,
        _nhat_lag_2,
        _dmin_d2,
        _march_bits
    )
}

fn base_actual() -> Vec<(&'static str, usize, &'static str)> {
    let s = SimStateBase::default();
    macro_rules! rows {
        ($($f:ident),*) => { vec![$((stringify!($f), offset_of!(SimStateBase, $f), ty(&s.$f))),*] };
    }
    rows!(
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
        budget_bits,
        dE_reg,
        closure_min,
        closure_step,
        _reserved,
        coll_floor_bits,
        _nhat_lag_0,
        _nhat_lag_1,
        _nhat_lag_2,
        _dmin_d2,
        _march_bits
    )
}

fn check_layout(
    name: &str,
    actual: &[(&str, usize, &str)],
    expected: &[(&str, usize, &str)],
    size: usize,
    end: usize,
) {
    assert_eq!(
        actual, expected,
        "{name}: offsets or types differ from payload §1"
    );
    assert_eq!(
        end, size,
        "{name}: payload §1 has no padding, yet size_of is not the fields' sum"
    );
}

#[test]
fn qa_payload_layout_every_offset_and_type_from_section_1() {
    check_layout(
        "SimStateFTLE",
        &ftle_actual(),
        &expected(true),
        size_of::<SimStateFTLE>(),
        section_1_bytes(true),
    );
    check_layout(
        "SimStateBase",
        &base_actual(),
        &expected(false),
        size_of::<SimStateBase>(),
        section_1_bytes(false),
    );
}

negative_control!(
    qa_payload_layout_every_offset_and_type_from_section_1,
    "SimStateBase against the FTLE table must fail",
    expected = "SimStateBase: offsets or types differ from payload §1",
    check_layout(
        "SimStateBase",
        &base_actual(),
        &expected(true),
        size_of::<SimStateBase>(),
        section_1_bytes(false)
    )
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-003, REQ-PAY-008: size_of/align_of are 168/8 and 120/8, with the budget word and the regularised drift
// (R-603, R-605; TASK-M3-41; 160/8 and 112/8 before them), with the escape rule's lagged n̂
// register (three Reals in place of 4 B of padding, TASK-M3-12; 152/8 and 104/8 before it, with the collision floor's
// 8 B, R-973, TASK-M3-71; 144/8 and 96/8 before that), and equal the widths the corpus quotes.

/// The first number of bytes after `marker` in `text`.
fn quoted_after(text: &str, marker: &str) -> usize {
    let at = text
        .find(marker)
        .unwrap_or_else(|| panic!("marker `{marker}` not in the doc"));
    let rest = &text[at + marker.len()..];
    let digits: String = rest
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().expect("a number")
}

fn doc(path: &str) -> String {
    std::fs::read_to_string(format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn check_widths(ftle: (usize, usize), base: (usize, usize)) {
    assert_eq!(ftle, (176, 8), "SimStateFTLE (size_of, align_of)");
    assert_eq!(base, (128, 8), "SimStateBase (size_of, align_of)");
    let payload = doc("docs/design/principia_dd_simstate_payload.md");
    let render = doc("docs/contracts/principia_render_contract.md");
    let quotes = [
        (
            "payload §1 SimStateFTLE",
            quoted_after(&payload, "// SimStateFTLE: "),
            ftle.0,
        ),
        (
            "payload §1 SimStateBase",
            quoted_after(&payload, "// SimStateBase: drop r_sh,p_sh"),
            base.0,
        ),
        (
            "payload §7 FTLE-on",
            quoted_after(&payload, "Per-sample: hot `SimState` "),
            ftle.0,
        ),
        (
            "payload §7 FTLE-off",
            quoted_after(&payload, "B (FTLE-on) / "),
            base.0,
        ),
        (
            "render Part 1 FTLE-on",
            quoted_after(&render, "it is **"),
            ftle.0,
        ),
        (
            "render Part 1 FTLE-off",
            quoted_after(&render, "B (FTLE-on) / "),
            base.0,
        ),
    ];
    for (place, quoted, actual) in quotes {
        println!("{place}: corpus quotes {quoted} B, size_of is {actual} B");
        assert_eq!(
            quoted, actual,
            "{place}: the corpus quote is not the generated width"
        );
    }
}

#[test]
fn qa_payload_layout_widths_are_the_corpus_quotes() {
    check_widths(
        (size_of::<SimStateFTLE>(), align_of::<SimStateFTLE>()),
        (size_of::<SimStateBase>(), align_of::<SimStateBase>()),
    );
}

negative_control!(
    qa_payload_layout_widths_are_the_corpus_quotes,
    "the retired 136/88 widths must fail",
    expected = "SimStateFTLE (size_of, align_of)",
    check_widths((136, 8), (88, 8))
);

// ---------------------------------------------------------------------------------------------------------------
// §3.3a: the word buffer's element is one vec4<u32>, 16 B; §3.6: ICDescriptor is 12 f32 in order plus 16 B padding.

/// §3.6's order (R-22, R-62).
const IC: [&str; 12] = [
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

fn ic_actual() -> Vec<(&'static str, usize, &'static str)> {
    let s = ICDescriptor::default();
    macro_rules! rows {
        ($($f:ident),*) => { vec![$((stringify!($f), offset_of!(ICDescriptor, $f), ty(&s.$f))),*] };
    }
    rows!(
        m0,
        m1,
        m2,
        q_mass,
        rho_mag,
        lambda_mag,
        rho_ratio,
        rho_angle,
        K_0,
        V_0,
        virial_ratio,
        r_min_pair_0
    )
}

fn check_ic(actual: &[(&str, usize, &str)], names: &[&str]) {
    let expected: Vec<(&str, usize, &str)> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (*n, 4 * i, "f32"))
        .collect();
    assert_eq!(
        actual, expected,
        "ICDescriptor is not §3.6's 12 × f32 in order"
    );
    assert_eq!(size_of::<ICDescriptor>(), 64, "ICDescriptor is 64 B (R-86)");
    let debug = format!("{:?}", ICDescriptor::default());
    for old in ["rho0_mag", "rho1_mag"] {
        assert!(!debug.contains(old), "ICDescriptor has `{old}`: {debug}");
    }
    let w = FreeGroupWord::default();
    assert_eq!(
        (size_of::<FreeGroupWord>(), ty(&w.free_group_word)),
        (16, "[u32; 4]"),
        "the word-buffer element is one vec4<u32>"
    );
}

#[test]
fn qa_icdescriptor_names_offsets_and_word_element() {
    check_ic(&ic_actual(), &IC);
}

negative_control!(
    qa_icdescriptor_names_offsets_and_word_element,
    "R-22's superseded rho0_mag/rho1_mag names in place of rho_mag/lambda_mag must fail",
    expected = "ICDescriptor is not §3.6's 12 × f32 in order",
    check_ic(&ic_actual(), &{
        let mut n = IC;
        n[4] = "rho0_mag";
        n[5] = "rho1_mag";
        n
    })
);
