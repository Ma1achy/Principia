//! QA tests for TASK-M1-13 (REQ-TOOL-026), written from debug_tooling_plan §F's table and the registry rule
//! (gui_state_contract §3: the fragment registry is the scanned filesystem; colour_composition §6: a structural view
//! is a `ctx.quad` preset, `frag/debug/<id>.wgsl`), not from `engine::structural`:
//!
//! - each structural view's graph has exactly one colour node, and its occupant is the shader file
//!   `frag/debug/<id>.wgsl` on disk, read here directly, not through the registry lookup the code under test uses;
//! - that occupant reads the `ctx.quad` member its §F row shows (depth, state, coherence, impurity, spread, suspect
//!   fraction, priority, cache age, ancestor gap) and no other row's member, so one view's shader standing in for
//!   another's is caught on the CPU, without a GPU.
//!
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use engine::stain::{NodeKind, Occupant, StainGraph};
use engine::structural::view;
use validation::negative_control;

/// debug_tooling_plan §F's scalar rows, each view's shader and the `ctx.quad` member it shows (colour_composition §3's
/// quad members; dd_generation_root §3.7a).
const ROWS: [(&str, &str); 9] = [
    ("s_depth", "depth"),
    ("s_state", "state"),
    ("s_coherence", "coherence"),
    ("s_impurity", "impurity"),
    ("s_spread", "spread"),
    ("s_suspect", "suspect_frac"),
    ("s_priority", "priority"),
    ("s_cache_age", "cache_age"),
    ("s_ancestor_gap", "ancestor_gap"),
];

fn shader(id: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../render/shaders/wgsl/frag/debug")
        .join(format!("{id}.wgsl"))
}

/// Whether `src` reads `ctx.quad.<member>`, as a whole identifier.
fn reads(src: &str, member: &str) -> bool {
    let pat = format!("ctx.quad.{member}");
    src.match_indices(&pat).any(|(i, _)| {
        src[i + pat.len()..]
            .chars()
            .next()
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'))
    })
}

/// The occupants of `g`'s colour nodes, every node a wire touches.
fn colour_occupants(g: &StainGraph) -> Vec<Occupant> {
    let mut ids: Vec<_> = g.wires().iter().flat_map(|w| [w.from, w.to]).collect();
    ids.sort();
    ids.dedup();
    ids.into_iter()
        .filter_map(|id| g.node(id))
        .filter(|n| n.kind == NodeKind::Colour)
        .map(|n| n.occupant.clone())
        .collect()
}

/// Checks that `g` is the structural view `id`, showing `member`: one colour node, holding `frag/debug/<id>.wgsl` as
/// on disk, which reads `ctx.quad.<member>` and no other row's member.
fn check_view(g: &StainGraph, id: &str, member: &str) {
    let occupants = colour_occupants(g);
    assert_eq!(occupants.len(), 1, "{id}: colour nodes {occupants:?}");
    let Occupant::Custom(src) = &occupants[0] else {
        panic!(
            "{id}: the colour occupant {:?} is no shader source",
            occupants[0]
        );
    };
    let on_disk = std::fs::read_to_string(shader(id)).unwrap_or_else(|e| panic!("{id}: {e}"));
    assert!(
        src.trim() == on_disk.trim(),
        "{id}: the colour occupant is not frag/debug/{id}.wgsl"
    );
    check_member(id, src, member);
}

#[test]
fn qa_each_structural_view_shows_its_own_shader_and_member() {
    for (id, member) in ROWS {
        let g = view(id).unwrap_or_else(|e| panic!("{id}: {e}"));
        check_view(&g, id, member);
    }
}

negative_control!(
    qa_each_structural_view_shows_its_own_shader_and_member,
    "the quad-depth view's graph taken for the quad-state view fails",
    expected = "s_state: the colour occupant is not frag/debug/s_state.wgsl",
    {
        let g = view("s_depth").expect("a view");
        check_view(&g, "s_state", "state");
    }
);

/// Checks that `src`, the shader of the view whose row shows `member`, reads `ctx.quad.<member>` and no other row's.
fn check_member(id: &str, src: &str, member: &str) {
    assert!(
        reads(src, member),
        "{id}: the shader does not read ctx.quad.{member}"
    );
    for (other, m) in ROWS {
        if m != member {
            assert!(
                !reads(src, m),
                "{id}: the shader reads ctx.quad.{m}, {other}'s member"
            );
        }
    }
}

#[test]
fn qa_each_structural_shader_reads_its_rows_member() {
    assert!(
        !reads("ctx.quad.depthx", "depth"),
        "a longer identifier is no read"
    );
    for (id, member) in ROWS {
        let src = std::fs::read_to_string(shader(id)).unwrap_or_else(|e| panic!("{id}: {e}"));
        check_member(id, &src, member);
    }
}

negative_control!(
    qa_each_structural_shader_reads_its_rows_member,
    "the quad-depth shader checked against the quad-state row fails",
    expected = "s_depth: the shader does not read ctx.quad.state",
    {
        let src = std::fs::read_to_string(shader("s_depth")).expect("s_depth.wgsl");
        check_member("s_depth", &src, "state");
    }
);
