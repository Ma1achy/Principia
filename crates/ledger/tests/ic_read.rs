//! The read side's `ICDescriptor` reader (render contract Part 1's `ctx.ic`; RQ-227; TASK-M1-08): `ic_read` fills the
//! `ic.<member>` fields a stain reads, each an `ICDescriptor` member, and a name that is none is refused, the padding
//! included, as a read-side field that is none is (R-378); the read side's fields end with them. Each test has a
//! registered negative control (R-176).

use ledger::gen::{self, read};
use validation::negative_control;

/// [`read::wgsl_for`] at the full tier for `fields`: the error, or `None` where it generates.
fn refusal(fields: &[&str]) -> Option<String> {
    let l = ledger::layout();
    let entries = gen::validate(&l).expect("the ledger validates");
    read::wgsl_for(&l.words, &entries, read::Tier::FULL, fields).err()
}

/// Each of `names` is refused as no read-side field, and each of `members` generates.
fn check_ic_names(names: &[&str], members: &[&str]) {
    for &name in names {
        assert_eq!(
            refusal(&["d_min", name]),
            Some(format!("`{name}` is no read-side field")),
            "`{name}` was not refused"
        );
    }
    for &name in members {
        assert_eq!(refusal(&[name]), None, "`{name}` was refused");
    }
}

#[test]
fn ic_read_fields_are_the_icdescriptors_members() {
    check_ic_names(
        &["ic._pad", "ic.nope", "ic.", "ic.m0.x"],
        &["ic.m0", "ic.r_min_pair_0"],
    );
    assert_eq!(
        read::ic_fields().len(),
        12,
        "the ICDescriptor's twelve members, its padding left out"
    );
}

negative_control!(
    ic_read_fields_are_the_icdescriptors_members,
    "a member of the ICDescriptor is not refused",
    expected = "`ic.m0` was not refused",
    check_ic_names(&["ic.m0"], &[])
);

/// `fields` is every field [`read::wgsl_for`] fills: each read-side member, the word's four components, then the
/// `ICDescriptor`'s members as `ic.<member>`; and each generates.
fn check_fields(fields: &[String]) {
    let l = ledger::layout();
    let entries = gen::validate(&l).expect("the ledger validates");
    let mut want: Vec<String> = read::members(&l.words, &entries)
        .into_iter()
        .map(|m| m.name)
        .collect();
    want.extend(["word.x", "word.y", "word.z", "word.w"].map(str::to_owned));
    want.extend(read::ic_fields());
    assert_eq!(fields, want, "the read side's fields");
    let all: Vec<&str> = fields.iter().map(String::as_str).collect();
    assert_eq!(
        refusal(&all),
        None,
        "the read side's fields do not generate"
    );
}

#[test]
fn ic_read_fields_follow_the_members_and_the_word() {
    let l = ledger::layout();
    let entries = gen::validate(&l).expect("the ledger validates");
    check_fields(&read::fields(&l.words, &entries));
}

negative_control!(
    ic_read_fields_follow_the_members_and_the_word,
    "the fields without the ICDescriptor's are not the read side's",
    expected = "the read side's fields",
    {
        let l = ledger::layout();
        let entries = gen::validate(&l).expect("the ledger validates");
        let mut fields = read::fields(&l.words, &entries);
        fields.retain(|f| !f.starts_with("ic."));
        check_fields(&fields);
    }
);
