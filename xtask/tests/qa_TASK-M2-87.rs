//! qa's tests for TASK-M2-87, written from REQ-INT-115 (the CPU-to-GPU record and table contract; R-926 items 5 and
//! 6, R-935 under RQ-761, R-985, R-989, R-990, R-995, R-1006). REQ-INT-115's verify is a review checklist on contract
//! text, so these read the contracts as text; they check no code (that is REQ-INT-116's, on TASK-M4-05).
//!
//! - The contract's section states every element REQ-INT-115's statement and verify name: the record's content
//!   (REQ-DEC-005's `(m, r, p)`, descriptor and tag words, a branch word, the stop index of RQ-761, the route and, for
//!   a patch, the quad's level and address), where a record is formed (on the CPU, in f64, narrowed once; a patched
//!   sample's `x₀ + J_D·δ` expanded in f64, carrying the centre's words), which quads take the patch (R-990's check,
//!   with R-1006's binade case), what a tier-3 record holds, and `SimState` a separate buffer (REQ-SCHED-012).
//! - "Each replaced line kept as *Was*": every *Was* quote in TASK-M2-87's dated notes is found verbatim in the same
//!   contract outside the note, so the line it replaces is kept, not removed.
//! - The section exists once under the heading REQ-INT-115's source names, and every contract that gained a pointer
//!   to it names that heading.

use std::fs;
use std::path::PathBuf;

use validation::negative_control;

const LOWERING: &str = "docs/contracts/principia_lowering_contract.md";
const SCHEDULER: &str = "docs/contracts/principia_scheduler_contract.md";
const DECODER: &str = "docs/contracts/principia_chart_decoder_contract.md";

/// The section REQ-INT-115's source names.
const HEADING: &str = "The record and table contract (CPU to GPU)";

/// The phrases the section must state, each with what it stands for in REQ-INT-115 (statement and verify detail).
const REQUIRED: &[(&str, &str)] = &[
    ("never decodes", "the entry never decodes (R-926 item 5)"),
    (
        "no chart code",
        "no chart code in the integrator (R-926 item 5)",
    ),
    ("REQ-DEC-005's `(m, r, p)`", "the record's (m, r, p)"),
    ("the descriptor and tag words", "descriptor and tag words"),
    ("a branch word", "a branch word"),
    ("`k_stop", "the stop index"),
    ("RQ-761", "the stop index of RQ-761"),
    ("route, exact or patch", "the route (R-995)"),
    (
        "the quad's level and address",
        "a patch's level and address (R-995)",
    ),
    (
        "`SimState` is a separate buffer",
        "SimState a separate buffer",
    ),
    (
        "REQ-SCHED-012",
        "SimState a separate buffer (REQ-SCHED-012)",
    ),
    (
        "formed on the CPU in f64 and narrowed once",
        "every record formed on the CPU in f64, narrowed once (R-989)",
    ),
    (
        "`x₀ + J_D·δ` expanded in f64",
        "a patched sample's x0 + J_D·δ expanded in f64 (R-989 Q1 (a))",
    ),
    (
        "the quad centre's descriptor, tag and branch words",
        "a patched sample carries the centre's words (R-989)",
    ),
    (
        "REQ-DEC-063",
        "on a quad R-990's check admits (REQ-DEC-063)",
    ),
    (
        "four corners and four edge midpoints",
        "R-990's eight checked points",
    ),
    ("4/5 of the bound", "R-990's comparison at 4/5 of the bound"),
    ("footprint meets no site", "R-990's footprint test"),
    ("crosses zero", "R-990's zero-crossing component"),
    (
        "the quad's smallest value",
        "R-990's zero-crossing comparison value",
    ),
    (
        "smallest sample magnitude",
        "R-1006's binade-drop comparison",
    ),
    ("tier-3", "the tier-3 record"),
    (
        "the sample's own f64 evaluation, narrowed once",
        "a tier-3 record holds its own f64 evaluation (R-989 Q2 (i))",
    ),
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("reading {rel}: {e}"))
}

/// Collapses every whitespace run to one space, so a reflowed line still matches.
fn norm(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The body of the `###` section titled `heading`, up to the next `##` or `###` heading. Panics unless exactly one
/// such heading exists.
fn section(text: &str, heading: &str) -> String {
    let wanted = format!("### {heading}");
    let starts: Vec<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.trim_end() == wanted)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        starts.len(),
        1,
        "qa_m2_87: heading {wanted:?} found {} times, want once",
        starts.len()
    );
    text.lines()
        .skip(starts[0] + 1)
        .take_while(|l| !(l.starts_with("## ") || l.starts_with("### ")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Panics naming each element of REQ-INT-115 the section does not state.
fn check_states_everything(body: &str) {
    let body = norm(body);
    let missing: Vec<&str> = REQUIRED
        .iter()
        .filter(|(phrase, _)| !body.contains(&norm(phrase)))
        .map(|(_, what)| *what)
        .collect();
    assert!(
        missing.is_empty(),
        "qa_m2_87: the record contract misses: {}",
        missing.join("; ")
    );
}

/// The lines of `text` that hold TASK-M2-87's dated notes (each note is one line in these contracts).
fn notes(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.contains("15 Nov 2026 (TASK-M2-87"))
        .map(str::to_string)
        .collect()
}

/// The double-quoted strings after the first `*Was` in `note`.
fn was_quotes(note: &str) -> Vec<String> {
    let Some(at) = note.find("*Was") else {
        return Vec::new();
    };
    note[at..]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// Panics unless every *Was* quote in TASK-M2-87's notes in `text` appears verbatim in `text` with those notes taken
/// out: the replaced line is kept. Returns how many quotes were checked.
fn check_was_kept(name: &str, text: &str) -> usize {
    let notes = notes(text);
    let mut rest = text.to_string();
    for note in &notes {
        rest = rest.replace(note.as_str(), "");
    }
    let rest = norm(&rest);
    let mut checked = 0;
    for note in &notes {
        for quote in was_quotes(note) {
            assert!(
                rest.contains(&norm(&quote)),
                "qa_m2_87: {name}'s *Was* quote is not kept in the contract: {quote:?}"
            );
            checked += 1;
        }
    }
    checked
}

#[test]
fn qa_m2_87_record_contract_states_every_element_of_req_int_115() {
    check_states_everything(&section(&read(LOWERING), HEADING));
}

negative_control!(
    qa_m2_87_record_contract_states_every_element_of_req_int_115,
    "the section with its route bullet (R-995) and its tier-3 bullet (R-989 Q2 (i)) taken out",
    expected = "qa_m2_87: the record contract misses: the route (R-995)",
    {
        let body: String = section(&read(LOWERING), HEADING)
            .lines()
            .filter(|l| !l.starts_with("- its route") && !l.starts_with("- **A tier-3 map's**"))
            .collect::<Vec<_>>()
            .join("\n");
        check_states_everything(&body)
    }
);

#[test]
fn qa_m2_87_every_was_quote_is_kept_verbatim() {
    let mut checked = 0;
    for rel in [LOWERING, SCHEDULER, DECODER] {
        checked += check_was_kept(rel, &read(rel));
    }
    // The lowering contract's Part 3 note quotes two clauses, the scheduler contract's Part 5 note four.
    assert!(
        checked >= 6,
        "qa_m2_87: only {checked} *Was* quotes found in TASK-M2-87's notes, want at least 6"
    );
}

negative_control!(
    qa_m2_87_every_was_quote_is_kept_verbatim,
    "the scheduler contract with the replaced DECODE_MODE paragraph's sentence removed, its *Was* quote then kept nowhere",
    expected = "qa_m2_87: docs/contracts/principia_scheduler_contract.md's *Was* quote is not kept in the contract",
    {
        let text = read(SCHEDULER);
        let original = "When `DECODE_MODE` is set, the reference IC `x₀` and the Jacobian `J_D` travel in a \
                        **separate uniform buffer**, bound only for linear-path quads, not inside the request.";
        let notes = notes(&text);
        let mut edited = text.clone();
        for note in &notes {
            edited = edited.replace(note.as_str(), "@@NOTE@@");
        }
        assert!(edited.contains(original), "the original sentence is in the scheduler contract");
        edited = edited.replacen(original, "", 1);
        for note in &notes {
            edited = edited.replacen("@@NOTE@@", note, 1);
        }
        check_was_kept(SCHEDULER, &edited);
    }
);

/// Panics unless each other contract that points at the record contract (the scheduler's Part 5, the chart and
/// decoder's Part 2) names its heading in a TASK-M2-87 note, and the requirement's source names it too.
fn check_pointers(texts: &[(&str, String)], requirements: &str) {
    for (name, text) in texts {
        let pointed = notes(text).iter().any(|n| n.contains(HEADING));
        assert!(
            pointed,
            "qa_m2_87: {name} has no TASK-M2-87 note naming {HEADING:?}"
        );
    }
    let req = requirements
        .split("\n- id: ")
        .find(|block| block.starts_with("REQ-INT-115\n"))
        .expect("REQ-INT-115 is in docs/history/requirements.yaml");
    assert!(
        req.contains(&format!("section: {HEADING}")),
        "qa_m2_87: REQ-INT-115's source does not name {HEADING:?}"
    );
}

#[test]
fn qa_m2_87_pointers_name_the_record_contract() {
    // The section exists once (section() panics otherwise).
    let _ = section(&read(LOWERING), HEADING);
    let texts: Vec<(&str, String)> = [SCHEDULER, DECODER]
        .into_iter()
        .map(|rel| (rel, read(rel)))
        .collect();
    check_pointers(&texts, &read("docs/history/requirements.yaml"));
}

negative_control!(
    qa_m2_87_pointers_name_the_record_contract,
    "the scheduler contract's pointer naming a heading that does not exist",
    expected =
        "qa_m2_87: docs/contracts/principia_scheduler_contract.md has no TASK-M2-87 note naming",
    {
        let texts: Vec<(&str, String)> = [SCHEDULER, DECODER]
            .into_iter()
            .map(|rel| {
                let text = read(rel);
                let text = if rel == SCHEDULER {
                    text.replace(HEADING, "The record contract (CPU to GPU)")
                } else {
                    text
                };
                (rel, text)
            })
            .collect();
        check_pointers(&texts, &read("docs/history/requirements.yaml"))
    }
);
