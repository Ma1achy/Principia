//! qa's tests for TASK-M0-07, written from REQ-GEN-002, REQ-GEN-024's definition and R-242:
//!
//! - "Every ledger entry must carry the §3.8 metadata (name, location, type, scale, range, optional sentinel and tier
//!   gate, provenance, consumers), and a field without a complete entry must fail generation loudly, naming the
//!   field" (REQ-GEN-002; dd_generation_root §3.8, §5 test 6). Checked for every required key of every kind of entry
//!   (packed, scalar, vector, derived), for several incomplete entries at once, and that a refused generation runs no
//!   emitter (the driver validates, then emits: TASK-M0-07 Goal).
//! - The entry carries each §3.8 kind as written: every location, type, scale, provenance and consumer, with its
//!   sentinel and tier gate.
//! - R-242: a `Range` end is closed, open or unbounded; an entry with no name is reported as "entry #i (unnamed)".
//! - §3.8's definition (REQ-GEN-024, R-72): "`from` names the stored fields it is computed from; each must be an
//!   entry of the ledger whose location is a packed word or a scalar index, never another derived field." A derived
//!   field whose `from` is empty, names no entry, or names a derived field is not a complete entry, so generation
//!   must refuse it, naming the field.
//!
//! The fixture here is qa's own, built through the public schema; field names are distinctive (`qa_…`) so that a
//! message "naming the field" cannot match by accident.

use std::cell::Cell;

use ledger::gen::{self, Generated};
use ledger::schema::{
    Bound, Consumer, Entry, EntryBuilder, FieldType, Ledger, Location, Provenance, Range, Scale,
    Span, Word,
};
use validation::negative_control;

/// The §3.8 keys an entry must carry; `sentinel` and `tier_gate` are optional.
const REQUIRED: [&str; 7] = [
    "name",
    "location",
    "type",
    "scale",
    "range",
    "provenance",
    "consumers",
];

fn closed(lo: f64, hi: f64) -> Range {
    Range {
        lo: Bound::Closed(lo),
        hi: Bound::Closed(hi),
    }
}

/// A complete entry, every key set by hand.
fn complete(
    name: &'static str,
    location: Location,
    ty: FieldType,
    scale: Scale,
    range: Range,
    provenance: Provenance,
    consumers: &[Consumer],
) -> EntryBuilder {
    EntryBuilder {
        name: Some(name),
        location: Some(location),
        ty: Some(ty),
        scale: Some(scale),
        range: Some(range),
        sentinel: None,
        tier_gate: None,
        provenance: Some(provenance),
        consumers: Some(consumers.to_vec()),
    }
}

/// qa's ledger: a 32-bit word with a 3-bit enum, a flag and reserved bits; a u16 word; a scalar with a sentinel and
/// a tier gate; a vector `n`; and two derived fields (dd_generation_root §3.1, §3.4, §3.8).
fn qa_ledger() -> Ledger {
    let all = [Consumer::Render, Consumer::Export, Consumer::Debug];
    Ledger {
        words: vec![
            Word {
                name: "qa_descriptor",
                bits: 16,
                reserved: vec![Span {
                    offset: 4,
                    width: 12,
                }],
            },
            Word {
                name: "qa_times",
                bits: 16,
                reserved: vec![],
            },
        ],
        entries: vec![
            complete(
                "qa_state",
                Location::Packed {
                    word: "qa_descriptor",
                    offset: 0,
                    width: 3,
                },
                FieldType::UBits,
                Scale::Categorical(6),
                closed(0.0, 5.0),
                Provenance::Kernel,
                &all,
            ),
            complete(
                "qa_saturated",
                Location::Packed {
                    word: "qa_descriptor",
                    offset: 3,
                    width: 1,
                },
                FieldType::UBits,
                Scale::Flag,
                closed(0.0, 1.0),
                Provenance::Kernel,
                &all,
            ),
            complete(
                "qa_t_end_step",
                Location::Packed {
                    word: "qa_times",
                    offset: 0,
                    width: 16,
                },
                FieldType::UBits,
                Scale::Lin,
                closed(0.0, 65535.0),
                Provenance::Kernel,
                &all,
            ),
            EntryBuilder {
                sentinel: Some(-1.0),
                tier_gate: Some("qa_ftle_valid"),
                ..complete(
                    "qa_diffusion",
                    Location::Scalar(0),
                    FieldType::F32,
                    Scale::Lin,
                    Range {
                        lo: Bound::Closed(0.0),
                        hi: Bound::Unbounded,
                    },
                    Provenance::Kernel,
                    &all,
                )
            },
            complete(
                "qa_shape_vector",
                Location::Scalar(1),
                FieldType::Vector {
                    component: Box::new(FieldType::F32),
                    k: 3,
                },
                Scale::Lin,
                closed(-1.0, 1.0),
                Provenance::Decode,
                &all,
            ),
            complete(
                "qa_t_end_fraction",
                Location::Derived {
                    from: vec!["qa_t_end_step"],
                },
                FieldType::F32,
                Scale::Lin,
                closed(0.0, 1.0),
                Provenance::Cpu,
                &[Consumer::Render, Consumer::Debug],
            ),
            complete(
                "qa_retrograde",
                Location::Derived {
                    from: vec!["qa_diffusion"],
                },
                FieldType::UBits,
                Scale::Flag,
                closed(0.0, 1.0),
                Provenance::Cpu,
                &[Consumer::Render],
            ),
        ],
    }
}

/// Sets the §3.8 key `key` of `e` back to unset, field by field (independent of the builder's own helpers).
fn unset(e: &mut EntryBuilder, key: &str) {
    match key {
        "name" => e.name = None,
        "location" => e.location = None,
        "type" => e.ty = None,
        "scale" => e.scale = None,
        "range" => e.range = None,
        "provenance" => e.provenance = None,
        "consumers" => e.consumers = None,
        other => panic!("qa: no required key `{other}`"),
    }
}

/// Generation from `ledger` with no emitter is refused; its message.
fn refused(ledger: &Ledger) -> String {
    match gen::generate(ledger, &[]) {
        Ok(_) => panic!("qa: generation was not refused"),
        Err(e) => e.to_string(),
    }
}

/// Generation from `ledger` succeeds; the entries validated.
fn generates(ledger: &Ledger) -> Vec<Entry> {
    match gen::validate(ledger) {
        Ok(entries) => entries,
        Err(e) => panic!("qa: generation refused: {e}"),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-002: each required key, on each kind of entry.
// ---------------------------------------------------------------------------------------------------------------

/// For each entry and each key in `keys`, unsetting that key refuses generation, naming the field (or, without its
/// name, "entry #… (unnamed)") and the missing key.
fn check_each_missing_key_refused(keys: &[&str]) {
    let base = qa_ledger();
    for (i, e) in base.entries.iter().enumerate() {
        let field = e.name.unwrap();
        for key in keys {
            let mut ledger = base.clone();
            unset(&mut ledger.entries[i], key);
            let message = refused(&ledger);
            if *key == "name" {
                assert!(
                    message.contains("(unnamed)") && message.contains("entry #"),
                    "qa: an entry without its name is not reported as \"entry #i (unnamed)\": {message}"
                );
            } else {
                assert!(
                    message.contains(field),
                    "qa: refusal for `{key}` missing does not name the field `{field}`: {message}"
                );
            }
            assert!(
                message.contains(key),
                "qa: refusal does not name the missing key `{key}` of `{field}`: {message}"
            );
        }
    }
}

#[test]
fn qa_m007_every_required_key_missing_refuses_naming_field() {
    check_each_missing_key_refused(&REQUIRED);
}

negative_control!(
    qa_m007_every_required_key_missing_refuses_naming_field,
    "unsetting no key leaves every entry complete, so generation is not refused",
    expected = "generation was not refused",
    {
        let base = qa_ledger();
        refused(&base);
    }
);

/// The complete ledger generates, and the optional keys may be absent.
fn check_complete_generates(without_optional: bool) {
    let mut ledger = qa_ledger();
    if without_optional {
        for e in &mut ledger.entries {
            e.sentinel = None;
            e.tier_gate = None;
        }
    }
    let entries = generates(&ledger);
    assert_eq!(entries.len(), ledger.entries.len(), "qa: entries lost");
}

#[test]
fn qa_m007_complete_ledger_generates_optional_keys_absent() {
    check_complete_generates(false);
    check_complete_generates(true);
}

negative_control!(
    qa_m007_complete_ledger_generates_optional_keys_absent,
    "a ledger with one entry's scale deleted is incomplete, so it does not generate",
    expected = "generation refused",
    {
        let mut ledger = qa_ledger();
        ledger.entries[3].scale = None;
        generates(&ledger);
    }
);

/// Every incomplete entry is named, not only the first.
fn check_every_incomplete_entry_named(ledger: &Ledger, fields: &[&str]) {
    let message = refused(ledger);
    for f in fields {
        assert!(
            message.contains(f),
            "qa: refusal does not name the incomplete field `{f}`: {message}"
        );
    }
}

#[test]
fn qa_m007_every_incomplete_entry_is_named() {
    let mut ledger = qa_ledger();
    ledger.entries[0].scale = None;
    ledger.entries[4].provenance = None;
    ledger.entries[6].consumers = None;
    check_every_incomplete_entry_named(&ledger, &["qa_state", "qa_shape_vector", "qa_retrograde"]);
}

negative_control!(
    qa_m007_every_incomplete_entry_is_named,
    "a complete entry is not incomplete, so asking for it to be named must fail",
    expected = "does not name the incomplete field `qa_saturated`",
    {
        let mut ledger = qa_ledger();
        ledger.entries[0].scale = None;
        check_every_incomplete_entry_named(&ledger, &["qa_state", "qa_saturated"]);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// The driver validates before it emits.
// ---------------------------------------------------------------------------------------------------------------

thread_local! {
    /// The counting emitter's tally, per test thread (tests run in parallel).
    static EMITTED: Cell<usize> = const { Cell::new(0) };
}

/// An emitter that counts its calls and the entries it was given.
fn counting_emitter(_: &[Word], entries: &[Entry]) -> Vec<Generated> {
    EMITTED.with(|c| c.set(c.get() + 1 + entries.len() * 1000));
    vec![Generated {
        path: "qa/out.txt".into(),
        contents: format!("{}", entries.len()),
    }]
}

/// Generation from `ledger` with the counting emitter: refused ⇒ the emitter never ran; allowed ⇒ it ran once over
/// every entry.
fn check_emits_only_when_complete(ledger: &Ledger) {
    EMITTED.with(|c| c.set(0));
    let result = gen::generate(ledger, &[counting_emitter as gen::Emitter]);
    let calls = EMITTED.with(Cell::get);
    match result {
        Err(e) => assert_eq!(
            calls, 0,
            "qa: the emitter ran although generation was refused ({e})"
        ),
        Ok(files) => {
            assert_eq!(
                calls,
                1 + ledger.entries.len() * 1000,
                "qa: the emitter did not run once over every entry"
            );
            assert_eq!(files.len(), 1, "qa: the emitter's file was not returned");
        }
    }
}

#[test]
fn qa_m007_refused_generation_runs_no_emitter() {
    let mut ledger = qa_ledger();
    ledger.entries[2].range = None;
    check_emits_only_when_complete(&ledger);
    check_emits_only_when_complete(&qa_ledger());
}

negative_control!(
    qa_m007_refused_generation_runs_no_emitter,
    "an emitter run after refusal is exactly what the check guards; emulate it",
    expected = "the emitter ran although generation was refused",
    {
        EMITTED.with(|c| c.set(0));
        let mut ledger = qa_ledger();
        ledger.entries[2].range = None;
        counting_emitter(&[], &[]);
        let result = gen::generate(&ledger, &[]);
        let calls = EMITTED.with(Cell::get);
        if let Err(e) = result {
            assert_eq!(
                calls, 0,
                "qa: the emitter ran although generation was refused ({e})"
            );
        }
    }
);

// ---------------------------------------------------------------------------------------------------------------
// The entry carries each §3.8 kind as written.
// ---------------------------------------------------------------------------------------------------------------

/// A one-entry ledger for each §3.8 kind; each validated entry equals, key by key, what was written.
fn check_each_kind_carried(mangle: fn(&mut Entry)) {
    let locations = [
        Location::Packed {
            word: "qa_w",
            offset: 2,
            width: 5,
        },
        Location::Scalar(7),
        Location::Derived {
            from: vec!["qa_base"],
        },
    ];
    let types = [
        FieldType::UBits,
        FieldType::F32,
        FieldType::F16Pair,
        FieldType::Fixed16,
        FieldType::Vector {
            component: Box::new(FieldType::F16Pair),
            k: 2,
        },
    ];
    let scales = [
        Scale::Lin,
        Scale::Log,
        Scale::Cyclic,
        Scale::Diverging,
        Scale::Categorical(9),
        Scale::Flag,
    ];
    let provenances = [
        Provenance::Kernel,
        Provenance::Decode,
        Provenance::Reduction,
        Provenance::Cpu,
    ];
    let consumers = [
        Consumer::Render,
        Consumer::Export,
        Consumer::Debug,
        Consumer::Scheduler,
    ];
    let base = complete(
        "qa_base",
        Location::Scalar(0),
        FieldType::F32,
        Scale::Lin,
        closed(0.0, 1.0),
        Provenance::Kernel,
        &consumers,
    );
    let n = locations.len() * types.len() * scales.len() * provenances.len();
    for i in 0..n {
        let location = locations[i % 3].clone();
        let ty = types[(i / 3) % 5].clone();
        let scale = scales[(i / 15) % 6];
        let provenance = provenances[(i / 90) % 4];
        let who = &consumers[..1 + i % 4];
        let written = EntryBuilder {
            sentinel: Some(-(i as f64)),
            tier_gate: Some("qa_gate"),
            ..complete(
                "qa_kind",
                location.clone(),
                ty.clone(),
                scale,
                Range {
                    lo: Bound::Open(-2.0),
                    hi: Bound::Closed(i as f64),
                },
                provenance,
                who,
            )
        };
        let ledger = Ledger {
            words: vec![Word {
                name: "qa_w",
                bits: 8,
                reserved: vec![],
            }],
            entries: vec![base.clone(), written],
        };
        let mut got = generates(&ledger).remove(1);
        mangle(&mut got);
        let want = Entry {
            name: "qa_kind",
            location,
            ty,
            scale,
            range: Range {
                lo: Bound::Open(-2.0),
                hi: Bound::Closed(i as f64),
            },
            sentinel: Some(-(i as f64)),
            tier_gate: Some("qa_gate"),
            provenance,
            consumers: who.to_vec(),
        };
        assert_eq!(got, want, "qa: the entry does not carry what was written");
    }
}

#[test]
fn qa_m007_entry_carries_every_38_kind() {
    check_each_kind_carried(|_| {});
}

negative_control!(
    qa_m007_entry_carries_every_38_kind,
    "an entry whose scale changed does not carry what was written",
    expected = "the entry does not carry what was written",
    check_each_kind_carried(|e| e.scale = Scale::Categorical(10))
);

// ---------------------------------------------------------------------------------------------------------------
// R-242: range ends, and the unnamed-entry report.
// ---------------------------------------------------------------------------------------------------------------

/// Every combination of closed, open and unbounded ends is a complete range: the entry generates with it.
fn check_range_ends_accepted(ends: &[Bound]) {
    for lo in ends {
        for hi in ends {
            let mut ledger = qa_ledger();
            ledger.entries[3].range = Some(Range { lo: *lo, hi: *hi });
            let got = generates(&ledger);
            assert_eq!(
                got[3].range,
                Range { lo: *lo, hi: *hi },
                "qa: the range written is not the range carried"
            );
        }
    }
}

#[test]
fn qa_m007_range_ends_closed_open_unbounded() {
    check_range_ends_accepted(&[Bound::Closed(0.0), Bound::Open(0.0), Bound::Unbounded]);
    // `Range::int` is the closed integer range.
    assert_eq!(
        Range::int(-3, 65535),
        closed(-3.0, 65535.0),
        "qa: Range::int is not closed at both ends"
    );
}

negative_control!(
    qa_m007_range_ends_closed_open_unbounded,
    "an entry whose range is deleted does not generate",
    expected = "generation refused",
    {
        let mut ledger = qa_ledger();
        ledger.entries[3].range = None;
        generates(&ledger);
    }
);

/// The `entry #…` numbers of every "entry #… (unnamed)" in `message`, in order.
fn unnamed_indices(message: &str) -> Vec<String> {
    message
        .split("entry #")
        .skip(1)
        .filter_map(|rest| {
            let (num, tail) = rest.split_once(' ')?;
            tail.starts_with("(unnamed)").then(|| num.to_owned())
        })
        .collect()
}

/// Two entries without names, at different positions, are each reported as "entry #i (unnamed)", with different
/// `i`, in ledger order.
fn check_unnamed_reported(ledger: &Ledger) {
    let message = refused(ledger);
    let found = unnamed_indices(&message);
    assert_eq!(
        found.len(),
        2,
        "qa: not two \"entry #i (unnamed)\" reports: {message}"
    );
    let a: usize = found[0].parse().expect("qa: i is a number");
    let b: usize = found[1].parse().expect("qa: i is a number");
    assert!(
        a < b,
        "qa: the unnamed entries are not told apart by position: {message}"
    );
}

#[test]
fn qa_m007_unnamed_entry_reported_as_entry_index() {
    let mut ledger = qa_ledger();
    ledger.entries[1].name = None;
    ledger.entries[5].name = None;
    ledger.entries[5].scale = None;
    check_unnamed_reported(&ledger);
}

negative_control!(
    qa_m007_unnamed_entry_reported_as_entry_index,
    "with one unnamed entry there are not two reports",
    expected = "not two \"entry #i (unnamed)\" reports",
    {
        let mut ledger = qa_ledger();
        ledger.entries[1].name = None;
        ledger.entries[5].scale = None;
        check_unnamed_reported(&ledger);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// §3.8's derived-field definition (REQ-GEN-024): `from` names stored entries of the ledger.
// ---------------------------------------------------------------------------------------------------------------

/// A derived field `qa_t_end_fraction` whose `from` is `from` is not a complete entry: generation is refused,
/// naming it.
fn check_bad_derived_refused(from: Vec<&'static str>) {
    let mut ledger = qa_ledger();
    ledger.entries[5].location = Some(Location::Derived { from: from.clone() });
    let message = refused(&ledger);
    assert!(
        message.contains("qa_t_end_fraction"),
        "qa: refusal of derived(from: {from:?}) does not name the field: {message}"
    );
}

#[test]
fn qa_m007_derived_from_must_name_stored_entries() {
    // Names no entry of the ledger.
    check_bad_derived_refused(vec!["qa_no_such_field"]);
    // Names another derived field ("never another derived field").
    check_bad_derived_refused(vec!["qa_retrograde"]);
    // Names nothing: a derived field is computed from stored fields.
    check_bad_derived_refused(vec![]);
}

negative_control!(
    qa_m007_derived_from_must_name_stored_entries,
    "a derived field from a stored packed entry is complete, so it is not refused",
    expected = "generation was not refused",
    check_bad_derived_refused(vec!["qa_t_end_step"])
);
