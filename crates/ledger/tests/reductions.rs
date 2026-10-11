//! The catalogue's reductions (gui_state_contract §4; dd_generation_root §3.8; render contract Part 5; TASK-M1-12):
//! - REQ-VAL-010, REQ-TOOL-157: every vector field of the ledger, `n` and the k = 6 fields alike, has its 'as
//!   direction-cosines' view beside its '‖·‖ as scalar' view, drawn by its `k`'s helper, `dbg_dircos3` for `n` and
//!   `dbg_dircos6` for `r`, `p` and the shadow; a vector the presentation layer has no rendering for refuses generation,
//!   naming it (`reductions_every_vector`).
//! - REQ-TOOL-154: the `ICDescriptor` masses have their one ternary view, `dbg_ternary` of `(m0, m1, m2)`
//!   (`reductions_masses_ternary`).
//! - The checked-in reductions are the emitter's output, none stale (`reductions_checked_in`).
//!
//! Each test has a registered negative control (R-176).

use std::path::Path;

use ledger::gen::catalogue::{self, Reduction};
use ledger::gen::{self, Generated};
use ledger::schema::{FieldType, Ledger};
use validation::negative_control;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// The reductions of `ledger`.
fn reductions_of(ledger: &Ledger) -> Vec<Reduction> {
    let entries = gen::validate(ledger).unwrap_or_else(|e| panic!("{e}"));
    catalogue::reductions(&ledger.words, &entries)
}

/// The vector fields of `ledger`, in its order.
fn vectors(ledger: &Ledger) -> Vec<&'static str> {
    gen::validate(ledger)
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .filter(|e| matches!(e.ty, FieldType::Vector { .. }))
        .map(|e| e.name)
        .collect()
}

// ── REQ-VAL-010, REQ-TOOL-157: both reductions of every vector field ─────────────────────────────────────────────

/// Checks that `reductions` holds, for each of `fields`, its direction-cosines view, `<field>_dircos.wgsl` in the
/// reductions' directory, drawing `dbg_dircos3` of `ctx.sample.n` for `n` and `dbg_dircos6` of the field otherwise, and
/// that each field's '‖·‖ as scalar' view is the catalogue's view of it.
fn check_every_vector(reductions: &[Reduction], fields: &[&str]) {
    let views = catalogue::views(
        &ledger::layout().words,
        &gen::validate(&ledger::layout()).unwrap_or_else(|e| panic!("{e}")),
    );
    for field in fields {
        let name = format!("{field}_dircos");
        let r = reductions
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("`{field}` has no direction-cosines view"));
        assert_eq!(
            r.path.to_string_lossy(),
            format!("{}/{name}.wgsl", catalogue::REDUCTIONS_DIR)
        );
        assert_eq!(r.fields, [*field]);
        let helper = if *field == "n" {
            "dbg_dircos3"
        } else {
            "dbg_dircos6"
        };
        let call = format!("return {helper}(ctx.sample.{field}, ctx.frag_xy);");
        assert!(
            r.wgsl.contains(&call),
            "`{field}`'s direction-cosines view does not `{call}`:\n{}",
            r.wgsl
        );
        assert!(
            views.iter().any(|v| v.field == *field),
            "`{field}` has no '‖·‖ as scalar' view"
        );
    }
}

#[test]
fn reductions_every_vector() {
    let ledger = ledger::layout();
    let fields = vectors(&ledger);
    assert_eq!(fields, ["r", "p", "r_sh", "p_sh", "n"]);
    check_every_vector(&reductions_of(&ledger), &fields);
    let entries = gen::validate(&ledger).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        catalogue::refused(&ledger.words, &entries),
        Vec::<String>::new()
    );
}

negative_control!(
    reductions_every_vector,
    "a catalogue whose reductions lose `r`'s direction cosines",
    expected = "`r` has no direction-cosines view",
    {
        let ledger = ledger::layout();
        let kept: Vec<Reduction> = reductions_of(&ledger)
            .into_iter()
            .filter(|r| r.name != "r_dircos")
            .collect();
        check_every_vector(&kept, &vectors(&ledger));
    }
);

/// The ledger with the mass `m0` typed a vector: the fragment reads it from the `ICDescriptor`, a scalar, so the
/// presentation layer has no direction-cosines rendering of it.
fn with_vector_mass() -> Ledger {
    let mut ledger = ledger::layout();
    let at = ledger
        .entries
        .iter()
        .position(|e| e.name == Some("m0"))
        .expect("the ledger has `m0`");
    ledger.entries[at].ty = Some(FieldType::Vector {
        component: Box::new(FieldType::F32),
        k: 3,
    });
    ledger
}

/// Checks that generation refuses `ledger`'s vector `m0`, naming it, and makes no direction-cosines view of it.
fn check_vector_refused(ledger: &Ledger) {
    let entries = gen::validate(ledger).unwrap_or_else(|e| panic!("{e}"));
    let refused = catalogue::refused(&ledger.words, &entries);
    assert!(
        refused
            .iter()
            .any(|r| r.starts_with("vector field `m0` has no direction-cosines view")),
        "a vector with no rendering is not refused: {refused:?}"
    );
    assert!(
        !reductions_of(ledger).iter().any(|r| r.name == "m0_dircos"),
        "a vector with no rendering has a direction-cosines view"
    );
}

#[test]
fn reductions_every_vector_refuses_one_with_no_rendering() {
    check_vector_refused(&with_vector_mass());
}

negative_control!(
    reductions_every_vector_refuses_one_with_no_rendering,
    "the ledger as it is, `m0` a scalar",
    expected = "a vector with no rendering is not refused",
    check_vector_refused(&ledger::layout())
);

// ── REQ-TOOL-154: the ternary masses ─────────────────────────────────────────────────────────────────────────────

/// Checks that `reductions` ends with the ternary masses view, drawing `dbg_ternary` of the three masses in order.
fn check_ternary(reductions: &[Reduction]) {
    let r = reductions
        .last()
        .filter(|r| r.name == catalogue::MASSES_TERNARY)
        .unwrap_or_else(|| panic!("the reductions do not end with the ternary masses view"));
    assert_eq!(r.fields, catalogue::MASSES);
    assert_eq!(
        r.path.to_string_lossy(),
        format!("{}/masses_ternary.wgsl", catalogue::REDUCTIONS_DIR)
    );
    let call = "return dbg_ternary(vec3<f32>(ctx.ic.m0, ctx.ic.m1, ctx.ic.m2), ctx.frag_xy);";
    assert!(
        r.wgsl.contains(call),
        "the ternary view does not `{call}`:\n{}",
        r.wgsl
    );
}

#[test]
fn reductions_masses_ternary() {
    check_ternary(&reductions_of(&ledger::layout()));
}

negative_control!(
    reductions_masses_ternary,
    "reductions with the ternary masses view dropped",
    expected = "do not end with the ternary masses view",
    {
        let mut reductions = reductions_of(&ledger::layout());
        reductions.pop();
        check_ternary(&reductions);
    }
);

// ── The checked-in reductions are the emitter's ──────────────────────────────────────────────────────────────────

/// The `.wgsl` files in the reductions' directory, relative to the root, sorted.
fn listing() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(root().join(catalogue::REDUCTIONS_DIR))
        .expect("the reductions' directory")
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| n.ends_with(".wgsl"))
        .map(|n| format!("{}/{n}", catalogue::REDUCTIONS_DIR))
        .collect();
    out.sort();
    out
}

/// Checks that every reduction the emitter writes is `on_disk`'s, and that `listing` holds exactly them.
fn check_checked_in(on_disk: &dyn Fn(&str) -> String, listing: &[String]) {
    let files: Vec<Generated> =
        gen::generate(&ledger::layout(), gen::EMITTERS).unwrap_or_else(|e| panic!("{e}"));
    let mut emitted = Vec::new();
    for f in files {
        let path = f.path.to_string_lossy().into_owned();
        if !path.starts_with(catalogue::REDUCTIONS_DIR) {
            continue;
        }
        assert!(
            on_disk(&path) == f.contents,
            "the checked-in {path} is not the emitter's output: run `cargo xtask codegen`"
        );
        emitted.push(path);
    }
    emitted.sort();
    assert_eq!(
        listing, emitted,
        "the reductions on disk are not the emitted ones"
    );
    assert_eq!(
        emitted.len(),
        6,
        "five vectors' direction cosines and the masses"
    );
}

fn checked_in(path: &str) -> String {
    let path = root().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn reductions_checked_in() {
    check_checked_in(&checked_in, &listing());
}

negative_control!(
    reductions_checked_in,
    "a checked-in reduction edited by hand",
    expected = "is not the emitter's output",
    check_checked_in(
        &|p| checked_in(p).replace("dbg_dircos6", "dbg_dircos3"),
        &listing()
    )
);
