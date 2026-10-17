//! The WGSL written at runtime, checked in (render contract Part 4, "The occupant rules `cargo xtask lint wgsl`
//! checks"; TASK-M1-15): a field ramp's colour occupant (`render::colour::field_ramp`) and the presets' custom occupants
//! (`engine::presets`, through `validation::presets`) are written at runtime, out of xtask's reach (systems_architecture
//! §7.1: nothing depends on xtask, and xtask on no workspace crate but ledger). So their output is checked in under
//! `crates/render/frag/runtime/`, where `cargo xtask lint wgsl` holds it to every rule, and this test keeps the files
//! equal to what the writers emit: no file missing, none changed, none stale. `PRINCIPIA_BLESS_RUNTIME_WGSL=1` rewrites
//! them instead.
//!
//! The test registers its negative control (R-176).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use render::assemble::Occupant;
use render::colour::field_ramp::{Compaction, FieldRamp, Lut, Map, Ramp, ScalarField};
use validation::negative_control;
use validation::presets::{self, CoordinateMode};

/// The checked-in directory, relative to the render crate.
const DIR: &str = "frag/runtime";

/// The variable that rewrites the checked-in files from their writers.
const BLESS: &str = "PRINCIPIA_BLESS_RUNTIME_WGSL";

/// The field ramp of `field`: `d_min`'s and `length`'s by their own constructors, every other on `lin` over [0, 1].
fn ramp(field: &str) -> Result<FieldRamp, String> {
    match field {
        "d_min" => FieldRamp::d_min(),
        "length" => FieldRamp::length(),
        _ => Ok(FieldRamp::new(
            ScalarField::payload(field)?,
            Compaction::Lin { lo: 0.0, hi: 1.0 },
            Map::ramp(Ramp::Lut(Lut::Viridis)),
        )),
    }
}

/// Every runtime writer's WGSL, by its path under [`DIR`]: `field_ramp/<field>.wgsl`, the ramp of every field a
/// `ScalarField` reads, and `debug/presets/<preset>_<kind>.wgsl`, each custom occupant of each preset.
fn written() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let l = ledger::payload::ledger();
    let entries = ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
    for e in &entries {
        if ScalarField::payload(e.name).is_ok() {
            let ramp = ramp(e.name).unwrap_or_else(|why| panic!("{why}"));
            out.insert(format!("field_ramp/{}.wgsl", e.name), ramp.wgsl());
        }
    }
    let graphs = [
        ("uv_screen", presets::uv_screen()),
        ("uv_quad", presets::uv_quad()),
        (
            "coordinate_uv",
            presets::coordinate_view(CoordinateMode::Uv),
        ),
        (
            "coordinate_delta",
            presets::coordinate_view(CoordinateMode::Delta),
        ),
    ];
    for (name, graph) in graphs {
        let stain = graph
            .and_then(|g| g.lower())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        for node in stain.nodes() {
            if let Occupant::Custom(text) = &node.occupant {
                let path = format!("debug/presets/{name}_{}.wgsl", node.kind.name());
                out.insert(path, text.clone());
            }
        }
    }
    out
}

/// Each `.wgsl` file under `dir`, recursively, by its `/`-separated path under `root`, with its text.
fn files(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            files(root, &path, out);
        } else if path.extension().is_some_and(|x| x == "wgsl") {
            let rel = path.strip_prefix(root).expect("under the root");
            let rel: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            let text = std::fs::read_to_string(&path).expect("the file");
            out.insert(rel.join("/"), text);
        }
    }
}

/// The checked-in directory.
fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(DIR)
}

/// Writes `want` under the checked-in directory, removing every other file there.
fn bless(want: &BTreeMap<String, String>) {
    let root = dir();
    let _ = std::fs::remove_dir_all(&root);
    for (rel, text) in want {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        std::fs::write(&path, text).expect("the file");
    }
}

/// The checked-in files under `root` are `want`: none missing, none different, none stale.
fn check_checked_in(root: &Path, want: &BTreeMap<String, String>) {
    let mut have = BTreeMap::new();
    files(root, root, &mut have);
    for (rel, text) in want {
        match have.get(rel) {
            None => panic!("{DIR}/{rel} is missing: rerun with {BLESS}=1"),
            Some(t) => assert!(
                t == text,
                "{DIR}/{rel} differs from what its writer emits: rerun with {BLESS}=1"
            ),
        }
    }
    let stale: Vec<&String> = have.keys().filter(|k| !want.contains_key(*k)).collect();
    assert!(
        stale.is_empty(),
        "stale file(s) under {DIR}: {stale:?}: rerun with {BLESS}=1"
    );
}

#[test]
fn runtime_wgsl_is_checked_in() {
    let want = written();
    assert!(
        want.keys().any(|k| k == "field_ramp/ftle.wgsl")
            && want
                .keys()
                .any(|k| k.starts_with("debug/presets/coordinate_uv")),
        "the writers emit the field ramps and the presets: {:?}",
        want.keys().collect::<Vec<_>>()
    );
    if std::env::var_os(BLESS).is_some() {
        bless(&want);
    }
    check_checked_in(&dir(), &want);
}

negative_control!(
    runtime_wgsl_is_checked_in,
    "the ftle field ramp's WGSL without its tier's has_ftle, as before TASK-M1-15",
    expected = "differs from what its writer emits",
    {
        let mut want = written();
        let ftle = want.get_mut("field_ramp/ftle.wgsl").expect("ftle's ramp");
        *ftle = ftle.replace("has_ftle && ", "");
        check_checked_in(&dir(), &want)
    }
);

/// The field ramp of a tier-gated field reads the gate beside its tier's `has_<feature>`: `ftle`'s guard is
/// `has_ftle && ctx.sample.ftle_valid` (render contract Part 2; REQ-RENDER-015).
fn check_ftle_guard(field: &str) {
    let wgsl = ramp(field).unwrap_or_else(|e| panic!("{e}")).wgsl();
    assert!(
        wgsl.contains("has_ftle && ctx.sample.ftle_valid"),
        "`{field}`'s ramp does not guard with has_ftle && ftle_valid:\n{wgsl}"
    );
}

#[test]
fn runtime_wgsl_ftle_ramp_guards_with_has_ftle() {
    check_ftle_guard("ftle");
}

negative_control!(
    runtime_wgsl_ftle_ramp_guards_with_has_ftle,
    "a field with no tier gate, d_min, has no has_ftle",
    expected = "does not guard with has_ftle && ftle_valid",
    check_ftle_guard("d_min")
);
