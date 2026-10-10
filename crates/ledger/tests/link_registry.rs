//! The link registry (`ledger::links`; dd_generation_root §3.9; chart_decoder_contract Part 2.5):
//! - `link_registry_complete_*`: the registry generates and enumerates §3.9's six rows and its R-72 rows, each entry
//!   with forward, inverse, log-det, ε clamps and sampling note; deleting any member of any entry, or any entry, fails
//!   generation, naming it; a malformed entry is refused; each block holds two links whose sampling notes differ; the
//!   checked-in generated files are the generator's (REQ-CHART-032, REQ-GEN-014, REQ-GEN-015, REQ-GEN-026).
//! - `link_codomain_compat_*`: a link whose codomain is not the block control's is refused; each default resolves to
//!   the named entry (REQ-GEN-013).
//! - `link_registry_chart_constants`: the registry's chart constants are REQ-DEC-009's values, register entries.

use ledger::constants::{Admissibility, Value, REGISTER};
use ledger::gen::links::{generate, CONSTANTS_PATH, LINKS_PATH};
use ledger::links::{
    builders, check, rows, select, select_from, slots, Codomain, Expr, Link, LinkBuilder, Op,
    Param, Slot, CHART_CONSTANTS,
};
use validation::negative_control;

/// The registry's builders, owned, to edit.
fn owned() -> Vec<LinkBuilder> {
    builders().to_vec()
}

/// `builders` with the entry `name` edited by `edit`.
fn edited(name: &str, edit: impl Fn(&mut LinkBuilder)) -> Vec<LinkBuilder> {
    let mut b = owned();
    edit(b.iter_mut().find(|e| e.name == name).expect("the entry"));
    b
}

// ── Complete ────────────────────────────────────────────────────────────────────────────────────────────────────

/// `builders` generates, and holds every entry of each of §3.9's rows, each with all five members.
fn check_complete(builders: &[LinkBuilder]) {
    let files = generate(builders);
    assert!(
        files.is_ok(),
        "the registry does not generate: {:?}",
        files.err()
    );
    for (row, names) in rows() {
        for name in names {
            let b = builders.iter().find(|b| b.name == *name);
            let whole = b.is_some_and(|b| {
                b.codomain.is_some()
                    && b.forward.is_some_and(|f| !f.is_empty())
                    && b.inverse.is_some_and(|i| !i.is_empty())
                    && b.log_det.is_some()
                    && b.clamps.is_some()
                    && b.sampling_note.is_some_and(|n| !n.is_empty())
            });
            assert!(whole, "§3.9's row \"{row}\" lacks a whole entry `{name}`");
        }
    }
}

#[test]
fn link_registry_complete_six_rows() {
    let labels: Vec<&str> = rows().iter().map(|r| r.0).collect();
    for (i, prefix) in [
        "Simplex Δ²: softmax",
        "Bounded (a,b)",
        "Bounded alt",
        "Positive",
        "Symmetric",
        "Unbounded",
    ]
    .iter()
    .enumerate()
    {
        assert!(labels[i].starts_with(prefix), "row {i} is not §3.9's");
    }
    check_complete(builders());
}

negative_control!(
    link_registry_complete_six_rows,
    "a registry without the symmetric row's c·tanh must not generate",
    expected = "the registry does not generate",
    check_complete(
        &owned()
            .into_iter()
            .filter(|b| b.name != "tanh_q")
            .collect::<Vec<_>>()
    )
);

/// Generating `builders` is refused with a line containing `naming`.
fn check_refused(builders: &[LinkBuilder], naming: &str) {
    match generate(builders) {
        Ok(_) => panic!("generation was not refused, though it should name {naming:?}"),
        Err(lines) => assert!(
            lines.iter().any(|l| l.contains(naming)),
            "generation was refused, but no line names {naming:?}: {lines:?}"
        ),
    }
}

/// An edit of an entry.
type Edit = fn(&mut LinkBuilder);

/// Each member's deletion, by its name in the refusal.
fn deletions() -> Vec<(&'static str, Edit)> {
    vec![
        ("codomain", |b| b.codomain = None),
        ("forward", |b| b.forward = None),
        ("inverse", |b| b.inverse = None),
        ("log-det", |b| b.log_det = None),
        ("ε clamps", |b| b.clamps = None),
        ("sampling note", |b| b.sampling_note = None),
        ("sampling note", |b| b.sampling_note = Some("  ")),
    ]
}

#[test]
fn link_registry_complete_deleting_a_member_fails_generation() {
    for b in builders() {
        for (member, delete) in deletions() {
            let naming = format!("link `{}` has no {member}", b.name);
            check_refused(&edited(b.name, delete), &naming);
        }
    }
}

negative_control!(
    link_registry_complete_deleting_a_member_fails_generation,
    "the whole registry generates, so it must fail the refusal check",
    expected = "generation was not refused",
    check_refused(&owned(), "has no forward")
);

#[test]
fn link_registry_complete_deleting_an_entry_fails_generation() {
    for b in builders() {
        let without: Vec<LinkBuilder> = owned().into_iter().filter(|e| e.name != b.name).collect();
        check_refused(&without, &format!("`{}`", b.name));
    }
}

negative_control!(
    link_registry_complete_deleting_an_entry_fails_generation,
    "a registry with every entry generates, so it must fail the refusal check",
    expected = "generation was not refused",
    check_refused(&owned(), "`exp`")
);

/// A malformed edit of an entry, and the problem its refusal names.
type Malformed = (&'static str, Edit, &'static str);

const X0: Expr = Expr::Input(0);
const X1: Expr = Expr::Input(1);
const X3: Expr = Expr::Input(3);

/// Edits each check of an entry refuses.
fn malformed() -> Vec<Malformed> {
    vec![
        (
            "sigmoid_alpha",
            |b| b.forward = Some(&[X0, X0]),
            "its forward has 2 trees, codomain `alpha` needs 1",
        ),
        (
            "softmax_tanh",
            |b| b.inverse = Some(&[X0]),
            "its inverse has 1 trees, codomain `mass` needs 2",
        ),
        (
            "sigmoid_alpha",
            |b| b.forward = Some(&[X1]),
            "its forward reads input(1) of an argument of 1 components",
        ),
        (
            "softmax_tanh",
            |b| b.inverse = Some(&[X3, X0]),
            "its inverse reads input(3) of an argument of 3 components",
        ),
        (
            "softmax_tanh",
            |b| b.log_det = Some(Expr::Input(2)),
            "its log-det reads input(2) of an argument of 2 components",
        ),
        (
            "exp",
            |b| {
                b.params = &[Param {
                    name: "mu_max",
                    value: 4.0,
                }]
            },
            "declares `mu_max` = 4, not the chart constant",
        ),
        (
            "exp",
            |b| {
                b.params = &[Param {
                    name: "mu_max",
                    value: 5.0,
                }]
            },
            "declares `mu_max`, which none of its trees reads",
        ),
        (
            "identity",
            |b| b.log_det = Some(Expr::Param("q_max")),
            "reads `q_max`, which is not among its ε clamps or parameters",
        ),
        (
            "tanh_q",
            |b| b.name = "sigmoid_q",
            "two link registry entries are named `sigmoid_q`",
        ),
        (
            "sigmoid_q",
            |b| b.codomain = Some(Codomain::Alpha),
            "slot `q0`: its default `sigmoid_q` is not an entry onto `momentum`",
        ),
    ]
}

#[test]
fn link_registry_complete_refuses_malformed_entries() {
    for (name, edit, naming) in malformed() {
        check_refused(&edited(name, edit), naming);
    }
    // Momentum's three links with one sampling note.
    let mut b = owned();
    for e in b
        .iter_mut()
        .filter(|e| e.codomain == Some(Codomain::Momentum))
    {
        e.sampling_note = Some("as bounded");
    }
    check_refused(
        &b,
        "codomain `momentum` has fewer than two links whose sampling notes differ",
    );
}

negative_control!(
    link_registry_complete_refuses_malformed_entries,
    "a well-formed entry generates, so it must fail the refusal check",
    expected = "generation was not refused",
    check_refused(&edited("exp", |_| {}), "declares")
);

/// `builders` passes the registry's checks.
fn check_accepted(builders: &[LinkBuilder]) {
    assert!(
        check(builders).is_ok(),
        "an entry reading only its last input was refused: {:?}",
        check(builders).err()
    );
}

/// The simplex default with its log-det also reading `input(1)`, the last of its two controls.
fn reading_last(i: u32) -> Vec<LinkBuilder> {
    edited("softmax_tanh", |b| {
        let ld = b.log_det.expect("a log-det");
        b.log_det = Some(Expr::Op(Op::Add, Vec::leak(vec![ld, Expr::Input(i)])));
    })
}

#[test]
fn link_registry_complete_accepts_the_last_input() {
    check_accepted(&reading_last(1));
}

negative_control!(
    link_registry_complete_accepts_the_last_input,
    "a log-det reading input(2) of two controls must fail the acceptance check",
    expected = "an entry reading only its last input was refused",
    check_accepted(&reading_last(2))
);

/// Each slot's codomain holds at least two links whose sampling notes differ (REQ-GEN-014), in `registered`.
fn check_two_notes(registered: &[(Link, Codomain)]) {
    for slot in slots() {
        let mut notes: Vec<&str> = registered
            .iter()
            .filter(|(_, c)| *c == slot.codomain)
            .map(|(l, _)| l.sampling_note)
            .collect();
        notes.sort_unstable();
        notes.dedup();
        assert!(
            notes.len() >= 2,
            "block control `{}` has {} distinct sampling notes, not two (REQ-GEN-014)",
            slot.name,
            notes.len()
        );
    }
}

#[test]
fn link_registry_complete_two_notes_per_block() {
    check_two_notes(&check(builders()).expect("the registry checks"));
}

negative_control!(
    link_registry_complete_two_notes_per_block,
    "the defaults alone are one link per block",
    expected = "distinct sampling notes, not two",
    {
        let registered = check(builders()).expect("the registry checks");
        let defaults: Vec<(Link, Codomain)> = registered
            .into_iter()
            .filter(|(l, _)| slots().iter().any(|s| s.default == l.name))
            .collect();
        check_two_notes(&defaults)
    }
);

/// The generated files are the checked-in ones (`cargo xtask codegen` was run).
fn check_current(files: &[(String, String)]) {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    for (path, contents) in files {
        let on_disk = std::fs::read_to_string(format!("{root}{path}"))
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        assert!(
            on_disk == *contents,
            "{path} is not the generator's output: run `cargo xtask codegen`"
        );
    }
}

/// The registry's generated files, by path.
fn generated() -> Vec<(String, String)> {
    generate(builders())
        .expect("the registry generates")
        .into_iter()
        .map(|g| (g.path.display().to_string(), g.contents))
        .collect()
}

#[test]
fn link_registry_complete_generated_files_are_current() {
    let files = generated();
    let paths: Vec<&str> = files.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(paths, [CONSTANTS_PATH, LINKS_PATH]);
    check_current(&files);
}

negative_control!(
    link_registry_complete_generated_files_are_current,
    "a registry with another sampling note generates another links.rs",
    expected = "is not the generator's output",
    {
        let b = edited("exp", |b| b.sampling_note = Some("heavy-tailed"));
        let files: Vec<(String, String)> = generate(&b)
            .expect("the edited registry generates")
            .into_iter()
            .map(|g| (g.path.display().to_string(), g.contents))
            .collect();
        check_current(&files)
    }
);

// ── Codomain compatibility ──────────────────────────────────────────────────────────────────────────────────────

/// `select` refuses every link whose codomain is not the slot's, and accepts every one whose codomain is, naming the
/// constraints in its refusal; a name the registry lacks is refused.
fn check_compat(select: impl Fn(&Slot, &str) -> Result<&'static Link, String>) {
    let registered = check(builders()).expect("the registry checks");
    for slot in slots() {
        for (l, c) in &registered {
            match (select(&slot, l.name), *c == slot.codomain) {
                (Ok(got), true) => assert_eq!(got.name, l.name, "the wrong link was selected"),
                (Err(e), false) => assert!(
                    e.contains(slot.codomain.constraint().spelling())
                        && e.contains(l.constraint.spelling()),
                    "the refusal does not name the constraints: {e}"
                ),
                (Ok(_), false) => panic!(
                    "`{}` onto `{}` was not rejected for slot `{}` onto `{}`",
                    l.name,
                    c.name(),
                    slot.name,
                    slot.codomain.name()
                ),
                (Err(e), true) => panic!("`{}` was refused for slot `{}`: {e}", l.name, slot.name),
            }
        }
        assert!(
            select(&slot, "no_such_link").is_err(),
            "a link the registry lacks was not rejected"
        );
    }
}

#[test]
fn link_codomain_compat_rejects_a_mismatched_link() {
    check_compat(select);
    let alpha = slots()[1];
    let e = select(&alpha, "softplus").expect_err("a positive link onto α");
    assert!(
        e.contains("slot `alpha` takes a bounded link onto `alpha`; `softplus` is a positive link"),
        "{e}"
    );
    assert!(
        select(&alpha, "sigmoid_beta").is_err(),
        "β's σ, bounded but onto (0, π), was selected for α"
    );
}

negative_control!(
    link_codomain_compat_rejects_a_mismatched_link,
    "a selection by name alone must fail the compatibility check",
    expected = "was not rejected",
    check_compat(|_, name| ledger::links::registry()
        .iter()
        .find(|l| l.name == name)
        .ok_or_else(|| "absent".to_owned()))
);

/// Each slot's default is dd_decoder §3's, and selecting it by name gives that entry.
fn check_defaults(slots: &[Slot]) {
    for slot in slots {
        let want = match slot.block {
            "mass" => "softmax_tanh",
            "config" => ["sigmoid_alpha", "sigmoid_beta"][slot.z as usize],
            _ => "sigmoid_q",
        };
        assert_eq!(
            slot.default, want,
            "slot `{}`'s default is not dd_decoder §3's",
            slot.name
        );
        let got = select(slot, slot.default).expect("the default is selectable");
        assert_eq!(got.name, want, "the default does not resolve by name");
    }
}

#[test]
fn link_codomain_compat_defaults_resolve_by_name() {
    let s = slots();
    let blocks: Vec<(&str, &str, u32)> = s.iter().map(|s| (s.name, s.block, s.z)).collect();
    assert_eq!(
        blocks,
        [
            ("mass", "mass", 6),
            ("alpha", "config", 0),
            ("beta", "config", 1),
            ("q0", "momentum", 2),
            ("q1", "momentum", 3),
            ("q2", "momentum", 4),
            ("q3", "momentum", 5),
        ],
        "the slots are not chart_decoder Part 2's controls in decode order"
    );
    check_defaults(&s);
    let registered = check(builders()).expect("the registry checks");
    assert!(
        select_from(&registered[1..], &s[0], "softmax_tanh").is_err(),
        "a default absent from the registry resolved"
    );
}

negative_control!(
    link_codomain_compat_defaults_resolve_by_name,
    "a config default of tanh is not dd_decoder §3's sigmoid",
    expected = "default is not dd_decoder §3's",
    {
        let mut s = slots();
        s[1].default = "tanh_alpha";
        check_defaults(&s)
    }
);

// ── The chart constants ─────────────────────────────────────────────────────────────────────────────────────────

/// `chart` is REQ-DEC-009's constants, each a register entry of its value, in canonical units.
fn check_chart_constants(chart: &[Param]) {
    let want = [
        ("mu_max", 5.0),
        ("q_max", 2.0),
        ("alpha_min", 0.0),
        ("eps_mu", 1e-6),
        ("eps_z", 1e-6),
        ("eps_q", 1e-6),
        ("delta_lambda", 1e-12),
        ("eps_w", 1e-10),
    ];
    let got: Vec<(&str, f64)> = chart.iter().map(|p| (p.name, p.value)).collect();
    assert_eq!(got, want, "the chart constants are not REQ-DEC-009's");
    for p in chart {
        let k = REGISTER
            .iter()
            .find(|k| k.name == p.name)
            .unwrap_or_else(|| panic!("`{}` is not a register entry", p.name));
        assert_eq!(k.value, Some(Value::Exact(p.value)), "`{}`", p.name);
        assert_eq!(k.class, Some(Admissibility::CanonicalUnits), "`{}`", p.name);
    }
}

#[test]
fn link_registry_chart_constants() {
    check_chart_constants(CHART_CONSTANTS);
}

negative_control!(
    link_registry_chart_constants,
    "μ_max = 4 must fail the check",
    expected = "the chart constants are not REQ-DEC-009's",
    {
        let mut c = CHART_CONSTANTS.to_vec();
        c[0].value = 4.0;
        check_chart_constants(&c)
    }
);
