//! qa's tests for TASK-M0-16, written from REQ-SYS-002 and REQ-SYS-003 (canonical_spec §8; temporal note "The
//! rename"; memory_tiers §1; R-111, R-140, R-259). No retired term is spelled in this file: each is read from
//! `xtask::lint_vocab::TERMS` by a prefix, so the lint, which reads this file, stays clean.
//!
//! - REQ-SYS-002 / R-259: the lint scans `.md` and `.html` under `docs/`, so each term fails naming it in an `.html`
//!   doc, and the `retired-terms` markers exempt a passage there too (R-111).
//! - R-259: the four retired ideas match as case-insensitive phrases: any case, any whitespace run, and a phrase that
//!   reappears in the plural is still the retired idea (the checkpoint and layout-constant phrases with an `s`).
//! - R-140: the identifier terms match case-sensitively as whole identifiers, wherever they stand in code (a path, a
//!   generic, a string literal); the prose "timeout" passes.
//! - R-111: a marked passage holding non-ASCII text (the corpus's retirement passages are full of `→`) must not break
//!   the lint: a term after it is still found, on its own line.
//! - R-259: only `docs/archive/` and `docs/reference/` are excluded, not a sibling whose name starts the same.
//! - The runner fails on a finding and passes on a clean tree.

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::lint_vocab::{check, run, scan, Found, Match, Term, TERMS};

/// The term whose text starts with `prefix` (so no term is spelled here).
fn term(prefix: &str) -> &'static Term {
    TERMS
        .iter()
        .find(|t| t.text.starts_with(prefix))
        .unwrap_or_else(|| panic!("no term starts with {prefix:?}"))
}

/// A fresh tree named `name` under this target's tmp dir, holding each `(path, text)`.
fn tree(name: &str, files: &[(&str, String)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_lint_vocab_m016")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("root created");
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("directory created");
        fs::write(&path, text).expect("file written");
    }
    root
}

/// The (line, term text) of each finding of `scan(text, doc)`.
fn terms_found(text: &str, doc: bool) -> Vec<(usize, &'static str)> {
    scan(text, doc)
        .into_iter()
        .map(|(line, found)| match found {
            Found::Term(t) => (line, t.text),
            other => panic!("unexpected marker finding {other:?} at line {line}"),
        })
        .collect()
}

// --- REQ-SYS-002 / R-259: `.html` docs are read, and the markers work there ---------------------------------------

/// Each term, placed on line 2 of an `.html` doc, fails naming it, its file and its line.
fn check_each_term_in_html(case: &str, pick: fn(usize) -> usize) {
    for (i, t) in TERMS.iter().enumerate() {
        let placed = TERMS[pick(i)].text;
        let root = tree(
            &format!("{case}_{i:02}"),
            &[(
                "docs/gui/page.html",
                format!("<p>An HTML doc.</p>\n<p>{placed}</p>\n"),
            )],
        );
        let found = check(&root).expect("lint ran");
        let named = found.len() == 1
            && found[0].found == Found::Term(t)
            && found[0].line == 2
            && found[0].path == Path::new("docs/gui/page.html")
            && found[0].to_string().contains(t.text);
        assert!(
            named,
            "an .html doc holding term {i} is not failed naming it at docs/gui/page.html:2: {found:?}"
        );
    }
}

#[test]
fn qa_each_term_fails_naming_it_in_an_html_doc() {
    check_each_term_in_html("html", |i| i);
}

negative_control!(
    qa_each_term_fails_naming_it_in_an_html_doc,
    "each term's check given the next term",
    expected = "is not failed naming it",
    check_each_term_in_html("ctl_html", |i| (i + 1) % TERMS.len())
);

/// A retired term wrapped in the markers in an `.html` doc is not read (R-111); `wrapped` false leaves it bare.
fn check_marked_html_passes(wrapped: bool) {
    let t = term("SimRe");
    let body = if wrapped {
        format!("<p>Intro.</p>\n<!-- retired-terms -->\n<p>{} is retired.</p>\n<!-- /retired-terms -->\n", t.text)
    } else {
        format!("<p>Intro.</p>\n<p>{} is retired.</p>\n", t.text)
    };
    let root = tree(
        if wrapped { "html_marked" } else { "html_bare" },
        &[("docs/gui/page.html", body)],
    );
    let found = check(&root).expect("lint ran");
    assert!(
        found.is_empty(),
        "a marked passage in an .html doc is read: {found:?}"
    );
}

#[test]
fn qa_marked_passage_passes_in_an_html_doc() {
    check_marked_html_passes(true);
}

negative_control!(
    qa_marked_passage_passes_in_an_html_doc,
    "the same term with no markers",
    expected = "a marked passage in an .html doc is read",
    check_marked_html_passes(false)
);

// --- R-259: the four retired ideas as case-insensitive phrases ---------------------------------------------------

/// Each retired idea, in another case, with other whitespace, and in the plural, is found once in docs and in code.
fn check_phrase_variants(variants: &[String]) {
    for v in variants {
        for doc in [true, false] {
            let found = terms_found(&format!("Before.\nThe {v} here.\n"), doc);
            assert!(
                found.len() == 1 && found[0].0 == 2,
                "the retired idea {v:?} (doc: {doc}) is not found once on line 2: {found:?}"
            );
        }
    }
}

/// The variants: other case, other whitespace, and the plural, of each phrase term.
fn phrase_variants() -> Vec<String> {
    let mut v = Vec::new();
    for t in TERMS.iter().filter(|t| t.matching == Match::Phrase) {
        v.push(t.text.to_ascii_uppercase());
        v.push(t.text.replace(' ', " \t \n  "));
    }
    let count = term("checkpoint").text;
    let ts = term("TS layout").text;
    let typescript = term("TypeScript").text;
    let fr = term("fro").text;
    v.push(format!("{count}s"));
    v.push(format!("{ts}s"));
    v.push(format!("{typescript}s"));
    v.push(format!("Math.{fr}(x)"));
    v.push(format!("MATH.{}", fr.to_ascii_uppercase()));
    v
}

#[test]
fn qa_retired_ideas_match_as_case_insensitive_phrases() {
    check_phrase_variants(&phrase_variants());
}

negative_control!(
    qa_retired_ideas_match_as_case_insensitive_phrases,
    "a phrase broken by a hyphen, which is not the phrase",
    expected = "is not found once on line 2",
    check_phrase_variants(&[term("checkpoint").text.replace(' ', "-")])
);

// --- R-140: identifiers are whole and case-sensitive -------------------------------------------------------------

/// Each identifier term, placed in each of these code contexts, is found once; the lower-case prose passes.
fn check_identifier_contexts(contexts: &[&str]) {
    for t in TERMS.iter().filter(|t| t.matching == Match::Identifier) {
        for ctx in contexts {
            let line = ctx.replace("{}", t.text);
            let found = terms_found(&format!("// before\n{line}\n"), false);
            assert!(
                found == [(2, t.text)],
                "the identifier in {line:?} is not found once on line 2: {found:?}"
            );
        }
        // The same letters in another case are another word (R-140: the identifier, not the prose).
        let other = if t.text.to_ascii_lowercase() == t.text {
            t.text.to_ascii_uppercase()
        } else {
            t.text.to_ascii_lowercase()
        };
        let found = terms_found(&format!("prose: no separate {other} state\n"), true);
        assert!(
            found.is_empty(),
            "the other-case word {other:?} is found: {found:?}"
        );
    }
}

const CONTEXTS: &[&str] = &[
    "use crate::a::{};",
    "Outcome::{} => 0,",
    "let x: Vec<{}> = v;",
    "f(\"{}\")",
    "x.{}.y",
    "#[{}]",
];

#[test]
fn qa_identifiers_match_whole_and_case_sensitive_in_any_context() {
    check_identifier_contexts(CONTEXTS);
}

negative_control!(
    qa_identifiers_match_whole_and_case_sensitive_in_any_context,
    "the identifier glued to a longer identifier, which is not it",
    expected = "is not found once on line 2",
    check_identifier_contexts(&["let x = {}_v2;"])
);

/// The prose R-140 names as not a retired term ("no separate timeout state", canonical_spec :117 and others) passes
/// in the real docs that hold it.
#[test]
fn qa_prose_timeout_passes_in_the_docs() {
    check_docs_pass(&[
        "docs/contracts/principia_canonical_spec.md",
        "docs/design/principia_systems_architecture.md",
        "docs/design/principia_dd_integrator.md",
        "docs/contracts/principia_integrator_contract.md",
    ]);
}

fn check_docs_pass(docs: &[&str]) {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for d in docs {
        let text = fs::read_to_string(repo.join(d)).unwrap_or_else(|e| panic!("{d}: {e}"));
        let found = scan(&text, true);
        assert!(found.is_empty(), "{d} is flagged: {found:?}");
    }
}

negative_control!(
    qa_prose_timeout_passes_in_the_docs,
    "a doc holding a retired identifier",
    expected = "is flagged",
    check_docs_pass(&["xtask/tests/fixtures/vocab/term_04.md"])
);

// --- R-111: a marked passage with non-ASCII text ------------------------------------------------------------------

/// A doc with a marked passage of `arrows` copies of `→` (one line), then `gap` plain lines, then a retired term on
/// the next line: the term is found on that line, and only it.
fn check_term_after_non_ascii_passage(arrows: usize, gap: usize, want_line_offset: isize) {
    let t = term("SimRe");
    let mut text = String::from("Intro.\n<!-- retired-terms -->\n");
    text.push_str(&"\u{2192}".repeat(arrows));
    text.push_str("\n<!-- /retired-terms -->\n");
    for _ in 0..gap {
        text.push_str("x\n");
    }
    text.push_str(&format!("{} again.\n", t.text));
    let line = 4 + gap + 1;
    let want = (line as isize + want_line_offset) as usize;
    let found = terms_found(&text, true);
    assert!(
        found == [(want, t.text)],
        "a term after a marked passage of {arrows} arrows and {gap} plain lines is not found on line {want}: \
         {found:?}"
    );
}

#[test]
fn qa_term_after_a_non_ascii_marked_passage_is_found_on_its_line() {
    // Right after the passage; and after a gap short enough that the blanked bytes would cross lines.
    check_term_after_non_ascii_passage(1, 0, 0);
    check_term_after_non_ascii_passage(60, 0, 0);
    check_term_after_non_ascii_passage(60, 10, 0);
    check_term_after_non_ascii_passage(61, 3, 0);
}

negative_control!(
    qa_term_after_a_non_ascii_marked_passage_is_found_on_its_line,
    "an ASCII-only passage, required on the line after the term's",
    expected = "is not found on line",
    check_term_after_non_ascii_passage(0, 3, 1)
);

/// The real canonical_spec, whose marked §8 passage holds `→`, with a retired term appended as its last line: the
/// lint flags that line and nothing else.
fn check_term_appended_to_canonical_spec(shift: usize) {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut text =
        fs::read_to_string(repo.join("docs/contracts/principia_canonical_spec.md")).expect("spec");
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let t = term("TileI");
    text.push_str(&format!("{} reappears.\n", t.text));
    let last = text.lines().count();
    let found = terms_found(&text, true);
    assert!(
        found == [(last + shift, t.text)],
        "a term appended to canonical_spec at line {} is found as {found:?}",
        last + shift
    );
}

#[test]
fn qa_term_appended_to_canonical_spec_is_found_on_its_line() {
    check_term_appended_to_canonical_spec(0);
}

negative_control!(
    qa_term_appended_to_canonical_spec_is_found_on_its_line,
    "the line after the appended one",
    expected = "is found as",
    check_term_appended_to_canonical_spec(1)
);

// --- R-259: only `docs/archive/` and `docs/reference/` are excluded ----------------------------------------------

/// A term in each of `placed`: exactly `read` are flagged.
fn check_read(case: &str, placed: &[&str], read: &[&str]) {
    let text = format!("{}\n", term("SimRe").text);
    let files: Vec<(&str, String)> = placed.iter().map(|p| (*p, text.clone())).collect();
    let root = tree(case, &files);
    let found: Vec<PathBuf> = check(&root)
        .expect("lint ran")
        .into_iter()
        .map(|f| f.path)
        .collect();
    let read: Vec<PathBuf> = read.iter().map(PathBuf::from).collect();
    assert_eq!(found, read, "the lint does not flag exactly the siblings");
}

const SIBLINGS: &[&str] = &[
    "docs/archive/a.md",
    "docs/archive2/a.md",
    "docs/contracts/archive/a.md",
    "docs/reference/a.html",
    "docs/reference_notes/a.html",
    "docs/gui/reference/a.html",
    "web/deep/nested/dir/a.ts",
    "xtask/tests/fixtures/vocab2/a.md",
];

#[test]
fn qa_only_archive_and_reference_are_excluded() {
    check_read(
        "siblings",
        SIBLINGS,
        &[
            "docs/archive2/a.md",
            "docs/contracts/archive/a.md",
            "docs/gui/reference/a.html",
            "docs/reference_notes/a.html",
            "web/deep/nested/dir/a.ts",
            "xtask/tests/fixtures/vocab2/a.md",
        ],
    );
}

negative_control!(
    qa_only_archive_and_reference_are_excluded,
    "the siblings required to be excluded",
    expected = "the lint does not flag exactly the siblings",
    check_read("ctl_siblings", SIBLINGS, &["web/deep/nested/dir/a.ts"])
);

// --- The runner ---------------------------------------------------------------------------------------------------

/// `run` over a tree holding `text` in a contract: Ok exactly when `clean`.
fn check_run(case: &str, text: String, clean: bool) {
    let root = tree(case, &[("docs/contracts/c.md", text)]);
    let result = run(&root.join("Cargo.toml"));
    assert_eq!(
        result.is_ok(),
        clean,
        "the runner's result {result:?} does not match a {} tree",
        if clean { "clean" } else { "dirty" }
    );
}

#[test]
fn qa_runner_fails_on_a_finding_and_passes_clean() {
    check_run("run_dirty", format!("{}\n", term("TILE_P").text), false);
    check_run(
        "run_clean",
        "QuadSummary and SAMPLES_PER_QUAD_AXIS.\n".into(),
        true,
    );
}

negative_control!(
    qa_runner_fails_on_a_finding_and_passes_clean,
    "a dirty tree required to pass",
    expected = "does not match a clean tree",
    check_run("ctl_run", format!("{}\n", term("TileS").text), true)
);

// --- Edges: a term at the very start of a file; a stray closing marker before a paired passage -------------------

/// Each term as the first bytes of a file, in docs and in code, is found on line 1.
fn check_term_at_file_start(prefix: &str) {
    for t in TERMS {
        for doc in [true, false] {
            let found = terms_found(&format!("{prefix}{} first.\n", t.text), doc);
            assert!(
                found == [(1, t.text)],
                "a term at the start of a file (doc: {doc}) is not found on line 1: {found:?}"
            );
        }
    }
}

#[test]
fn qa_term_at_the_start_of_a_file_is_found() {
    check_term_at_file_start("");
}

negative_control!(
    qa_term_at_the_start_of_a_file_is_found,
    "the term glued to a leading identifier character, which is not the term",
    expected = "is not found on line 1",
    check_term_at_file_start("x")
);

/// A closing marker with no opening one, then a paired passage holding a term: the stray marker is a finding on its
/// line, the paired passage is not read, and a term after it is found on its line.
fn check_stray_close_then_pair(wrap: bool) {
    let t = term("SimRe");
    let (open, close) = if wrap {
        ("<!-- retired-terms -->", "<!-- /retired-terms -->")
    } else {
        ("", "")
    };
    let text = format!(
        "Intro.\n<!-- /retired-terms -->\n{open}\n{} inside.\n{close}\n{} after.\n",
        t.text, t.text
    );
    let found = scan(&text, true);
    let want = vec![(2, Found::Unopened), (6, Found::Term(t))];
    assert_eq!(
        found, want,
        "a stray closing marker before a paired passage yields other findings"
    );
}

#[test]
fn qa_stray_close_before_a_pair_is_a_finding_and_the_pair_still_works() {
    check_stray_close_then_pair(true);
}

negative_control!(
    qa_stray_close_before_a_pair_is_a_finding_and_the_pair_still_works,
    "the passage left unwrapped, so its term is read",
    expected = "yields other findings",
    check_stray_close_then_pair(false)
);
