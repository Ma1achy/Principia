//! qa's second-round tests for TASK-M0-07, written from dd_generation_root §3.8's completed definition
//! (REQ-GEN-024, R-72) and REQ-GEN-002:
//!
//! - "**`vector(type, k)`** is the type of a vector-valued field: `k ≥ 2` components, each of the scalar `type`
//!   (`u-bits`, `f32`, `f16-pair` or `fixed16`, never a vector)." Every scalar component type is accepted at `k = 2`
//!   and at §3.8's own `k` (3 for `n`, 6 for `r` and `p`); `k < 2` and a vector component are refused, naming the
//!   field, whatever the scalar inside.
//! - "`from` names the stored fields it is computed from; each must be an entry of the ledger whose location is a
//!   packed word or a scalar index, never another derived field, and `from` is never empty. It names entries by
//!   `name`, in any §3 struct ... It names ledger entries only. A sim-key parameter ... is not an entry, so it is left
//!   out of `from`." A `from` naming a sim-key parameter (`T`), the field itself, a derived field listed after it,
//!   or one bad name among good ones is refused, naming the field.
//! - "Worked entries: §3.4's derived fields and `n`" — the six entries exactly as §3.8's table gives them (location,
//!   type, scale, range, sentinel, tier gate, provenance `kernel`, consumers `[render, export, debug]`), over stored
//!   entries named as §3.8 names them, generate. Their stored inputs are §3's own and not this task's; here each is a
//!   complete scalar-index entry, which §3.8 allows ("a packed word or a scalar index").
//!
//! Names here are qa's own except the worked entries' and their inputs', which are §3.8's, since those are what the
//! table fixes.

use ledger::gen;
use ledger::schema::{
    Bound, Consumer, EntryBuilder, FieldType, Ledger, Location, Provenance, Range, Scale,
};
use validation::negative_control;

const ALL: [Consumer; 3] = [Consumer::Render, Consumer::Export, Consumer::Debug];

fn range(lo: Bound, hi: Bound) -> Range {
    Range { lo, hi }
}

fn unbounded() -> Range {
    range(Bound::Unbounded, Bound::Unbounded)
}

/// A complete entry: `name` at `location`, of type `ty`, `scale` over `range`, provenance `kernel`, consumers
/// `[render, export, debug]` (§3.8's worked entries).
fn entry(
    name: &'static str,
    location: Location,
    ty: FieldType,
    scale: Scale,
    range: Range,
) -> EntryBuilder {
    EntryBuilder {
        name: Some(name),
        location: Some(location),
        ty: Some(ty),
        scale: Some(scale),
        range: Some(range),
        sentinel: None,
        tier_gate: None,
        overflow: None,
        floor: None,
        provenance: Some(Provenance::Kernel),
        consumers: Some(ALL.to_vec()),
    }
}

fn derived(from: &[&'static str]) -> Location {
    Location::Derived {
        from: from.to_vec(),
    }
}

fn vector(component: FieldType, k: u32) -> FieldType {
    FieldType::Vector {
        component: Box::new(component),
        k,
    }
}

fn refused_message(ledger: &Ledger) -> String {
    match gen::generate(ledger, &[]) {
        Ok(_) => panic!("qa: generation was not refused"),
        Err(e) => e.to_string(),
    }
}

fn assert_refused_naming(ledger: &Ledger, fields: &[&str]) {
    let message = refused_message(ledger);
    for field in fields {
        assert!(
            message.contains(&format!("`{field}`")),
            "qa: refusal does not name `{field}`: {message}"
        );
    }
}

fn assert_generates(ledger: &Ledger) {
    if let Err(e) = gen::generate(ledger, &[]) {
        panic!("qa: generation refused: {e}");
    }
}

// ---------------------------------------------------------------------------------------------------------------
// `vector(type, k)`: k ≥ 2 scalar components.
// ---------------------------------------------------------------------------------------------------------------

const SCALARS: [FieldType; 4] = [
    FieldType::UBits,
    FieldType::F32,
    FieldType::F16Pair,
    FieldType::Fixed16,
];

/// A one-entry ledger: `qa_vec` of type `ty` at scalar index 0, `lin` over [−1, 1] per component.
fn vector_ledger(ty: FieldType) -> Ledger {
    Ledger {
        words: vec![],
        entries: vec![entry(
            "qa_vec",
            Location::Scalar(0),
            ty,
            Scale::Lin,
            range(Bound::Closed(-1.0), Bound::Closed(1.0)),
        )],
    }
}

fn check_vector_accepted(ty: FieldType) {
    assert_generates(&vector_ledger(ty));
}

fn check_vector_refused(ty: FieldType) {
    assert_refused_naming(&vector_ledger(ty), &["qa_vec"]);
}

#[test]
fn qa_m007_r2_vector_of_each_scalar_type_accepted() {
    for component in SCALARS {
        for k in [2, 3, 6] {
            check_vector_accepted(vector(component.clone(), k));
        }
    }
}

negative_control!(
    qa_m007_r2_vector_of_each_scalar_type_accepted,
    "a vector of one component is not a vector (k ≥ 2), so the accepted check must fail on it",
    expected = "generation refused",
    check_vector_accepted(vector(FieldType::F32, 1))
);

#[test]
fn qa_m007_r2_vector_needs_k_at_least_2_scalar_components() {
    for component in SCALARS {
        // k below 2, whatever the scalar.
        check_vector_refused(vector(component.clone(), 0));
        check_vector_refused(vector(component.clone(), 1));
        // A vector component ("never a vector"), at a valid outer and inner k.
        check_vector_refused(vector(vector(component.clone(), 2), 2));
        check_vector_refused(vector(vector(component, 3), 6));
    }
}

negative_control!(
    qa_m007_r2_vector_needs_k_at_least_2_scalar_components,
    "vector(fixed16, 2) is a valid vector, so the refused check must fail on it",
    expected = "generation was not refused",
    check_vector_refused(vector(FieldType::Fixed16, 2))
);

// ---------------------------------------------------------------------------------------------------------------
// `derived(from: [...])`: `from` names stored ledger entries only.
// ---------------------------------------------------------------------------------------------------------------

/// A ledger with no packed word: `qa_steps` (scalar), `qa_pos` (a stored vector), and derived `qa_frac` from `from`,
/// with a second derived field `qa_late` listed after it.
fn derived_ledger(from: &[&'static str]) -> Ledger {
    Ledger {
        words: vec![],
        entries: vec![
            entry(
                "qa_steps",
                Location::Scalar(0),
                FieldType::UBits,
                Scale::Lin,
                range(Bound::Closed(0.0), Bound::Closed(65535.0)),
            ),
            entry(
                "qa_pos",
                Location::Scalar(1),
                vector(FieldType::F32, 6),
                Scale::Lin,
                unbounded(),
            ),
            entry(
                "qa_frac",
                derived(from),
                FieldType::F32,
                Scale::Lin,
                unbounded(),
            ),
            entry(
                "qa_late",
                derived(&["qa_steps"]),
                FieldType::F32,
                Scale::Lin,
                unbounded(),
            ),
        ],
    }
}

fn check_from_refused(from: &[&'static str]) {
    assert_refused_naming(&derived_ledger(from), &["qa_frac"]);
}

fn check_from_accepted(from: &[&'static str]) {
    assert_generates(&derived_ledger(from));
}

#[test]
fn qa_m007_r2_from_names_stored_entries_only() {
    // A sim-key parameter is not an entry (§3.8: "left out of `from`").
    check_from_refused(&["T"]);
    check_from_refused(&["qa_steps", "dt"]);
    // The field itself is a derived field.
    check_from_refused(&["qa_frac"]);
    // A derived field listed after it in the ledger.
    check_from_refused(&["qa_late"]);
    // One bad name among good ones.
    check_from_refused(&["qa_steps", "qa_pos", "qa_late"]);
}

negative_control!(
    qa_m007_r2_from_names_stored_entries_only,
    "a `from` naming a stored scalar and a stored vector is valid, so the refused check must fail on it",
    expected = "generation was not refused",
    check_from_refused(&["qa_steps", "qa_pos"])
);

#[test]
fn qa_m007_r2_from_may_name_a_stored_vector() {
    // §3.8's worked entries derive from the stored vectors `r` and `p`, at a scalar index.
    check_from_accepted(&["qa_pos"]);
    check_from_accepted(&["qa_pos", "qa_steps"]);
}

negative_control!(
    qa_m007_r2_from_may_name_a_stored_vector,
    "a `from` naming a derived field is refused, so the accepted check must fail on it",
    expected = "generation refused",
    check_from_accepted(&["qa_pos", "qa_late"])
);

// ---------------------------------------------------------------------------------------------------------------
// §3.8's worked entries: §3.4's derived fields and `n`.
// ---------------------------------------------------------------------------------------------------------------

/// The stored inputs §3.8's table names, each a complete scalar-index entry (their real entries are §3's), less those
/// in `omit`; then the six worked entries as the table gives them.
fn worked_ledger(omit: &[&str]) -> Ledger {
    let stored: [(&'static str, FieldType); 11] = [
        ("t_end_step", FieldType::UBits),
        ("S", FieldType::F32),
        ("shadow", vector(FieldType::F32, 6)),
        ("r", vector(FieldType::F32, 6)),
        ("p", vector(FieldType::F32, 6)),
        ("C_ty", FieldType::F32),
        ("E_0", FieldType::F32),
        ("Lz_0", FieldType::F32),
        ("m0", FieldType::F32),
        ("m1", FieldType::F32),
        ("m2", FieldType::F32),
    ];
    let mut entries: Vec<EntryBuilder> = Vec::new();
    let mut slot = 0;
    for (name, ty) in stored {
        let width = match &ty {
            FieldType::Vector { k, .. } => *k,
            _ => 1,
        };
        if !omit.contains(&name) {
            entries.push(entry(
                name,
                Location::Scalar(slot),
                ty,
                Scale::Lin,
                unbounded(),
            ));
        }
        slot += width;
    }
    let f32 = || FieldType::F32;
    entries.push(entry(
        "t_end",
        derived(&["t_end_step"]),
        f32(),
        Scale::Lin,
        // [0, T]; T is a sim-key parameter, so the stored bound here is a stand-in for it.
        range(Bound::Closed(0.0), Bound::Closed(1.0)),
    ));
    let mut ftle = entry(
        "ftle",
        derived(&["S", "shadow", "r", "p", "t_end_step"]),
        f32(),
        Scale::Lin,
        unbounded(),
    );
    ftle.tier_gate = Some("ftle_valid");
    entries.push(ftle);
    entries.push(entry(
        "energy_drift",
        derived(&["r", "p", "m0", "m1", "m2", "E_0"]),
        f32(),
        Scale::Diverging,
        unbounded(),
    ));
    entries.push(entry(
        "Lz_drift",
        derived(&["r", "p", "Lz_0"]),
        f32(),
        Scale::Diverging,
        unbounded(),
    ));
    let mut diffusion = entry(
        "diffusion",
        derived(&["C_ty", "t_end_step"]),
        f32(),
        Scale::Lin,
        unbounded(),
    );
    diffusion.sentinel = Some(-1.0);
    entries.push(diffusion);
    entries.push(entry(
        "n",
        derived(&["r", "m0", "m1", "m2"]),
        vector(FieldType::F32, 3),
        Scale::Lin,
        range(Bound::Closed(-1.0), Bound::Closed(1.0)),
    ));
    Ledger {
        words: vec![],
        entries,
    }
}

fn check_worked_entries_generate(omit: &[&str]) {
    let ledger = worked_ledger(omit);
    let entries = match gen::validate(&ledger) {
        Ok(entries) => entries,
        Err(e) => panic!("qa: generation refused: {e}"),
    };
    let n = entries
        .iter()
        .find(|e| e.name == "n")
        .expect("qa: `n` missing");
    assert_eq!(n.ty, vector(FieldType::F32, 3), "qa: `n` is vector(f32, 3)");
    assert_eq!(
        n.location,
        derived(&["r", "m0", "m1", "m2"]),
        "qa: `n` location"
    );
}

#[test]
fn qa_m007_r2_worked_entries_generate() {
    check_worked_entries_generate(&[]);
}

negative_control!(
    qa_m007_r2_worked_entries_generate,
    "without the stored mass `m2`, `n` and `energy_drift` name a non-entry, so generation must be refused",
    expected = "generation refused",
    check_worked_entries_generate(&["m2"])
);

#[test]
fn qa_m007_r2_worked_entries_refused_without_an_input_naming_each_reader() {
    // `r` feeds `ftle`, `energy_drift`, `Lz_drift` and `n`; each is named.
    assert_refused_naming(
        &worked_ledger(&["r"]),
        &["ftle", "energy_drift", "Lz_drift", "n"],
    );
    // `t_end_step` feeds `t_end`, `ftle` and `diffusion`.
    assert_refused_naming(
        &worked_ledger(&["t_end_step"]),
        &["t_end", "ftle", "diffusion"],
    );
}

negative_control!(
    qa_m007_r2_worked_entries_refused_without_an_input_naming_each_reader,
    "with every stored input present the worked entries generate, so the refused check must fail on it",
    expected = "generation was not refused",
    assert_refused_naming(&worked_ledger(&[]), &["n"])
);
