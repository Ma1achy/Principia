//! The payload ledger (dd_simstate_payload §0–§1; dd_generation_root §3.1, §3.3a–§3.6; render contract Part 1): the
//! generated Rust structs against the ledger, precision, §3.4's catalogue metadata, the two payload buffers, no
//! per-pair entry, unique names, and the drifts' `floor` (REQ-PAY-002, -005, -007, -010, REQ-GEN-001, REQ-GEN-029,
//! REQ-RENDER-002).

mod support;

use ledger::gen::{self, rust};
use ledger::schema::{Bound, Entry, FieldType, Ledger, Overflow, Range, Scale, Struct};
use ledger::{layout, payload};
use support::{check_generates, check_refused_naming, entry, fixture};
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

/// Each struct's `pub name: type,` lines in `generated`, by struct.
fn parsed(generated: &str) -> Vec<(String, Vec<String>)> {
    let mut structs: Vec<(String, Vec<String>)> = Vec::new();
    for line in generated.lines() {
        if let Some(name) = line.strip_prefix("pub struct ") {
            structs.push((name.trim_end_matches(" {").to_owned(), Vec::new()));
        } else if let (Some(field), Some(s)) = (line.strip_prefix("    pub "), structs.last_mut()) {
            s.1.push(field.to_owned());
        }
    }
    structs
}

/// The generated file on disk matches `structs` member by member, and `structs` matches the ledger.
fn check_fields(structs: &[Struct]) {
    let on_disk = std::fs::read_to_string(format!(
        "{}/../../{}",
        env!("CARGO_MANIFEST_DIR"),
        rust::PATH
    ))
    .expect("read the generated file");
    let expected: Vec<(String, Vec<String>)> = structs
        .iter()
        .map(|s| {
            let fields = s
                .members
                .iter()
                .map(|m| format!("{}: {},", m.name, m.storage.rust()))
                .collect();
            (s.name.to_owned(), fields)
        })
        .collect();
    assert_eq!(
        parsed(&on_disk),
        expected,
        "generated structs differ from the ledger's"
    );
    let found = rust::check(structs, &layout().words, &entries(), payload::PENDING);
    assert!(
        found.is_empty(),
        "structs differ from the ledger: {found:?}"
    );
}

#[test]
fn payload_fields_generated_structs_match_the_ledger() {
    check_generates(&layout());
    check_fields(&payload::structs());
}

negative_control!(
    payload_fields_generated_structs_match_the_ledger,
    "SimStateFTLE with closure_min as u32 differs from the generated file, so the field check must fail",
    expected = "generated structs differ from the ledger's",
    check_fields(&{
        let mut s = payload::structs();
        s[0].members[14].storage = ledger::schema::Storage::U32;
        s
    })
);

/// With `SimStateFTLE`'s members `i` and `i + 1` swapped, the check finds `count` members off the ledger.
fn check_swapped(i: usize, count: usize) {
    let mut s = payload::structs();
    s[0].members.swap(i, i + 1);
    let found = rust::check(&s, &layout().words, &entries(), payload::PENDING);
    assert_eq!(found.len(), count, "members off the ledger: {found:?}");
}

#[test]
fn payload_fields_check_finds_members_off_their_slots() {
    check_swapped(4, 2);
}

negative_control!(
    payload_fields_check_finds_members_off_their_slots,
    "swapping S and theta moves both, so the check must fail on a count of none",
    expected = "members off the ledger",
    check_swapped(4, 0)
);

/// The payload ledger with `mean_y` at scalar slot `slot`, or without its entry if `slot` is `None`; generation from it
/// is refused, naming each of `names` (dd_generation_root §3.8: "A field without a complete entry fails generation
/// loudly").
fn check_mean_y_refused(slot: Option<u32>, names: &[&str]) {
    let mut ledger = layout();
    match slot {
        Some(slot) => {
            let e = ledger
                .entries
                .iter_mut()
                .find(|e| e.name == Some("mean_y"))
                .expect("the payload ledger has `mean_y`");
            e.location = Some(ledger::schema::Location::Scalar(slot));
        }
        None => ledger.entries.retain(|e| e.name != Some("mean_y")),
    }
    check_refused_naming(&ledger, names);
}

#[test]
fn payload_fields_generation_refuses_a_member_off_its_slot() {
    check_mean_y_refused(
        Some(36),
        &["`SimStateFTLE.mean_y` is at byte 104, not slot 36"],
    );
}

negative_control!(
    payload_fields_generation_refuses_a_member_off_its_slot,
    "mean_y at its own slot 26 is where SimStateFTLE stores it, so generation must not be refused",
    expected = "generation was not refused",
    check_mean_y_refused(Some(26), &["`SimStateFTLE.mean_y`"])
);

#[test]
fn payload_fields_generation_refuses_a_member_with_no_entry() {
    check_mean_y_refused(
        None,
        &[
            "`SimStateFTLE.mean_y` has no ledger entry or word",
            "`SimStateBase.mean_y` has no ledger entry or word",
        ],
    );
}

negative_control!(
    payload_fields_generation_refuses_a_member_with_no_entry,
    "with its entry kept at its own slot, mean_y is tied to the ledger, so generation must not be refused",
    expected = "generation was not refused",
    check_mean_y_refused(
        Some(26),
        &["`SimStateFTLE.mean_y` has no ledger entry or word"]
    )
);

/// Render contract Part 1's `SimState` and `ICDescriptor` field lists, each a ledger entry, word or member.
const PART_1: [&str; 33] = [
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
    "t_end_step",
    "t_dmin_step",
    "total_substeps",
    "state",
    "detail",
    "saturated",
    "dmin_pair",
    "last_symbol",
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

fn check_subset(names: &[&str]) {
    let entries = entries();
    let missing: Vec<_> = names
        .iter()
        .filter(|n| !entries.iter().any(|e| e.name == **n))
        .collect();
    assert!(
        missing.is_empty(),
        "render contract Part 1 names absent from the ledger: {missing:?}"
    );
}

#[test]
fn payload_fields_render_contract_part_1_is_a_subset() {
    check_subset(&PART_1);
}

negative_control!(
    payload_fields_render_contract_part_1_is_a_subset,
    "rho0_mag is R-22's superseded name, so the subset check must fail on it",
    expected = "render contract Part 1 names absent from the ledger",
    check_subset(&["rho0_mag"])
);

/// Each of `names` is f32 in the ledger, and no generated artefact names bf16.
fn check_f32(names: &[&str]) {
    let entries = entries();
    for name in names {
        let ty = &find(&entries, name).ty;
        let f32s = FieldType::Vector {
            component: Box::new(FieldType::F32),
            k: 6,
        };
        assert!(
            *ty == FieldType::F32 || *ty == f32s,
            "`{name}` is {ty:?}, not f32"
        );
    }
    for file in gen::generate(&layout(), gen::EMITTERS).expect("generates") {
        assert!(
            !file.contents.to_lowercase().contains("bf16"),
            "{} names bf16",
            file.path.display()
        );
    }
}

#[test]
fn payload_precision_hot_fields_are_f32_and_no_bf16() {
    check_f32(&[
        "r", "p", "r_sh", "p_sh", "S", "theta", "mean_y", "C_ty", "E_0", "Lz_0",
    ]);
}

negative_control!(
    payload_precision_hot_fields_are_f32_and_no_bf16,
    "d_min is an f16 latch, so the f32 check must fail on it",
    expected = "`d_min` is F16Pair, not f32",
    check_f32(&["d_min"])
);

fn any() -> Range {
    Range {
        lo: Bound::Unbounded,
        hi: Bound::Unbounded,
    }
}

fn above(lo: Bound) -> Range {
    Range {
        lo,
        hi: Bound::Unbounded,
    }
}

/// §3.4's rows: its name (the ledger's, R-70), scale, range and sentinel.
fn section_3_4() -> Vec<(&'static str, Scale, Range, Option<f64>)> {
    vec![
        ("t_end_step", Scale::Lin, Range::int(0, 65535), None),
        (
            "d_min",
            Scale::Log,
            above(Bound::Open(0.0)),
            Some(f64::INFINITY),
        ),
        ("ftle", Scale::Lin, any(), None),
        ("energy_drift", Scale::Diverging, any(), None),
        ("diffusion", Scale::Lin, any(), None),
        ("dE_max", Scale::Log, above(Bound::Closed(0.0)), None),
        ("Lz_drift", Scale::Diverging, any(), None),
        ("dLz_max", Scale::Log, above(Bound::Closed(0.0)), None),
        ("E_0", Scale::Diverging, any(), None),
        ("Lz_0", Scale::Diverging, any(), None),
        ("closure_min", Scale::Log, above(Bound::Closed(0.0)), None),
        ("closure_step", Scale::Lin, Range::int(0, 65535), None),
    ]
}

fn check_catalogue(rows: &[(&str, Scale, Range, Option<f64>)]) {
    let entries = entries();
    for (name, scale, range, sentinel) in rows {
        let e = find(&entries, name);
        assert_eq!(
            (e.scale, e.range, e.sentinel),
            (*scale, *range, *sentinel),
            "§3.4 row `{name}`"
        );
    }
    for name in ["d_min", "dE_max", "dLz_max"] {
        assert_eq!(
            find(&entries, name).overflow,
            Some(Overflow::Saturate),
            "`{name}` saturates"
        );
    }
    assert_eq!(
        find(&entries, "ftle").tier_gate,
        Some("ftle_valid"),
        "ftle's tier gate"
    );
}

#[test]
fn catalogue_scalars_match_section_3_4() {
    check_catalogue(&section_3_4());
}

negative_control!(
    catalogue_scalars_match_section_3_4,
    "a sequential-log energy_drift is §6's broken view, so the catalogue check must fail on it",
    expected = "§3.4 row `energy_drift`",
    check_catalogue(&[("energy_drift", Scale::Log, any(), None)])
);

/// `structs` has exactly two payload buffers, `SimState` and `word`, of fixed size, and both `SimState` variants hold
/// the closure fields.
fn check_buffers(structs: &[Struct]) {
    let mut buffers: Vec<_> = structs.iter().filter_map(|s| s.buffer).collect();
    buffers.dedup();
    assert_eq!(buffers, ["SimState", "word"], "payload buffers");
    let sizes: Vec<_> = structs
        .iter()
        .filter(|s| s.buffer.is_some())
        .map(|s| rust::offsets(s).1)
        .collect();
    assert_eq!(sizes, [144, 96, 16], "payload element sizes, fixed in t");
    for s in structs.iter().filter(|s| s.buffer == Some("SimState")) {
        let has = |n: &str| s.members.iter().any(|m| m.name == n);
        assert!(
            has("closure_min") && has("closure_step"),
            "{} lacks the closure fields",
            s.name
        );
    }
}

#[test]
fn payload_buffers_are_simstate_and_word() {
    check_buffers(&payload::structs());
}

negative_control!(
    payload_buffers_are_simstate_and_word,
    "closure in a parallel buffer is a third buffer, so the buffer check must fail on it",
    expected = "payload buffers",
    check_buffers(&{
        let mut s = payload::structs();
        s[3].buffer = Some("closure");
        s
    })
);

/// None of R-38's per-pair names is a ledger entry, word or struct member.
fn check_no_per_pair(ledger: &Ledger, structs: &[Struct]) {
    let mut names: Vec<&str> = ledger.entries.iter().filter_map(|e| e.name).collect();
    names.extend(ledger.words.iter().map(|w| w.name));
    names.extend(
        structs
            .iter()
            .flat_map(|s| s.members.iter().map(|m| m.name)),
    );
    for pair in [
        "enc_01",
        "enc_02",
        "enc_12",
        "encounter_count",
        "dominant_pair",
    ] {
        assert!(!names.contains(&pair), "`{pair}` is in the ledger (R-38)");
    }
}

#[test]
fn no_per_pair_entry_in_the_ledger() {
    check_no_per_pair(&layout(), &payload::structs());
}

negative_control!(
    no_per_pair_entry_in_the_ledger,
    "a dominant_pair entry must fail the per-pair check",
    expected = "`dominant_pair` is in the ledger (R-38)",
    check_no_per_pair(
        &{
            let mut l = layout();
            l.entries[0].name = Some("dominant_pair");
            l
        },
        &payload::structs()
    )
);

/// The fixture with `field` renamed to `to`: generation is refused naming `names`.
fn check_renamed_refused(field: &str, to: &'static str, names: &[&str]) {
    let mut ledger = fixture();
    entry(&mut ledger, field).name = Some(to);
    check_refused_naming(&ledger, names);
}

#[test]
fn gate_refuses_a_name_on_two_entries() {
    check_renamed_refused(
        "fx_detail",
        "fx_enum",
        &["field `fx_enum` has more than one entry"],
    );
    check_renamed_refused(
        "fx_flag",
        "fx_u16",
        &["`from` names `fx_u16`, not exactly one"],
    );
}

negative_control!(
    gate_refuses_a_name_on_two_entries,
    "a fresh name duplicates nothing, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_renamed_refused("fx_detail", "fx_other", &["more than one entry"])
);

/// Each named entry of `entries` carries its `floor` (§3.4, §3.8, R-263).
fn check_floors(entries: &[Entry], floors: &[(&str, &str)]) {
    for &(name, floor) in floors {
        assert_eq!(find(entries, name).floor, Some(floor), "`{name}`'s floor");
    }
}

#[test]
fn payload_floor_drifts_carry_eps_e_and_eps_l() {
    check_floors(
        &entries(),
        &[("energy_drift", "eps_E"), ("Lz_drift", "eps_L")],
    );
}

negative_control!(
    payload_floor_drifts_carry_eps_e_and_eps_l,
    "the floors swapped between the drifts must fail the floor check",
    expected = "`energy_drift`'s floor",
    check_floors(
        &entries(),
        &[("energy_drift", "eps_L"), ("Lz_drift", "eps_E")],
    )
);

/// The payload ledger with `energy_drift`'s `floor` set to `floor`: generation is refused naming `names`.
fn check_floor_refused(floor: &'static str, names: &[&str]) {
    let mut ledger = layout();
    entry(&mut ledger, "energy_drift").floor = Some(floor);
    check_refused_naming(&ledger, names);
}

#[test]
fn payload_floor_gate_refuses_a_ledger_field_or_constant() {
    check_floor_refused(
        "E_0",
        &["field `energy_drift`: `floor` `E_0` names a ledger entry"],
    );
    check_floor_refused(
        "f16_finite_max",
        &["`floor` `f16_finite_max` names a register constant"],
    );
    check_floor_refused("", &["field `energy_drift`: `floor` `` is empty"]);
}

negative_control!(
    payload_floor_gate_refuses_a_ledger_field_or_constant,
    "eps_E is a sim-key parameter, so the gate passes it and the refusal check must fail",
    expected = "generation was not refused",
    check_floor_refused("eps_E", &["floor"])
);
