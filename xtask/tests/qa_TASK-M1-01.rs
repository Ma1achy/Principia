//! QA tests for TASK-M1-01's change to `cargo xtask lint wgsl`, written from REQ-RENDER-001 as R-378 amends it, not
//! from the implementation: the generated read side's `sample_read` is the one reader of both buffers, at the same
//! sample index, and "each read loading one stored member (of the word, only the components a field needs), never a
//! whole SimStateFTLE". Each test takes the real generated WGSL (the unpack layer, then the read side, linted as its
//! continuation), breaks that rule once, and asserts the lint fires naming its rule. That the repository's own fragment
//! WGSL lints clean, the read side included, is CI's `cargo xtask lint wgsl` step (R-909 retired the test of it here).
//!
//! Each test has a registered negative control (R-176): the same edit kept within the rule, on which it must not fire.

use std::path::Path;

use validation::negative_control;
use xtask::lint_wgsl::{check_read_side, Finding, Rule, GENERATED, READ_SIDE_FILE};

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The read side with `from` replaced by `to` (the pattern must be there), linted as the layer's continuation.
fn edited(from: &str, to: &str) -> Vec<Finding> {
    let rs = read(READ_SIDE_FILE);
    assert!(
        rs.contains(from),
        "the edit's pattern is not in the read side: {from}"
    );
    check_read_side(&read(GENERATED), &rs.replacen(from, to, 1))
        .expect("the edited WGSL parses and validates")
}

fn fires(found: &[Finding], rule: Rule) {
    let hit: Vec<_> = found.iter().filter(|f| f.rule == rule).collect();
    assert!(
        !hit.is_empty(),
        "rule {rule} did not fire; findings: {found:?}"
    );
    for f in hit {
        assert!(
            f.to_string().contains(&format!("[{rule}]")),
            "the finding does not name {rule}: {f}"
        );
    }
}

const OUT: &str = "    var out: SimState;\n";

#[test]
fn qa_lint_per_member_whole_struct_load_in_sample_read_fires() {
    fires(
        &edited(
            OUT,
            &format!("    let qa_whole = simstate_buffer[i];\n{OUT}"),
        ),
        Rule::PerMember,
    );
}

negative_control!(
    qa_lint_per_member_whole_struct_load_in_sample_read_fires,
    "one more member load must not fire",
    expected = "rule per-member did not fire",
    fires(
        &edited(
            OUT,
            &format!("    let qa_member = simstate_buffer[i].S;\n{OUT}")
        ),
        Rule::PerMember
    )
);

#[test]
fn qa_lint_per_member_whole_struct_returned_fires() {
    // a sample_state-style reader, R-343's old form, appended beside the read side
    let rs = format!(
        "{}\nfn sample_state(i: u32) -> SimStateFTLE {{ return simstate_buffer[i]; }}\n",
        read(READ_SIDE_FILE)
    );
    let found = check_read_side(&read(GENERATED), &rs).expect("parses");
    fires(&found, Rule::PerMember);
    fires(&found, Rule::SampleOnly);
}

negative_control!(
    qa_lint_per_member_whole_struct_returned_fires,
    "a helper that takes a loaded member, not the buffer, must not fire",
    expected = "rule per-member did not fire",
    {
        let rs = format!(
            "{}\nfn qa_state(pa: u32) -> u32 {{ return sd_state(pa); }}\n",
            read(READ_SIDE_FILE)
        );
        fires(
            &check_read_side(&read(GENERATED), &rs).expect("parses"),
            Rule::PerMember,
        )
    }
);

const HEAD: &str = "fn sample_read(i: u32,";

#[test]
fn qa_lint_per_member_buffers_at_different_indices_fires() {
    let rs = read(READ_SIDE_FILE);
    assert!(rs.contains(HEAD) && rs.contains("word_buffer[i]"));
    let rs = rs
        .replacen(HEAD, "fn sample_read(i: u32, j: u32,", 1)
        .replacen("word_buffer[i]", "word_buffer[j]", 1);
    fires(
        &check_read_side(&read(GENERATED), &rs).expect("parses"),
        Rule::PerMember,
    );
}

negative_control!(
    qa_lint_per_member_buffers_at_different_indices_fires,
    "a second argument unused for indexing must not fire",
    expected = "rule per-member did not fire",
    {
        let rs = read(READ_SIDE_FILE).replacen(HEAD, "fn sample_read(i: u32, j: u32,", 1);
        fires(
            &check_read_side(&read(GENERATED), &rs).expect("parses"),
            Rule::PerMember,
        )
    }
);
