//! QA tests for TASK-M1-08 (the generated debug catalogue and the host export decoder), written from the requirements
//! and their sources, not from the implementation:
//! - REQ-TOOL-020 (RQ-218): the catalogue's field list is the ledger's, `ledger::layout().entries`: exactly one view
//!   and exactly one test per entry, none for anything else (PIT-3: a field left out is caught).
//! - Render contract Part 6's rows: the twelve `ICDescriptor` views read `ctx.ic` (RQ-227); the shadow `r_sh`/`p_sh`
//!   is viewed from the read side's `ctx.sample` (RQ-228); every other field from `ctx.sample`, a word field from the
//!   word. Each view is a colour occupant (gui_state_contract §3: `colour(ctx) -> vec3`).
//! - REQ-TOOL-017: each view and its test reference the same generated accessor symbols.
//! - REQ-GEN-011: a field added with its metadata appears in the catalogue, its tests and the export decoder; a field
//!   without metadata, or one no artefact can read, fails generation naming the field (`GenError::Unread`), and
//!   nothing is written.
//! - REQ-GEN-010: one ledger entry changed changes all four artefacts (Rust pack/unpack, the host export decoder, the
//!   WGSL unpack, the catalogue); the checked-in generated files are the emitters' output, so a hand edit is found.
//! - RQ-228 / RQ-227 on the GPU, against hand-filled buffers (debug_tooling_plan "Principle"): the shadow reads the
//!   stored shadow at the FTLE tier and the canonical quiet NaN `0x7FC00000` (lowering Part 3a) at the base tier;
//!   `ic_read` fills exactly the members the stain reads, each the stored value, and leaves the rest unloaded (R-378).
//!   A read-side field list naming no `ICDescriptor` member is refused.
//!
//! Each test has a registered negative control (R-176).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ledger::gen::{self, catalogue, export, read, rust, wgsl, GenError, Generated};
use ledger::schema::{
    Consumer, EntryBuilder, FieldType, Ledger, Location, Provenance, Range, Scale, Span,
};
use naga::{Module, TypeInner};
use read::Tier;
use validation::gpu::GpuHarness;
use validation::negative_control;

/// Render contract Part 6's `ICDescriptor` row: "all 12", read from `ctx.ic`.
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

/// The Benettin shadow (payload §1), read from the stored shadow at the FTLE tier (RQ-228).
const SHADOW: [&str; 2] = ["r_sh", "p_sh"];

/// Lowering Part 3a's canonical quiet NaN (R-72; REQ-RENDER-077).
const QNAN: u32 = 0x7FC0_0000;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn checked_in(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn generate(l: &Ledger) -> Vec<Generated> {
    gen::generate(l, gen::EMITTERS).unwrap_or_else(|e| panic!("{e}"))
}

/// The ledger's field names, `ledger::layout().entries`, in order (RQ-218).
fn names(l: &Ledger) -> Vec<&'static str> {
    l.entries
        .iter()
        .map(|e| e.name.expect("every payload entry is named"))
        .collect()
}

/// The text of the generated file at `path` among `files`, if any.
fn file<'a>(files: &'a [Generated], path: &str) -> Option<&'a str> {
    files
        .iter()
        .find(|f| f.path == Path::new(path))
        .map(|f| f.contents.as_str())
}

/// The views among `files`: field name → WGSL, from the files directly under the catalogue's directory.
fn views(files: &[Generated]) -> Vec<(String, String)> {
    files
        .iter()
        .filter(|f| f.path.parent() == Some(Path::new(catalogue::DIR)))
        .map(|f| {
            let name = f
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".wgsl"))
                .unwrap_or_else(|| panic!("{} is not a .wgsl view", f.path.display()))
                .to_owned();
            (name, f.contents.clone())
        })
        .collect()
}

/// The names of the `#[test]` functions in `text`.
fn tests_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut next_is_test = false;
    for line in text.lines() {
        let t = line.trim();
        if t == "#[test]" {
            next_is_test = true;
            continue;
        }
        if next_is_test {
            if let Some(rest) = t.strip_prefix("fn ") {
                out.push(rest.split('(').next().unwrap_or_default().to_owned());
            }
            next_is_test = false;
        }
    }
    out
}

// ── REQ-TOOL-020: one view and one test per ledger field, and nothing else ───────────────────────────────────────

/// `files` hold exactly one view per field of `fields` and nothing else, and the views' tests file exactly one test
/// per field, `catalogue_view_<field>` lowercased, and nothing else.
fn check_one_view_one_test(files: &[Generated], fields: &[&str]) {
    let views = views(files);
    let mut seen = BTreeSet::new();
    for (v, _) in &views {
        assert!(seen.insert(v.clone()), "two views of `{v}`");
        assert!(
            fields.contains(&v.as_str()),
            "a view of `{v}`, which is no ledger field"
        );
    }
    for f in fields {
        assert!(seen.contains(*f), "the catalogue has no view of `{f}`");
    }
    assert_eq!(
        views.len(),
        fields.len(),
        "views and ledger fields differ in number"
    );
    let tests = tests_in(file(files, catalogue::TESTS_PATH).expect("the views' tests"));
    let want: BTreeSet<String> = fields
        .iter()
        .map(|f| format!("catalogue_view_{}", f.to_lowercase()))
        .collect();
    let got: BTreeSet<String> = tests.iter().cloned().collect();
    assert_eq!(tests.len(), got.len(), "a field has two tests");
    assert_eq!(got, want, "the views' tests are not one per ledger field");
}

#[test]
fn qa_tool020_one_view_and_one_test_per_ledger_field() {
    let l = ledger::layout();
    let fields = names(&l);
    // RQ-218: every entry, RQ-227's twelve ICDescriptor fields and RQ-228's shadow among them.
    for f in IC.iter().chain(&SHADOW) {
        assert!(fields.contains(f), "`{f}` is not a ledger field");
    }
    check_one_view_one_test(&generate(&l), &fields);
}

negative_control!(
    qa_tool020_one_view_and_one_test_per_ledger_field,
    "a catalogue with `rho_angle`'s view deliberately left out is not the ledger's (PIT-3)",
    expected = "the catalogue has no view of `rho_angle`",
    {
        let l = ledger::layout();
        let files: Vec<Generated> = generate(&l)
            .into_iter()
            .filter(|f| !f.path.ends_with("rho_angle.wgsl"))
            .collect();
        check_one_view_one_test(&files, &names(&l))
    }
);

/// RQ-218's reading: a catalogue that also views a name the ledger lacks is not the ledger's either.
#[cfg(feature = "controls")]
mod qa_tool020_one_view_and_one_test_per_ledger_field_extra {
    use super::*;

    negative_control!(
        qa_tool020_one_view_and_one_test_per_ledger_field,
        "a catalogue checked against the ledger less `p_sh` has a view of a non-field",
        expected = "a view of `p_sh`, which is no ledger field",
        {
            let l = ledger::layout();
            let fields: Vec<&str> = names(&l).into_iter().filter(|&f| f != "p_sh").collect();
            check_one_view_one_test(&generate(&l), &fields)
        }
    );
}

// ── Render contract Part 6: what each view reads ─────────────────────────────────────────────────────────────────

/// The `ctx.<lane>.<member>` reads in `wgsl`, outside comments, as `(lane, member)`.
fn ctx_reads(wgsl: &str) -> BTreeSet<(String, String)> {
    let code: String = wgsl
        .lines()
        .map(|l| l.split("//").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = BTreeSet::new();
    for (k, _) in code.match_indices("ctx.") {
        let rest = &code[k + 4..];
        let lane: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let after = &rest[lane.len()..];
        let member = after
            .strip_prefix('.')
            .map(|m| {
                m.chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect::<String>()
            })
            .unwrap_or_default();
        out.insert((lane, member));
    }
    out
}

/// Each view of `views` is a colour occupant reading its own field, from the lane Part 6 names: `ctx.ic` for the
/// twelve `ICDescriptor` fields, `ctx.sample.word` for a field of the word's `.w`, `ctx.sample.<field>` for every
/// other, the shadow included; and nothing else of `ctx.sample` or `ctx.ic`.
fn check_view_lanes(views: &[(String, String)], word_fields: &[&str]) {
    for (field, text) in views {
        assert!(
            text.contains("fn colour(ctx: Ctx) -> vec3<f32>"),
            "`{field}`'s view is not a colour occupant"
        );
        let reads: BTreeSet<(String, String)> = ctx_reads(text)
            .into_iter()
            .filter(|(lane, _)| lane == "sample" || lane == "ic")
            .collect();
        let want = if IC.contains(&field.as_str()) {
            ("ic", field.as_str())
        } else if word_fields.contains(&field.as_str()) {
            ("sample", "word")
        } else {
            ("sample", field.as_str())
        };
        let want: BTreeSet<(String, String)> = [(want.0.to_owned(), want.1.to_owned())].into();
        assert_eq!(
            reads, want,
            "`{field}`'s view does not read its own field from its lane"
        );
    }
}

/// The ledger's fields of the word buffer's `.w` (payload §3).
fn word_fields(l: &Ledger) -> Vec<&'static str> {
    l.entries
        .iter()
        .filter(|e| matches!(e.location, Some(Location::Packed { word: "fgw_w", .. })))
        .map(|e| e.name.expect("named"))
        .collect()
}

#[test]
fn qa_tool020_each_view_reads_its_field_from_its_lane() {
    let l = ledger::layout();
    let views = views(&generate(&l));
    let ic: Vec<&str> = views
        .iter()
        .filter(|(_, t)| ctx_reads(t).iter().any(|(lane, _)| lane == "ic"))
        .map(|(f, _)| f.as_str())
        .collect();
    assert_eq!(
        ic, IC,
        "the views on `ctx.ic` are not Part 6's twelve, in the ledger's order"
    );
    check_view_lanes(&views, &word_fields(&l));
}

negative_control!(
    qa_tool020_each_view_reads_its_field_from_its_lane,
    "an `rho_angle` view reading `ctx.sample.theta` reads another field from another lane",
    expected = "`rho_angle`'s view does not read its own field from its lane",
    {
        let l = ledger::layout();
        let views: Vec<(String, String)> = views(&generate(&l))
            .into_iter()
            .map(|(f, t)| {
                let t = if f == "rho_angle" {
                    t.replace("ctx.ic.rho_angle", "ctx.sample.theta")
                } else {
                    t
                };
                (f, t)
            })
            .collect();
        check_view_lanes(&views, &word_fields(&l))
    }
);

/// RQ-228: a shadow view that reads the state `r` rather than the stored shadow is caught.
#[cfg(feature = "controls")]
mod qa_tool020_each_view_reads_its_field_from_its_lane_shadow {
    use super::*;

    negative_control!(
        qa_tool020_each_view_reads_its_field_from_its_lane,
        "an `r_sh` view reading `ctx.sample.r` does not read the shadow",
        expected = "`r_sh`'s view does not read its own field from its lane",
        {
            let l = ledger::layout();
            let views: Vec<(String, String)> = views(&generate(&l))
                .into_iter()
                .map(|(f, t)| {
                    let t = if f == "r_sh" {
                        t.replace("ctx.sample.r_sh", "ctx.sample.r")
                    } else {
                        t
                    };
                    (f, t)
                })
                .collect();
            check_view_lanes(&views, &word_fields(&l))
        }
    );
}

// ── REQ-TOOL-017: the view and its test reference the same generated accessors ───────────────────────────────────

/// The generated payload accessors (payload §6's prefixes, `sd_`, `tm_`, `pb_`, `fgw_`) called in `text`.
fn accessor_calls(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let bytes: Vec<char> = text.chars().collect();
    let mut k = 0;
    while k < bytes.len() {
        let starts = k == 0 || !(bytes[k - 1].is_ascii_alphanumeric() || bytes[k - 1] == '_');
        if starts && (bytes[k].is_ascii_alphabetic() || bytes[k] == '_') {
            let ident: String = bytes[k..]
                .iter()
                .take_while(|c| c.is_ascii_alphanumeric() || **c == '_')
                .collect();
            let next = bytes.get(k + ident.chars().count()).copied();
            if next == Some('(')
                && ["sd_", "tm_", "pb_", "fgw_"]
                    .iter()
                    .any(|p| ident.starts_with(p))
            {
                out.insert(ident.clone());
            }
            k += ident.chars().count().max(1);
        } else {
            k += 1;
        }
    }
    out
}

/// The body of `fn check_<field>` in the tests file, the check the view's test runs.
fn check_body<'a>(tests: &'a str, field: &str) -> &'a str {
    let head = format!("fn check_{}(", field.to_lowercase());
    let start = tests
        .find(&head)
        .unwrap_or_else(|| panic!("no `{head}` in the views' tests"));
    let body = &tests[start..];
    &body[..body.find("\n}\n").unwrap_or(body.len())]
}

/// Each view's accessor symbols, the member it reads (`SimState.<m>` from `ctx.sample.<m>`, `ICDescriptor.<m>` from
/// `ctx.ic.<m>`) and the generated accessors it calls, are referenced by its test: the Rust read side's `read.<m>`,
/// the stored descriptor's `ic.<m>`, and the same accessor functions.
fn check_shared_accessors(views: &[(String, String)], tests: &str) {
    for (field, text) in views {
        let body = check_body(tests, field);
        for (lane, member) in ctx_reads(text) {
            let rust = match lane.as_str() {
                "sample" => format!("read.{member}"),
                "ic" => format!("ic.{member}"),
                _ => continue,
            };
            let referenced = body.match_indices(&rust).any(|(k, _)| {
                !body[k + rust.len()..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
            });
            assert!(
                referenced,
                "`{field}`'s view reads `ctx.{lane}.{member}` and its test does not read `{rust}`"
            );
        }
        let calls = accessor_calls(text);
        let test_calls = accessor_calls(body);
        for c in &calls {
            assert!(
                test_calls.contains(c),
                "`{field}`'s view calls `{c}` and its test does not"
            );
        }
    }
}

#[test]
fn qa_tool017_view_and_test_reference_the_same_accessors() {
    let files = generate(&ledger::layout());
    let tests = file(&files, catalogue::TESTS_PATH).expect("the views' tests");
    let views = views(&files);
    // Word fields go through a generated accessor; the check is not vacuous.
    assert!(
        views.iter().any(|(_, t)| !accessor_calls(t).is_empty()),
        "no view calls a generated accessor"
    );
    check_shared_accessors(&views, tests);
}

negative_control!(
    qa_tool017_view_and_test_reference_the_same_accessors,
    "a `length` test that reads the word through another accessor does not share the view's",
    expected = "`length`'s view calls `fgw_length_raw` and its test does not",
    {
        let files = generate(&ledger::layout());
        let tests = file(&files, catalogue::TESTS_PATH).expect("the views' tests");
        let body = check_body(tests, "length");
        assert!(
            body.contains("fgw_length_raw"),
            "the control's pattern is gone"
        );
        let broken = tests.replace(body, &body.replace("fgw_length_raw", "fgw_payload"));
        check_shared_accessors(&views(&files), &broken)
    }
);

/// RQ-227: an IC view's test that reads the read side rather than the descriptor is caught.
#[cfg(feature = "controls")]
mod qa_tool017_view_and_test_reference_the_same_accessors_ic {
    use super::*;

    negative_control!(
        qa_tool017_view_and_test_reference_the_same_accessors,
        "a `K_0` test that reads `E_0` does not read the view's `ic.K_0`",
        expected = "`K_0`'s view reads `ctx.ic.K_0` and its test does not read `ic.K_0`",
        {
            let files = generate(&ledger::layout());
            let tests = file(&files, catalogue::TESTS_PATH).expect("the views' tests");
            let body = check_body(tests, "K_0");
            let broken = tests.replace(body, &body.replace("ic.K_0", "read.E_0"));
            check_shared_accessors(&views(&files), &broken)
        }
    );
}

// ── REQ-GEN-011: a new field is viewed, or generation fails naming it ────────────────────────────────────────────

/// The payload ledger plus `qa_cat`, a 4-category field in `packed_a`'s bits 10–11, the reserved span narrowed to
/// bits 12–15 (payload §2's reserved bits); its builder `entry`, or none when `add` is false.
fn with_qa_cat(add: bool, entry: impl FnOnce(EntryBuilder) -> EntryBuilder) -> Ledger {
    let mut l = ledger::layout();
    if !add {
        return l;
    }
    let a = l
        .words
        .iter_mut()
        .find(|w| w.name == "packed_a")
        .expect("packed_a");
    a.reserved = vec![Span {
        offset: 12,
        width: 4,
    }];
    let b = EntryBuilder::new("qa_cat")
        .location(Location::Packed {
            word: "packed_a",
            offset: 10,
            width: 2,
        })
        .ty(FieldType::UBits)
        .scale(Scale::Categorical(4))
        .range(Range::int(0, 3))
        .provenance(Provenance::Kernel)
        .consumers(&[Consumer::Render, Consumer::Export, Consumer::Debug]);
    l.entries.push(entry(b));
    l
}

/// Generating from `l` views `qa_cat` from `ctx.sample`, tests it, decodes it in the export decoder, and the catalogue
/// stays exactly the ledger's.
fn check_qa_cat_viewed(l: &Ledger) {
    let files = generate(l);
    let path = format!("{}/qa_cat.wgsl", catalogue::DIR);
    let view = file(&files, &path).unwrap_or_else(|| panic!("no view of the new field `qa_cat`"));
    assert!(
        ctx_reads(view).contains(&("sample".to_owned(), "qa_cat".to_owned())),
        "the new field's view does not read `ctx.sample.qa_cat`"
    );
    let decoder = file(&files, export::PATH).expect("the export decoder");
    assert!(
        decoder.contains("pub qa_cat:"),
        "the export decoder does not decode the new field"
    );
    check_one_view_one_test(&files, &names(l));
}

#[test]
fn qa_gen011_a_field_with_metadata_appears() {
    check_qa_cat_viewed(&with_qa_cat(true, |e| e));
}

negative_control!(
    qa_gen011_a_field_with_metadata_appears,
    "the ledger without the field has no view of it",
    expected = "no view of the new field `qa_cat`",
    check_qa_cat_viewed(&with_qa_cat(false, |e| e))
);

/// Generation from `l` is refused naming `field` and `why`, and a run writes nothing under a fresh root.
fn check_refused(l: &Ledger, field: &str, why: &str) {
    let msg = match gen::generate(l, gen::EMITTERS) {
        Ok(_) => panic!("generation was not refused"),
        Err(e) => e.to_string(),
    };
    assert!(
        msg.contains(&format!("`{field}`")) && msg.contains(why),
        "refused without naming `{field}` and {why:?}: {msg}"
    );
    let scratch = scratch("refused");
    let ran = gen::run(l, gen::EMITTERS, &scratch);
    let wrote = scratch.exists();
    let _ = std::fs::remove_dir_all(&scratch);
    assert!(ran.is_err(), "the run was not refused");
    assert!(!wrote, "a refused run wrote files");
}

/// A fresh scratch directory under the test target's temporary directory.
fn scratch(case: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let p = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_m1_08_{case}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&p);
    p
}

#[test]
fn qa_gen011_a_field_without_metadata_fails_naming_it() {
    for key in ["scale", "range", "type", "provenance", "consumers"] {
        check_refused(
            &with_qa_cat(true, |e| e.without(key)),
            "qa_cat",
            &format!("`{key}`"),
        );
    }
}

negative_control!(
    qa_gen011_a_field_without_metadata_fails_naming_it,
    "the field with its metadata complete generates",
    expected = "generation was not refused",
    check_refused(&with_qa_cat(true, |e| e), "qa_cat", "`range`")
);

/// The payload ledger plus `qa_unread`, a field with complete metadata derived from `S`, which neither the read side,
/// the word nor `ICDescriptor` holds, so no view or decoding can read it; or the ledger unchanged.
fn with_unread(add: bool) -> Ledger {
    let mut l = ledger::layout();
    if add {
        l.entries.push(
            EntryBuilder::new("qa_unread")
                .location(Location::Derived { from: vec!["S"] })
                .ty(FieldType::F32)
                .scale(Scale::Lin)
                .range(Range::int(0, 1))
                .provenance(Provenance::Kernel)
                .consumers(&[Consumer::Render, Consumer::Export, Consumer::Debug]),
        );
    }
    l
}

/// Generation from `l` fails with `GenError::Unread`, a line naming `qa_unread`; nothing is written.
fn check_unread(l: &Ledger) {
    match gen::generate(l, gen::EMITTERS) {
        Ok(_) => panic!("generation was not refused"),
        Err(GenError::Unread(lines)) => assert!(
            !lines.is_empty() && lines.iter().all(|l| l.contains("`qa_unread`")),
            "the refusal does not name `qa_unread` on each line: {lines:?}"
        ),
        Err(e) => panic!("refused, but not as unread: {e}"),
    }
    check_refused(l, "qa_unread", "qa_unread");
}

#[test]
fn qa_gen011_a_field_no_artefact_reads_fails_naming_it() {
    check_unread(&with_unread(true));
}

negative_control!(
    qa_gen011_a_field_no_artefact_reads_fails_naming_it,
    "the ledger without the unread field generates",
    expected = "generation was not refused",
    check_unread(&with_unread(false))
);

// ── REQ-GEN-010: one source, four artefacts; the checked-in files are the emitters' ─────────────────────────────

/// Which of the four artefacts `path` belongs to.
fn artefact(path: &Path) -> Option<&'static str> {
    let p = path.to_string_lossy();
    if p == rust::PATH || p == read::RUST_PATH {
        Some("Rust pack/unpack")
    } else if p == export::PATH {
        Some("the host export decoder")
    } else if p == wgsl::PATH || p == read::WGSL_PATH {
        Some("the WGSL unpack")
    } else if p.starts_with(catalogue::DIR) || p == catalogue::TESTS_PATH {
        Some("the catalogue")
    } else {
        None
    }
}

/// The payload ledger with `dmin_pair` renamed `qa_pair`, or unchanged.
fn renamed(rename: bool) -> Ledger {
    let mut l = ledger::layout();
    if rename {
        let e = l
            .entries
            .iter_mut()
            .find(|e| e.name == Some("dmin_pair"))
            .expect("dmin_pair");
        e.name = Some("qa_pair");
    }
    l
}

/// Renaming one entry changes each of the four artefacts, each to name the new field: its view replaces the old.
fn check_four_artefacts(l: &Ledger) {
    let before = generate(&ledger::layout());
    let after = generate(l);
    for name in [
        "Rust pack/unpack",
        "the host export decoder",
        "the WGSL unpack",
        "the catalogue",
    ] {
        let text = |files: &[Generated]| -> String {
            files
                .iter()
                .filter(|f| artefact(&f.path) == Some(name))
                .map(|f| format!("{}\n{}", f.path.display(), f.contents))
                .collect()
        };
        let (b, a) = (text(&before), text(&after));
        assert!(!b.is_empty(), "the emitters write no {name}");
        assert!(a != b, "{name} did not change with the ledger entry");
        assert!(
            a.contains("qa_pair"),
            "{name} does not name the renamed field"
        );
    }
    let old = format!("{}/dmin_pair.wgsl", catalogue::DIR);
    assert!(
        file(&after, &old).is_none(),
        "the old field's view is still generated"
    );
}

#[test]
fn qa_gen010_one_entry_changes_all_four_artefacts() {
    check_four_artefacts(&renamed(true));
}

negative_control!(
    qa_gen010_one_entry_changes_all_four_artefacts,
    "an unchanged ledger changes no artefact",
    expected = "did not change with the ledger entry",
    check_four_artefacts(&renamed(false))
);

/// Every file the emitters write from the payload ledger is the checked-in file (`on_disk`), all four artefacts; and
/// the views' directory holds no file the emitters do not write.
fn check_checked_in(on_disk: &dyn Fn(&str) -> String, listing: &[String]) {
    let files = generate(&ledger::layout());
    let mut kinds = BTreeSet::new();
    for f in &files {
        let path = f.path.to_string_lossy();
        if let Some(k) = artefact(&f.path) {
            kinds.insert(k);
        }
        assert!(
            on_disk(&path) == f.contents,
            "the checked-in {path} is not the emitters' output"
        );
    }
    assert_eq!(
        kinds.len(),
        4,
        "the emitters write {} of the four artefacts",
        kinds.len()
    );
    let emitted: BTreeSet<String> = views(&files).into_iter().map(|(f, _)| f).collect();
    for v in listing {
        assert!(
            emitted.contains(v),
            "the checked-in view `{v}` is not emitted"
        );
    }
}

/// The `.wgsl` files checked in under the views' directory, by field.
fn listing() -> Vec<String> {
    std::fs::read_dir(root().join(catalogue::DIR))
        .expect("the views' directory")
        .filter_map(|e| {
            let n = e
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned();
            n.strip_suffix(".wgsl").map(str::to_owned)
        })
        .collect()
}

#[test]
fn qa_gen010_checked_in_files_are_the_emitters_output() {
    check_checked_in(&|p| checked_in(p), &listing());
}

negative_control!(
    qa_gen010_checked_in_files_are_the_emitters_output,
    "a hand edit to the `r_sh` view is found",
    expected =
        "the checked-in crates/render/frag/debug/generated/r_sh.wgsl is not the emitters' output",
    check_checked_in(
        &|p| {
            let t = checked_in(p);
            if p.ends_with("/r_sh.wgsl") {
                t.replace("ctx.sample.r_sh", "ctx.sample.p_sh")
            } else {
                t
            }
        },
        &listing()
    )
);

/// A stale hand-added view beside the generated ones is found.
#[cfg(feature = "controls")]
mod qa_gen010_checked_in_files_are_the_emitters_output_stale {
    use super::*;

    negative_control!(
        qa_gen010_checked_in_files_are_the_emitters_output,
        "a view left for a field the ledger no longer has is found",
        expected = "the checked-in view `qa_gone` is not emitted",
        {
            let mut l = listing();
            l.push("qa_gone".to_owned());
            check_checked_in(&|p| checked_in(p), &l)
        }
    );
}

// ── RQ-228 / RQ-227 on the GPU: the shadow and `ctx.ic`, from hand-filled buffers ─────────────────────────────────

const BARE: Tier = Tier {
    has_ftle: false,
    has_word: false,
};
const FTLE_NO_WORD: Tier = Tier {
    has_ftle: true,
    has_word: false,
};

/// Samples per run.
const SAMPLES: u32 = 3;
/// Output words per sample: the shadow's 12 components, then the twelve `ICDescriptor` members.
const OUT: u32 = 24;

/// A distinct finite f32 for word `k` of a buffer `tag`, never NaN, never 0.
fn fill(tag: u32, k: u32) -> f32 {
    (tag * 1000 + k + 1) as f32 * 0.25
}

/// The named struct `name` in `m`: its size in words and its members' word offsets by name.
fn layout(m: &Module, name: &str) -> (u32, Vec<(String, u32)>) {
    let (_, ty) = m
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no struct `{name}`"));
    let TypeInner::Struct { members, span } = &ty.inner else {
        panic!("`{name}` is not a struct");
    };
    (
        span / 4,
        members
            .iter()
            .map(|mb| (mb.name.clone().unwrap_or_default(), mb.offset / 4))
            .collect(),
    )
}

/// The read side at `tier` filling `fields`, moved to the harness's group 0 (state 0, `ICDescriptor` 1, output 2),
/// with an entry writing each sample's shadow and descriptor.
fn shadow_ic_module(tier: Tier, fields: &[&str]) -> String {
    let l = ledger::layout();
    let entries = gen::validate(&l).expect("the ledger validates");
    let text = read::assemble(&l.words, &entries, tier, fields).unwrap_or_else(|e| panic!("{e}"));
    let mv = |t: String, from: &str, to: &str| {
        assert!(t.contains(from), "no `{from}` in the read side");
        t.replace(from, to)
    };
    let t = mv(text, "@group(1) @binding(0)", "@group(0) @binding(0)");
    let t = mv(t, "@group(1) @binding(2)", "@group(0) @binding(1)");
    let ic: String = IC
        .iter()
        .enumerate()
        .map(|(k, m)| format!("    qa_out[o + {}u] = bitcast<u32>(ic.{m});\n", 12 + k))
        .collect();
    // Constant indices only: the shadow's six components of each of `r_sh` and `p_sh`, in order.
    let shadow: String = ["r_sh", "p_sh"]
        .iter()
        .enumerate()
        .flat_map(|(a, n)| {
            (0..3).flat_map(move |j| {
                ["x", "y"].iter().enumerate().map(move |(c, xy)| {
                    format!(
                        "    qa_out[o + {}u] = bitcast<u32>(s.{n}[{j}].{xy});\n",
                        6 * a + 2 * j + c
                    )
                })
            })
        })
        .collect();
    format!(
        "{t}\n@group(0) @binding(2) var<storage, read_write> qa_out: array<u32>;\n\
         @compute @workgroup_size(64)\n\
         fn qa_main(@builtin(global_invocation_id) id: vec3<u32>) {{\n\
         \x20   let i = id.x;\n\
         \x20   // Both buffers are bound whatever the read side loads: the harness binds what the entry uses.\n\
         \x20   if i >= {SAMPLES}u || arrayLength(&simstate_buffer) == 0u || arrayLength(&ic_buffer) == 0u {{ return; }}\n\
         \x20   let s = sample_read(i, 0.0, false, vec3<f32>(1.0, 1.0, 1.0), ReadParams(0.125, 0.0625, 16u, 1000u));\n\
         \x20   let ic = ic_read(i);\n\
         \x20   let o = {OUT}u * i;\n{shadow}{ic}}}\n"
    )
}

/// One sample's output words, its expected shadow words (`None` at the base tier) and its descriptor's words.
type Drawn = (Vec<u32>, Option<Vec<u32>>, Vec<u32>);

/// Runs `module` at `tier` over hand-filled buffers; returns each sample's output and the expected shadow (`None`
/// at the base tier) and descriptor words, the latter for every member.
fn run_shadow_ic(module: &str, tier: Tier) -> Vec<Drawn> {
    let m = naga::front::wgsl::parse_str(module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(module)));
    let simstate = if tier.has_ftle {
        "SimStateFTLE"
    } else {
        "SimStateBase"
    };
    let (s_words, s_members) = layout(&m, simstate);
    let (i_words, i_members) = layout(&m, "ICDescriptor");
    let state: Vec<u32> = (0..SAMPLES * s_words)
        .map(|k| fill(1, k).to_bits())
        .collect();
    let ic: Vec<u32> = (0..SAMPLES * i_words)
        .map(|k| fill(2, k).to_bits())
        .collect();
    // The output must hold every sample's words; the harness sizes it by the first input.
    assert!(
        state.len() >= (SAMPLES * OUT) as usize,
        "the state buffer is shorter than the output"
    );
    let gpu = GpuHarness::new().expect("a GPU device");
    let out = gpu.run_wgsl(module, "qa_main", &[&state, &ic]);
    let off = |members: &[(String, u32)], n: &str| {
        members
            .iter()
            .find(|(m, _)| m == n)
            .map(|(_, o)| *o)
            .unwrap_or_else(|| panic!("no member `{n}`"))
    };
    (0..SAMPLES)
        .map(|i| {
            let got = out[(i * OUT) as usize..((i + 1) * OUT) as usize].to_vec();
            let shadow = tier.has_ftle.then(|| {
                ["r_sh", "p_sh"]
                    .iter()
                    .flat_map(|n| {
                        let o = i * s_words + off(&s_members, n);
                        (0..6).map(move |j| fill(1, o + j).to_bits())
                    })
                    .collect()
            });
            let desc = IC
                .iter()
                .map(|n| fill(2, i * i_words + off(&i_members, n)).to_bits())
                .collect();
            (got, shadow, desc)
        })
        .collect()
}

/// RQ-228: the shadow reads the stored shadow, every component, at the FTLE tier, and the canonical quiet NaN in
/// every component at the base tier; RQ-227: `ic_read` reads each member in `ic_read` as stored and leaves every other
/// member unloaded, zero (the module's `ic_fields`).
fn check_shadow_and_ic(tier: Tier, ic_fields: &[&str], module: &str) {
    for (i, (got, shadow, desc)) in run_shadow_ic(module, tier).into_iter().enumerate() {
        for j in 0..12 {
            let want = shadow.as_ref().map_or(QNAN, |s| s[j]);
            assert_eq!(
                got[j],
                want,
                "sample {i}: shadow component {j} at {} is {:#010x}, not {want:#010x} (RQ-228)",
                if tier.has_ftle {
                    "the FTLE tier"
                } else {
                    "the base tier"
                },
                got[j]
            );
        }
        for (k, m) in IC.iter().enumerate() {
            let want = if ic_fields.contains(&format!("ic.{m}").as_str()) {
                desc[k]
            } else {
                0
            };
            assert_eq!(
                got[12 + k],
                want,
                "sample {i}: ctx.ic.{m} is {:#010x}, not {want:#010x} (RQ-227, R-378)",
                got[12 + k]
            );
        }
    }
}

/// The module reading the shadow and `ic_fields` at `tier`.
fn shadow_ic_case(tier: Tier, ic_fields: &[&str]) -> String {
    let mut fields = vec!["r_sh", "p_sh"];
    fields.extend_from_slice(ic_fields);
    shadow_ic_module(tier, &fields)
}

#[test]
fn qa_rq228_rq227_shadow_and_ic_read_on_the_gpu() {
    let every: Vec<String> = IC.iter().map(|m| format!("ic.{m}")).collect();
    let every: Vec<&str> = every.iter().map(String::as_str).collect();
    for tier in [FTLE_NO_WORD, BARE] {
        for ic in [&every[..], &["ic.rho_angle", "ic.m1"][..], &[][..]] {
            let module = shadow_ic_case(tier, ic);
            check_shadow_and_ic(tier, ic, &module);
        }
    }
}

negative_control!(
    qa_rq228_rq227_shadow_and_ic_read_on_the_gpu,
    "a base-tier shadow that reads 0.0 is not the canonical NaN",
    expected = "at the base tier is 0x00000000, not 0x7fc00000 (RQ-228)",
    {
        let module = shadow_ic_case(BARE, &[]);
        let from = "vec2<f32>(canonical_nan())";
        assert!(module.contains(from), "the control's pattern is gone");
        check_shadow_and_ic(BARE, &[], &module.replace(from, "vec2<f32>(0.0)"))
    }
);

/// RQ-227: an `ic_read` filling `rho_angle` from another member's reader reads another value.
#[cfg(feature = "controls")]
mod qa_rq228_rq227_shadow_and_ic_read_on_the_gpu_ic {
    use super::*;

    negative_control!(
        qa_rq228_rq227_shadow_and_ic_read_on_the_gpu,
        "`ic.rho_angle` filled through `rho_ratio`'s reader is not the stored `rho_angle`",
        expected = "ctx.ic.rho_angle is",
        {
            let module = shadow_ic_case(FTLE_NO_WORD, &["ic.rho_angle"]);
            let from = "v.rho_angle = ic_read_rho_angle(i);";
            assert!(module.contains(from), "the control's pattern is gone");
            check_shadow_and_ic(
                FTLE_NO_WORD,
                &["ic.rho_angle"],
                &module.replace(from, "v.rho_angle = ic_read_rho_ratio(i);"),
            )
        }
    );
}

/// The FTLE tier's shadow read from the state `r` rather than the stored shadow is caught.
#[cfg(feature = "controls")]
mod qa_rq228_rq227_shadow_and_ic_read_on_the_gpu_ftle {
    use super::*;

    negative_control!(
        qa_rq228_rq227_shadow_and_ic_read_on_the_gpu,
        "an FTLE-tier `r_sh` that reads the stored `r` is not the stored shadow",
        expected = "at the FTLE tier is",
        {
            let module = shadow_ic_case(FTLE_NO_WORD, &[]);
            let from = "simstate_buffer[i].r_sh";
            assert!(module.contains(from), "the control's pattern is gone");
            check_shadow_and_ic(
                FTLE_NO_WORD,
                &[],
                &module.replace(from, "simstate_buffer[i].r"),
            )
        }
    );
}

/// The read side refuses a field list naming no `ICDescriptor` member: an unknown one and the padding (RQ-227: the
/// twelve members, never the struct's padding), naming it.
fn check_ic_names_refused(names: &[&str]) {
    let l = ledger::layout();
    let entries = gen::validate(&l).expect("the ledger validates");
    for n in names {
        match read::wgsl_for(&l.words, &entries, Tier::FULL, &[n]) {
            Ok(_) => panic!("`{n}` was not refused"),
            Err(e) => assert!(
                e.contains(&format!("`{n}`")),
                "the refusal of `{n}` does not name it: {e}"
            ),
        }
    }
    for m in IC {
        let f = format!("ic.{m}");
        read::wgsl_for(&l.words, &entries, Tier::FULL, &[&f])
            .unwrap_or_else(|e| panic!("`{f}` is refused: {e}"));
    }
}

#[test]
fn qa_rq227_read_side_refuses_a_field_that_is_no_ic_member() {
    check_ic_names_refused(&["ic.nope", "ic._pad", "ic.", "ic.r_sh"]);
}

negative_control!(
    qa_rq227_read_side_refuses_a_field_that_is_no_ic_member,
    "a real member is not refused",
    expected = "`ic.virial_ratio` was not refused",
    check_ic_names_refused(&["ic.virial_ratio"])
);
