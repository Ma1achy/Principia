//! QA tests for TASK-M0-09, written from REQ-PAY-002, REQ-PAY-005, REQ-PAY-007, REQ-PAY-009, REQ-PAY-010,
//! REQ-GEN-001 and REQ-GEN-029 against their sources (dd_simstate_payload §0–§2; dd_generation_root §3.1, §3.3a, §3.4,
//! §3.8; render contract Part 1; R-256, R-263), not from the implementation. Every expected value below is transcribed
//! from the corpus. Each test has a registered negative control (R-176).

use ledger::gen;
use ledger::schema::{Bound, Entry, FieldType, Ledger, Location, Range, Scale, Storage, Struct};
use ledger::{layout, payload};
use validation::negative_control;

fn entries() -> Vec<Entry> {
    gen::validate(&layout()).expect("the payload ledger validates")
}

fn find<'a>(entries: &'a [Entry], name: &str) -> &'a Entry {
    entries
        .iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no ledger entry `{name}`"))
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-002: the generated file is the ledger's output, byte for byte (the ledger is the single source of truth).

fn check_generated_is_emitted(on_disk: &str) {
    let files = gen::generate(&layout(), gen::EMITTERS).expect("the payload ledger generates");
    let rust = files
        .iter()
        .find(|f| f.path.ends_with("crates/kernel/src/payload/generated.rs"))
        .expect("an emitter writes crates/kernel/src/payload/generated.rs");
    assert!(
        rust.contents == on_disk,
        "the checked-in generated.rs is not the emitter's output"
    );
}

fn generated_on_disk() -> String {
    std::fs::read_to_string(format!(
        "{}/../kernel/src/payload/generated.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("read crates/kernel/src/payload/generated.rs")
}

#[test]
fn qa_payload_fields_generated_file_is_the_emitters_output() {
    check_generated_is_emitted(&generated_on_disk());
}

negative_control!(
    qa_payload_fields_generated_file_is_the_emitters_output,
    "a hand edit dropping the 8-byte alignment from the checked-in file must be caught",
    expected = "the checked-in generated.rs is not the emitter's output",
    check_generated_is_emitted(&generated_on_disk().replacen("align(8)", "align(4)", 1))
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-002: render contract Part 1's SimState and ICDescriptor names, each a ledger entry or packed word.

/// Render contract Part 1's `SimState` list (with the closure pair it quotes from payload §1) and `ICDescriptor`'s
/// twelve, as the ledger names them (`θ̃` is `theta`; the shadow is `r_sh`, `p_sh`, payload §1). The
/// `sample_descriptor` is the low half of `packed_a` (payload §2), so its fields stand for it.
const PART_1: &[&str] = &[
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
    "d_min",
    "dE_max",
    "dLz_max",
    "packed_a",
    "packed_b",
    "times",
    "t_end_step",
    "t_dmin_step",
    "total_substeps",
    "state",
    "detail",
    "saturated",
    "dmin_pair",
    "last_symbol",
    "closure_min",
    "closure_step",
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

fn check_part_1(ledger: &Ledger, names: &[&str]) {
    let known: Vec<&str> = ledger
        .entries
        .iter()
        .filter_map(|e| e.name)
        .chain(ledger.words.iter().map(|w| w.name))
        .collect();
    let missing: Vec<&&str> = names.iter().filter(|n| !known.contains(n)).collect();
    assert!(
        missing.is_empty(),
        "render contract Part 1 names not in the ledger: {missing:?}"
    );
}

#[test]
fn qa_payload_fields_part_1_names_are_all_in_the_ledger() {
    check_part_1(&layout(), PART_1);
}

negative_control!(
    qa_payload_fields_part_1_names_are_all_in_the_ledger,
    "the ledger without its closure_step entry must fail the Part 1 check",
    expected = "render contract Part 1 names not in the ledger",
    check_part_1(
        &{
            let mut l = layout();
            l.entries.retain(|e| e.name != Some("closure_step"));
            l
        },
        PART_1
    )
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-002, R-256 veto (a): the interiors of packed_a, packed_b and times at payload §2's bits.

/// Payload §2 / generation-root §3.1: `(name, word, offset, width, type)`.
fn section_2() -> Vec<(&'static str, &'static str, u32, u32, FieldType)> {
    use FieldType::{F16Pair, UBits};
    vec![
        ("state", "packed_a", 0, 3, UBits),
        ("detail", "packed_a", 3, 2, UBits),
        ("saturated", "packed_a", 5, 1, UBits),
        ("dmin_pair", "packed_a", 6, 2, UBits),
        ("last_symbol", "packed_a", 8, 2, UBits),
        ("d_min", "packed_a", 16, 16, F16Pair),
        ("dE_max", "packed_b", 0, 16, F16Pair),
        ("dLz_max", "packed_b", 16, 16, F16Pair),
        ("t_end_step", "times", 0, 16, UBits),
        ("t_dmin_step", "times", 16, 16, UBits),
    ]
}

fn check_packed(ledger: &Ledger, rows: &[(&'static str, &'static str, u32, u32, FieldType)]) {
    let entries = gen::validate(ledger).expect("validates");
    for (name, word, offset, width, ty) in rows {
        let e = find(&entries, name);
        let at = Location::Packed {
            word,
            offset: *offset,
            width: *width,
        };
        assert_eq!(
            (&e.location, &e.ty),
            (&at, ty),
            "`{name}` is not at payload §2's bits"
        );
    }
    // Descriptor bits 10–15 are reserved (payload §2), and nothing else of packed_a is.
    let a = ledger
        .words
        .iter()
        .find(|w| w.name == "packed_a")
        .expect("packed_a word");
    let reserved: Vec<(u32, u32)> = a.reserved.iter().map(|s| (s.offset, s.width)).collect();
    assert_eq!(reserved, [(10, 6)], "packed_a's reserved bits");
    for w in ["packed_a", "packed_b", "times"] {
        let bits = ledger.words.iter().find(|x| x.name == w).map(|x| x.bits);
        assert_eq!(bits, Some(32), "`{w}` is a u32 (payload §1)");
    }
}

#[test]
fn qa_payload_fields_packed_interiors_match_payload_section_2() {
    check_packed(&layout(), &section_2());
}

negative_control!(
    qa_payload_fields_packed_interiors_match_payload_section_2,
    "dE_max and dLz_max swapped halves must fail the bit check",
    expected = "`dE_max` is not at payload §2's bits",
    check_packed(&layout(), &{
        let mut rows = section_2();
        rows[6].2 = 16;
        rows
    })
);

/// Payload §2: `dmin_pair` stores `3` for unset/invalid ("`3`=unset/invalid; latched"), so its entry must admit 3,
/// in its range or as its sentinel; the payload doc is canonical over generation-root §3.1 (R-70, R-86).
fn check_dmin_pair_unset(e: &Entry) {
    let in_range = matches!(e.range.hi, Bound::Closed(x) if x >= 3.0);
    assert!(
        in_range || e.sentinel == Some(3.0),
        "dmin_pair's stored unset code 3 is neither in its range nor its sentinel: {:?}, sentinel {:?}",
        e.range,
        e.sentinel
    );
}

#[test]
fn qa_payload_fields_dmin_pair_admits_its_unset_code() {
    check_dmin_pair_unset(find(&entries(), "dmin_pair"));
}

negative_control!(
    qa_payload_fields_dmin_pair_admits_its_unset_code,
    "an entry ranged [0, 2] with no sentinel must fail",
    expected = "dmin_pair's stored unset code 3",
    check_dmin_pair_unset(&{
        let mut e = find(&entries(), "dmin_pair").clone();
        e.range = Range::int(0, 2);
        e.sentinel = None;
        e
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-005, REQ-PAY-007: exactly two buffers; SimState's members are payload §1's, in order; the word alone moves.

/// Payload §1's `SimStateFTLE` members, in order, with their storage.
fn section_1(ftle: bool) -> Vec<(&'static str, Storage)> {
    let mut m = vec![("r", Storage::Vec2x3), ("p", Storage::Vec2x3)];
    if ftle {
        m.extend([("r_sh", Storage::Vec2x3), ("p_sh", Storage::Vec2x3)]);
    }
    for f in ["S", "theta", "mean_y", "C_ty", "E_0", "Lz_0"] {
        m.push((f, Storage::F32));
    }
    for u in ["packed_a", "packed_b", "times", "total_substeps"] {
        m.push((u, Storage::U32));
    }
    m.extend([
        ("closure_min", Storage::F32),
        ("closure_step", Storage::U16),
        ("_reserved", Storage::U16),
    ]);
    m
}

fn members(s: &Struct) -> Vec<(&'static str, Storage)> {
    s.members.iter().map(|m| (m.name, m.storage)).collect()
}

fn check_two_buffers(structs: &[Struct]) {
    let mut buffers: Vec<&str> = structs.iter().filter_map(|s| s.buffer).collect();
    buffers.sort_unstable();
    buffers.dedup();
    assert_eq!(
        buffers,
        ["SimState", "word"],
        "the payload's buffers (payload §0)"
    );
    let sim: Vec<&Struct> = structs
        .iter()
        .filter(|s| s.buffer == Some("SimState"))
        .collect();
    let names: Vec<&str> = sim.iter().map(|s| s.name).collect();
    assert_eq!(
        names,
        ["SimStateFTLE", "SimStateBase"],
        "SimState's variants"
    );
    assert_eq!(
        members(sim[0]),
        section_1(true),
        "SimStateFTLE is not payload §1's"
    );
    assert_eq!(
        members(sim[1]),
        section_1(false),
        "SimStateBase is not payload §1's"
    );
    let word: Vec<&Struct> = structs
        .iter()
        .filter(|s| s.buffer == Some("word"))
        .collect();
    assert_eq!(word.len(), 1, "one word-buffer element");
    assert_eq!(
        members(word[0]).iter().map(|m| m.1).collect::<Vec<_>>(),
        [Storage::U32x4],
        "the word buffer's element is one vec4<u32> (§3.3a)"
    );
    assert_eq!(word[0].align, 16, "vec4<u32> is 16-aligned");
    for s in &sim {
        assert_eq!(s.align, 8, "{} is 8-byte aligned", s.name);
    }
}

#[test]
fn qa_payload_buffers_two_buffers_and_simstate_is_section_1() {
    check_two_buffers(&payload::structs());
}

negative_control!(
    qa_payload_buffers_two_buffers_and_simstate_is_section_1,
    "moving closure_min out of SimStateBase into its own buffer must fail",
    expected = "SimStateBase is not payload §1's",
    check_two_buffers(&{
        let mut s = payload::structs();
        let i = s[1]
            .members
            .iter()
            .position(|m| m.name == "closure_min")
            .unwrap();
        s[1].members.remove(i);
        s
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-009, REQ-PAY-010: the precision rule (payload §1): f16 only for the three latches; closure_min f32.

fn check_precision(entries: &[Entry]) {
    let f32x6 = FieldType::Vector {
        component: Box::new(FieldType::F32),
        k: 6,
    };
    for name in ["r", "p", "r_sh", "p_sh"] {
        assert_eq!(find(entries, name).ty, f32x6, "`{name}` is 12 × f32");
    }
    for name in ["S", "theta", "mean_y", "C_ty", "E_0", "Lz_0", "closure_min"] {
        assert_eq!(find(entries, name).ty, FieldType::F32, "`{name}` is f32");
    }
    let mut f16: Vec<&str> = entries
        .iter()
        .filter(|e| e.ty == FieldType::F16Pair || e.ty == FieldType::Fixed16)
        .map(|e| e.name)
        .collect();
    f16.sort_unstable();
    assert_eq!(
        f16,
        ["dE_max", "dLz_max", "d_min"],
        "only the three latches are 16-bit floats (payload §1)"
    );
}

#[test]
fn qa_payload_precision_rule_of_payload_section_1() {
    check_precision(&entries());
}

negative_control!(
    qa_payload_precision_rule_of_payload_section_1,
    "closure_min as an f16-pair is the width payload §1 rejects",
    expected = "`closure_min` is f32",
    check_precision(&{
        let mut e = entries();
        e.iter_mut().find(|e| e.name == "closure_min").unwrap().ty = FieldType::F16Pair;
        e
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-001: §3.4's table, transcribed here independently (R-256 (b): delta_*_max_abs are dE_max, dLz_max).

fn open0() -> Range {
    Range {
        lo: Bound::Open(0.0),
        hi: Bound::Unbounded,
    }
}

fn closed0() -> Range {
    Range {
        lo: Bound::Closed(0.0),
        hi: Bound::Unbounded,
    }
}

fn check_3_4(entries: &[Entry], rows: &[(&str, Scale, Option<Range>)]) {
    for (name, scale, lo_range) in rows {
        let e = find(entries, name);
        assert_eq!(e.scale, *scale, "§3.4 `{name}`'s scale");
        if let Some(r) = lo_range {
            assert_eq!(e.range, *r, "§3.4 `{name}`'s range");
        }
        // Payload §1: failed samples store 0.0 in `d_min`, a canonical sentinel (applied per R-227 / R-204).
        let want = if *name == "d_min" {
            Some(f64::INFINITY)
        } else {
            None
        };
        assert_eq!(e.sentinel, want, "§3.4 / payload §1 `{name}`'s sentinel");
    }
    for name in ["ftle", "energy_drift", "Lz_drift", "diffusion"] {
        assert!(
            matches!(find(entries, name).location, Location::Derived { .. }),
            "`{name}` is derived at read, not stored (payload §5)"
        );
    }
}

fn rows_3_4() -> Vec<(&'static str, Scale, Option<Range>)> {
    vec![
        ("t_end_step", Scale::Lin, Some(Range::int(0, 65535))),
        ("d_min", Scale::Log, Some(open0())),
        ("ftle", Scale::Lin, None),
        ("energy_drift", Scale::Diverging, None),
        ("diffusion", Scale::Lin, None),
        ("dE_max", Scale::Log, Some(closed0())),
        ("Lz_drift", Scale::Diverging, None),
        ("dLz_max", Scale::Log, Some(closed0())),
        ("E_0", Scale::Diverging, None),
        ("Lz_0", Scale::Diverging, None),
        ("closure_min", Scale::Log, Some(closed0())),
        ("closure_step", Scale::Lin, Some(Range::int(0, 65535))),
    ]
}

#[test]
fn qa_catalogue_scalars_section_3_4_table() {
    check_3_4(&entries(), &rows_3_4());
}

negative_control!(
    qa_catalogue_scalars_section_3_4_table,
    "Lz_drift on a lin scale is not §3.4's diverging",
    expected = "§3.4 `Lz_drift`'s scale",
    check_3_4(&entries(), &[("Lz_drift", Scale::Lin, None)])
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-029: the gate accepts `floor` only as a sim-key parameter name (R-263; §3.8).

fn check_floor_refused(floor: &'static str) {
    let mut ledger = layout();
    let e = ledger
        .entries
        .iter_mut()
        .find(|e| e.name == Some("Lz_drift"))
        .expect("Lz_drift entry");
    e.floor = Some(floor);
    let refused = gen::generate(&ledger, gen::EMITTERS).is_err();
    assert!(refused, "floor `{floor}` was accepted");
}

#[test]
fn qa_payload_floor_refuses_a_packed_word() {
    // `packed_a` is a stored SimState field (payload §1), declared in the ledger as a word, not a sim-key parameter.
    check_floor_refused("packed_a");
}

negative_control!(
    qa_payload_floor_refuses_a_packed_word,
    "eps_L is a sim-key parameter, so it is accepted and the refusal check must fail",
    expected = "floor `eps_L` was accepted",
    check_floor_refused("eps_L")
);

#[test]
fn qa_payload_floor_refuses_a_number() {
    // §3.8: the floor "is named, not stored"; `1e-6` is eps_L's default value (integrator contract), not a name.
    check_floor_refused("1e-6");
}

negative_control!(
    qa_payload_floor_refuses_a_number,
    "eps_L is a sim-key parameter, so it is accepted and the refusal check must fail",
    expected = "floor `eps_L` was accepted",
    check_floor_refused("eps_L")
);

fn check_floors(entries: &[Entry]) {
    let mut floored: Vec<(&str, &str)> = entries
        .iter()
        .filter_map(|e| e.floor.map(|f| (e.name, f)))
        .collect();
    floored.sort_unstable();
    assert_eq!(
        floored,
        [("Lz_drift", "eps_L"), ("energy_drift", "eps_E")],
        "the floors are §3.4's two and no others"
    );
}

#[test]
fn qa_payload_floor_only_the_two_drifts() {
    check_floors(&entries());
}

negative_control!(
    qa_payload_floor_only_the_two_drifts,
    "a floor on E_0 is not one of §3.4's",
    expected = "the floors are §3.4's two and no others",
    check_floors(&{
        let mut e = entries();
        e.iter_mut().find(|e| e.name == "E_0").unwrap().floor = Some("eps_E");
        e
    })
);
