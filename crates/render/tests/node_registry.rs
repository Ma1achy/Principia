//! One registry of stain nodes: a node names any shipped source by its id, looked up in the registry the scan made,
//! and there is no other table. A name no file gives the slot is refused naming it, never read as some other source;
//! two files a node would call by one name are refused naming both.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use render::assemble::{self, Kind, Node, Occupant, Stain};
use render::registry::{self, RegistryError};
use validation::negative_control;

/// A fresh scratch directory named after `case`.
fn scratch(case: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "node_registry_{case}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

fn write(root: &Path, files: &[(&str, &str)]) {
    for (rel, text) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        std::fs::write(&path, text).expect("the file");
    }
}

const VIEW: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5); }\n";

// ── Every shipped source is named by its id ──────────────────────────────────────────────────────────────────────

/// Each of `entries` is the shipped source its name resolves to, and the assembler places that file's text for it.
fn check_named(entries: &[registry::Entry]) {
    let mut count = 0;
    for e in entries {
        let found =
            registry::named(e.slot, e.name()).unwrap_or_else(|why| panic!("{}: {why}", e.id));
        assert_eq!(found.id, e.id, "`{}` names another file", e.name());
        let text = assemble::occupant_source(e.slot, &Occupant::BuiltIn(e.name().to_owned()))
            .unwrap_or_else(|why| panic!("{}: {why}", e.id));
        assert_eq!(text.as_deref(), Some(e.source.as_str()), "{}", e.id);
        count += 1;
    }
    eprintln!("{count} shipped sources, each named by its id");
    assert!(count > 8, "only {count} sources: the scan found too few");
}

#[test]
fn every_shipped_source_is_named_by_its_id() {
    check_named(&registry::registry().expect("the registry"));
}

negative_control!(
    every_shipped_source_is_named_by_its_id,
    "a source the shipped table does not hold, as a second table would list one",
    expected = "no colour source is named",
    {
        let mut entries = registry::registry().expect("the registry");
        let e = entries
            .iter_mut()
            .find(|e| e.id == "colour/margin")
            .expect("margin");
        e.id = "colour/margin_elsewhere".into();
        check_named(&entries);
    }
);

#[cfg(feature = "controls")]
mod every_shipped_source_wrong_file {
    use super::*;
    negative_control!(
        every_shipped_source_is_named_by_its_id,
        "a lookup that gives the slot's first file for every name, known names too",
        expected = "names another file",
        registry::probe::any_file_in(|| check_named(&registry::registry().expect("the registry")))
    );
}

// ── An unknown id is refused, naming it ──────────────────────────────────────────────────────────────────────────

/// Each name no file gives its slot is refused, the refusal naming it, both when looked up and when a stain holds it.
fn check_unknown_refused() {
    for (kind, name) in [
        (Kind::Colour, "no_such_source"),
        (Kind::Combiner, "debug/generated/no_such_field"),
        (Kind::Post, "multiply_"),
    ] {
        match registry::named(kind, name) {
            Err(RegistryError::Unknown(k, n)) => assert_eq!((k, n.as_str()), (kind, name)),
            other => panic!("`{name}` was not refused: {other:?}"),
        }
        let why = assemble::occupant_source(kind, &Occupant::BuiltIn(name.to_owned()))
            .expect_err("an unknown name was not refused")
            .to_string();
        assert!(
            why.contains(&format!("`{name}`")),
            "the refusal does not name `{name}`: {why}"
        );
    }
    // A known name in a slot not its own is refused as that file's wrong slot.
    assert_eq!(
        registry::named(Kind::Colour, "multiply").map(|e| e.id.clone()),
        Err(RegistryError::WrongSlot(
            "combiner/multiply".into(),
            Kind::Combiner,
            Kind::Colour
        ))
    );
    let stain = Stain::new(vec![
        Node {
            kind: Kind::Combiner,
            occupant: Occupant::BuiltIn("no_such_combiner".into()),
            inputs: vec![None, None],
        },
        Node {
            kind: Kind::Out,
            occupant: Occupant::None,
            inputs: vec![Some(0)],
        },
    ]);
    let why = stain
        .expect_err("a stain naming no source was not refused")
        .to_string();
    assert!(why.contains("`no_such_combiner`"), "{why}");
}

#[test]
fn an_unknown_id_is_refused_naming_it() {
    check_unknown_refused();
}

negative_control!(
    an_unknown_id_is_refused_naming_it,
    "a lookup that reads an unknown name as the slot's first source",
    expected = "was not refused",
    registry::probe::unknown_allowed_in(check_unknown_refused)
);

// ── An id present twice is refused, naming both files ────────────────────────────────────────────────────────────

/// A render crate with one generated view twice: hand-written under the occupant directory and generated, both
/// `debug/generated/x`; scanned, it is refused naming both files. And an embedded table holding one name twice.
fn check_twice_refused(case: &str) {
    let root = scratch(case);
    let hand = "shaders/wgsl/frag/debug/generated/x.wgsl";
    let generated = "frag/debug/generated/x.wgsl";
    write(&root, &[(hand, VIEW), (generated, VIEW)]);
    let got = registry::scan(&root);
    let _ = std::fs::remove_dir_all(&root);
    match got {
        Err(e @ RegistryError::Twice(..)) => {
            let why = e.to_string();
            assert!(
                why.contains(hand)
                    && why.contains(generated)
                    && why.contains("`debug/generated/x`"),
                "the refusal does not name both files and the id: {why}"
            );
        }
        other => panic!("an id given twice was not refused: {other:?}"),
    }
    let table = [
        ("colour/a", "shaders/wgsl/frag/colour/a.wgsl", VIEW),
        ("colour/a", "shaders/wgsl/frag/colour/a_copy.wgsl", VIEW),
    ];
    match registry::embedded(Path::new("render"), &table) {
        Err(e @ RegistryError::Twice(..)) => {
            let why = e.to_string();
            assert!(
                why.contains("colour/a.wgsl") && why.contains("colour/a_copy.wgsl"),
                "the refusal does not name both files: {why}"
            );
        }
        other => panic!("an id given twice was not refused: {other:?}"),
    }
}

#[test]
fn an_id_present_twice_is_refused_naming_both_files() {
    check_twice_refused("twice");
}

negative_control!(
    an_id_present_twice_is_refused_naming_both_files,
    "a scan that lets two files of one id through",
    expected = "an id given twice was not refused",
    registry::probe::twice_allowed(|| check_twice_refused("twice_control"))
);
