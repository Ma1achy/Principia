//! qa's tests for TASK-M2-60, written from REQ-CHART-112 (a definition requirement) and the rulings it cites.
//!
//! REQ-CHART-112: "The chart path must be defined in the contracts: a view as a point, a path as window edits plus
//! the state blend (R-926 items 1 and 3), a morph defined by its pairing (the same pixel on both charts), its path (by
//! default the shortest in each block's natural geometry: mass in μ_k = ln(m_k/m₀), the logs of the mass ratios, shape
//! on the great circle, momentum in q, time linear) and its easing, timing only (R-943), the other arms (per-axis blend,
//! pair map and per-field) retired as morphs and offered as maps (R-936), another path allowed, not built now and not
//! designed out." Verify: export contract Part 3
//! ("the chart path", the transition at a chart change and "a morph's frames are recorded"), chart decoder contract
//! Part 4 ("Chart paths: a view is a point, every edit a path") and `principia_sequence.md` (§3.3, §3.4, §3.6, §3.7)
//! carry the definition, as ruled.
//!
//! The source wins over the requirement (`plan/MILESTONES.md` § "Assumptions"): R-936 (i-a) retires `AxisBlend`,
//! `PairMap`'s bend and `PerField`, not `Path::Line`, which is the state blend, the morph itself (design § 17.2), so
//! `qa_m260_line_is_the_morph_not_a_retired_arm` asserts R-936's list, and R-943's time block is asserted with the
//! other three.
//!
//! These read the checked-in documents: `plan/check_plan.py` only resolves the cited headings, so without these a
//! later edit could empty the definition and every check stay green. Each test registers a negative control (R-176),
//! the document edited to lose the one thing that test asserts.

// The file name `qa_TASK-M2-60` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;

const EXPORT: &str = "docs/contracts/principia_export_animation_contract.md";
const DECODER: &str = "docs/contracts/principia_chart_decoder_contract.md";
const SEQUENCE: &str = "docs/design/principia_sequence.md";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// `text` with `from` replaced by `to` once, `from` required to be there (the controls' edits).
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "no {from:?} to edit");
    text.replacen(from, to, 1)
}

/// Whitespace collapsed, so a phrase matches across the documents' line wraps.
fn flat(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The lines from the heading line `heading` up to the next heading of the same or a higher level.
fn section<'a>(text: &'a str, heading: &str) -> Vec<&'a str> {
    let level = heading.chars().take_while(|&c| c == '#').count();
    let mut lines = text.lines();
    if lines.by_ref().find(|l| l.trim_end() == heading).is_none() {
        return Vec::new();
    }
    lines
        .take_while(|l| {
            let h = l.chars().take_while(|&c| c == '#').count();
            !(h > 0 && h <= level && l[h..].starts_with(' '))
        })
        .collect()
}

/// The list item starting with `lead` (its indent included), up to the next item at the same or a lower indent.
fn item<'a>(lines: &[&'a str], lead: &str) -> Vec<&'a str> {
    let indent = lead.len() - lead.trim_start().len();
    let Some(start) = lines.iter().position(|l| l.starts_with(lead)) else {
        return Vec::new();
    };
    let mut out = vec![lines[start]];
    for l in &lines[start + 1..] {
        let i = l.len() - l.trim_start().len();
        let blank = l.trim().is_empty();
        if !blank && (i < indent || i == indent && l.trim_start().starts_with("- ")) {
            break;
        }
        out.push(l);
    }
    out
}

fn part3(export: &str) -> Vec<&str> {
    let p = section(export, "## Part 3 — The keyframe system (retained)");
    assert!(!p.is_empty(), "the export contract has no Part 3");
    p
}

/// Part 3's "the chart path" item.
fn chart_path(export: &str) -> Vec<&str> {
    let b = item(&part3(export), "- **The chart path");
    assert!(!b.is_empty(), "Part 3 has no chart-path item");
    b
}

/// A sub-item of the chart path, by its italic lead.
fn sub(export: &str, lead: &str) -> String {
    let s = item(&chart_path(export), &format!("  - *{lead}"));
    assert!(!s.is_empty(), "the chart path has no {lead:?} sub-item");
    flat(&s.join("\n"))
}

// ---- a view is a point, a path window edits plus the state blend (R-926 item 1, R-850, R-925) ---------------------

fn check_view_and_path(export: &str) {
    let s = sub(export, "A view is a point");
    for want in [
        "A view is a point in chart space",
        "a flat window of a centre and a basis",
        "A path is window edits plus the state blend.",
        "only a change of map is a blend",
        "tilt, zoom, chart switch, lock, Save view as chart and morph",
    ] {
        assert!(s.contains(want), "a view and a path: no {want:?} in {s}");
    }
}

#[test]
fn qa_m260_a_view_is_a_point_and_a_path_window_edits_plus_the_blend() {
    check_view_and_path(&read(EXPORT));
}

negative_control!(
    qa_m260_a_view_is_a_point_and_a_path_window_edits_plus_the_blend,
    "the path made window edits only, with no blend",
    expected = "a view and a path",
    check_view_and_path(&edited(
        &read(EXPORT),
        "**A path is window edits plus the state blend.**",
        "**A path is window edits.**",
    ))
);

#[test]
fn qa_m260_the_chart_path_is_in_part_3() {
    assert!(!chart_path(&read(EXPORT)).is_empty());
}

negative_control!(
    qa_m260_the_chart_path_is_in_part_3,
    "the chart-path item moved out of Part 3, past Part 4's heading",
    expected = "Part 3 has no chart-path item",
    {
        let text = read(EXPORT);
        let start = text.find("- **The chart path").expect("the item");
        let moved = format!("{}## Part 4 — moved\n\n{}", &text[..start], &text[start..]);
        chart_path(&moved);
    }
);

// ---- the morph: pairing, path, easing (R-943), exact at both ends (R-926 item 3) ------------------------------------

fn check_morph(export: &str) {
    let s = sub(export, "The morph.");
    for want in [
        "\"Morph\" names the state blend only",
        "**the pairing**, the same pixel on both charts",
        "**the path**",
        "**the easing**, its timing and nothing else",
        "Both ends are exact",
    ] {
        assert!(s.contains(want), "the morph: no {want:?} in {s}");
    }
}

#[test]
fn qa_m260_a_morph_is_pairing_path_and_easing_timing_only() {
    check_morph(&read(EXPORT));
}

negative_control!(
    qa_m260_a_morph_is_pairing_path_and_easing_timing_only,
    "the easing allowed to change more than timing",
    expected = "the morph",
    check_morph(&edited(
        &read(EXPORT),
        "**the easing**, its timing and nothing else",
        "**the easing**, its timing and its shape",
    ))
);

// ---- the default path: each block in its natural geometry (R-943, R-936 (ii-a)) -----------------------------------

fn check_default_path(export: &str) {
    let b = chart_path(export);
    let lead = item(&b, "  - *The default path*");
    assert!(!lead.is_empty(), "the default path: no sub-item");
    assert!(
        flat(&lead.join("\n")).contains("the shortest in each block's natural geometry"),
        "the default path: not the shortest in each block's natural geometry"
    );
    let rows: Vec<&str> = lead.iter().map(|l| l.trim()).collect();
    let row = |block: &str| {
        rows.iter()
            .find(|r| r.starts_with(&format!("{block} ")))
            .copied()
    };
    let mass = row("mass").unwrap_or_else(|| panic!("the default path has no mass row"));
    assert!(
        mass.contains("μ_k = ln(m_k/m₀)") && mass.contains("a + s(b − a)"),
        "the default path's mass row is not linear in μ_k = ln(m_k/m₀): {mass}"
    );
    let shape = row("shape").unwrap_or_else(|| panic!("the default path has no shape row"));
    assert!(
        shape.contains("great circle"),
        "the default path's shape row is not the great circle: {shape}"
    );
    let momentum =
        row("momentum").unwrap_or_else(|| panic!("the default path has no momentum row"));
    assert!(
        momentum.contains(" q ") && momentum.contains("a + s(b − a)"),
        "the default path's momentum row is not linear in q: {momentum}"
    );
    let time = row("time").unwrap_or_else(|| panic!("the default path has no time row"));
    assert!(
        time.contains("a + s(b − a)"),
        "the default path's time row is not linear: {time}"
    );
}

#[test]
fn qa_m260_the_default_path_blocks() {
    check_default_path(&read(EXPORT));
}

negative_control!(
    qa_m260_the_default_path_blocks,
    "the mass row blending the masses, not their log-ratios",
    expected = "the default path's mass row is not linear in μ_k",
    check_default_path(&edited(
        &read(EXPORT),
        "μ_k = ln(m_k/m₀)   a + s",
        "m_k                a + s"
    ))
);

// ---- the other arms are maps, not morphs (R-936 (i-a)); `Path::Line` is the morph -----------------------------------

fn not_morphs(export: &str) -> String {
    sub(export, "Not morphs.")
}

fn check_other_arms(export: &str) {
    let s = not_morphs(export);
    let retired = s.split("retired as morphs").next().unwrap_or("");
    for arm in ["`AxisBlend`", "`PairMap`", "`PerField`"] {
        assert!(
            retired.contains(arm),
            "the retired arms: {arm} missing from {s}"
        );
    }
    assert!(
        s.contains("retired as morphs and offered as maps"),
        "the retired arms are not offered as maps: {s}"
    );
}

#[test]
fn qa_m260_the_other_arms_are_retired_as_morphs_and_offered_as_maps() {
    check_other_arms(&read(EXPORT));
}

negative_control!(
    qa_m260_the_other_arms_are_retired_as_morphs_and_offered_as_maps,
    "the per-field arm dropped from the retired list",
    expected = "the retired arms: `PerField` missing",
    check_other_arms(&edited(
        &read(EXPORT),
        "per-field path (`PerField`), are retired",
        "per-field path, are retired",
    ))
);

fn check_line_kept(export: &str) {
    let s = not_morphs(export);
    let retired = s.split("retired as morphs").next().unwrap_or("");
    assert!(
        !retired.contains("`Line`") && !retired.contains("Path::Line"),
        "the retired arms name Path::Line, the state blend itself (R-936 (i-a) retires only AxisBlend, PairMap's bend \
         and PerField): {s}"
    );
}

#[test]
fn qa_m260_line_is_the_morph_not_a_retired_arm() {
    check_line_kept(&read(EXPORT));
}

negative_control!(
    qa_m260_line_is_the_morph_not_a_retired_arm,
    "the line arm listed among the retired arms",
    expected = "the retired arms name Path::Line",
    check_line_kept(&edited(
        &read(EXPORT),
        "the pair-map bend (`PairMap`) and",
        "the pair-map bend (`PairMap`), the line (`Line`) and",
    ))
);

// ---- another path: allowed, not built now, not designed out (R-943) -------------------------------------------------

fn check_another_path(export: &str) {
    let s = sub(export, "Another path");
    for want in [
        "is allowed if both ends are exactly the end charts, every in-between frame is a valid system and it is continuous",
        "Only the default is built now",
        "none is designed out",
        "nothing assumes the default",
    ] {
        assert!(s.contains(want), "another path: no {want:?} in {s}");
    }
}

#[test]
fn qa_m260_another_path_allowed_not_built_not_designed_out() {
    check_another_path(&read(EXPORT));
}

negative_control!(
    qa_m260_another_path_allowed_not_built_not_designed_out,
    "the default made the only path the record can hold",
    expected = "another path",
    check_another_path(&edited(
        &read(EXPORT),
        "nothing assumes the default",
        "the record assumes the default",
    ))
);

// ---- two tests per operation, never across families (R-850 as amended) ---------------------------------------------

fn check_two_tests(export: &str) {
    let s = sub(export, "Two tests per operation");
    for want in [
        "reproduces today's behaviour bitwise",
        "reproduces its own path family's formula",
        "never compared across families",
    ] {
        assert!(
            s.contains(want),
            "two tests per operation: no {want:?} in {s}"
        );
    }
}

#[test]
fn qa_m260_two_tests_per_operation_never_across_families() {
    check_two_tests(&read(EXPORT));
}

negative_control!(
    qa_m260_two_tests_per_operation_never_across_families,
    "R-850's withdrawn test (2), agreement with the general formula to a tolerance",
    expected = "two tests per operation",
    check_two_tests(&edited(
        &read(EXPORT),
        "never compared across families",
        "compared with the general morph to a stated tolerance",
    ))
);

// ---- the transition at a chart change: cut, dissolve or morph (R-850, R-949) ----------------------------------------

/// `s` up to its first *Was* quote: the rule as it now reads.
fn current(s: &str) -> &str {
    s.split("*Was").next().unwrap_or(s)
}

fn check_transition(export: &str) {
    let p = part3(export);
    let interp = flat(&item(&p, "- **What interpolates:**").join("\n"));
    let now = current(&interp);
    assert!(
        now.contains("a chart *switch* is a cut, a dissolve (two renders composited) or a morph")
            && now.contains("never parameter interpolation across chart types"),
        "a chart switch: no morph among its transitions: {now}"
    );
    let all = flat(&p.join("\n"));
    assert!(
        all.contains("`cut`, `dissolve { length }` or `morph { path, retime, align }`"),
        "a chart switch: the segment's transition does not name the morph's path"
    );
    assert!(
        all.contains(
            "A changed link id takes the segment's transition, a cut, a dissolve or a morph"
        ),
        "a chart switch: a changed link id is still only a cut"
    );
}

#[test]
fn qa_m260_a_chart_change_may_morph() {
    check_transition(&read(EXPORT));
}

negative_control!(
    qa_m260_a_chart_change_may_morph,
    "Part 3's rule as it read before R-850, chart switches only cuts or dissolves",
    expected = "a chart switch: no morph",
    check_transition(&edited(
        &read(EXPORT),
        "a chart *switch* is a cut, a dissolve (two renders composited) or a morph (the state blend of **the chart path** below)",
        "chart *switches* are cuts or dissolves (two renders composited)",
    ))
);

// ---- a morph's frames are recorded ----------------------------------------------------------------------------------

fn check_record(export: &str) {
    let r = item(&part3(export), "- **A morph's frames are recorded");
    assert!(!r.is_empty(), "the frame record: no item");
    let r = flat(&r.join("\n"));
    for want in [
        "own `SimConfig`",
        "at a keyframe's own frame the keyframe's",
        "its two end charts, its path's name and its path parameter",
        "The easing is not in it",
        "a morph: every frame a different decoder",
    ] {
        assert!(r.contains(want), "the frame record: no {want:?} in {r}");
    }
}

#[test]
fn qa_m260_a_morph_frame_records_ends_path_and_parameter() {
    check_record(&read(EXPORT));
}

negative_control!(
    qa_m260_a_morph_frame_records_ends_path_and_parameter,
    "the path's name dropped from the frame record",
    expected = "the frame record",
    check_record(&edited(
        &read(EXPORT),
        "its two end charts, its path's name and its path parameter",
        "its two end charts and its path parameter",
    ))
);

// ---- the chart decoder contract's Part 4 carries it -----------------------------------------------------------------

fn check_decoder(decoder: &str) {
    let p4 = section(
        decoder,
        "## Part 4 — Navigation is chart construction (pan, slice, zoom, tilt, lock)",
    );
    assert!(!p4.is_empty(), "the decoder contract has no Part 4");
    let h =
        "### Chart paths: a view is a point, every edit a path (R-850, R-925, R-926, R-936, R-943)";
    assert!(
        p4.iter().any(|l| l.trim_end() == h),
        "Part 4 has no chart-paths subsection"
    );
    let s = flat(&section(decoder, h).join("\n"));
    for want in [
        "A view is a point in chart space",
        "A path is window edits plus the state blend",
        "pairing (the same pixel on both charts)",
        "`μ_k = ln(m_k/m₀)`",
        "great circle",
        "timing only",
        "retired as morphs and offered as maps",
        "another path is allowed, not built now and not designed out",
        "`principia_export_animation_contract.md` Part 3",
    ] {
        assert!(s.contains(want), "Part 4's chart paths: no {want:?} in {s}");
    }
}

#[test]
fn qa_m260_decoder_part_4_carries_the_chart_path() {
    check_decoder(&read(DECODER));
}

negative_control!(
    qa_m260_decoder_part_4_carries_the_chart_path,
    "the subsection placed under Part 5",
    expected = "Part 4 has no chart-paths subsection",
    {
        let text = read(DECODER);
        let h = "### Chart paths: a view is a point, every edit a path";
        let at = text.find(h).expect("the subsection");
        check_decoder(&format!(
            "{}## Part 5 — moved\n\n{}",
            &text[..at],
            &text[at..]
        ));
    }
);

// ---- the Sequence: §3.3, §3.4, §3.6, §3.7 --------------------------------------------------------------------------

fn check_sequence(seq: &str) {
    for (heading, want) in [
        ("### 3.3 The timeline and its lanes", "shows a **cut mark** at its second keyframe **unless it morphs**"),
        ("### 3.4 Segments: easing per segment, and transitions", "**A new chart-change segment is a morph** on every pair of ends"),
        ("### 3.6 Preview and the proof strip", "a morph segment driven live, every frame the blend at its path parameter and never a cut"),
        ("### 3.7 The planner and export", "a morph, \"a morph: every frame a different decoder\", with its path's name"),
    ] {
        let s = flat(&section(seq, heading).join("\n"));
        assert!(s.contains(want), "the Sequence {heading}: no {want:?}");
    }
}

#[test]
fn qa_m260_the_sequence_carries_the_morph() {
    check_sequence(&read(SEQUENCE));
}

negative_control!(
    qa_m260_the_sequence_carries_the_morph,
    "§3.6's preview cutting a morph as it cuts a dissolve",
    expected = "the Sequence ### 3.6",
    check_sequence(&edited(
        &read(SEQUENCE),
        "and a morph segment driven live, every frame the blend at its path parameter\n  and never a cut",
        "and a morph segment as a cut",
    ))
);

/// R-1028 restated §3.4's default ("on every pair of ends"), and the old wording survives in the section's *Was*
/// quote, so the pin must still tell the live sentence from the quote.
#[test]
fn qa_m260_a_new_chart_change_segment_morphs_on_every_pair_of_ends() {
    check_sequence(&read(SEQUENCE));
}

negative_control!(
    qa_m260_a_new_chart_change_segment_morphs_on_every_pair_of_ends,
    "§3.4's live sentence reverted to R-1028's *Was* (a morph only where the ends allow)",
    expected = "the Sequence ### 3.4",
    check_sequence(&edited(
        &read(SEQUENCE),
        "**A new chart-change segment is a morph** on every\n  pair of ends (R-850, R-1028)",
        "**A new chart-change segment is a morph** where the ends allow (R-850; which ends allow is RQ-830)",
    ))
);

// ---- fix round 1: the ends never run the blend (R-926 item 3; design § 17.4) ---------------------------------------

fn check_ends_exact(export: &str) {
    let s = sub(export, "The morph.");
    for want in [
        "at `s = 0` every pixel is chart A's system and at `s = 1` chart B's, bit for bit",
        "The ends never run the blend",
        "`s` is compared exactly, `0.0` taking A's own path and `1.0` B's, never left to arithmetic",
    ] {
        assert!(s.contains(want), "the ends: no {want:?} in {s}");
    }
}

#[test]
fn qa_m260_the_ends_compare_s_exactly() {
    check_ends_exact(&read(EXPORT));
}

negative_control!(
    qa_m260_the_ends_compare_s_exactly,
    "the end frames left to the blend's arithmetic, `a + (b − a)` taken as `b`",
    expected = "the ends",
    check_ends_exact(&edited(
        &read(EXPORT),
        "`s` is\n    compared exactly, `0.0` taking A's own path and `1.0` B's, never left to arithmetic",
        "the blend\n    runs at every `s`, `0.0` and `1.0` included",
    ))
);

// ---- fix round 1: the frame record by value (design § 17.5; REQ-CHART-112's "a morph's frames are recorded") --------

fn check_record_by_value(export: &str) {
    let r = item(&part3(export), "- **A morph's frames are recorded");
    assert!(!r.is_empty(), "the frame record: no item");
    let r = flat(&r.join("\n"));
    for want in [
        "recorded **by value**, `(A, B, path, s)`",
        "`A` and `B` each the whole end chart and view, never a name",
        "`s` the path parameter as its f64 bits, in the canonical serialisation (R-309)",
        "`g(s)` recomputed from them, never stored in its place",
        "the alignment `T`",
        "under a lock, the anchor's words",
        "It round-trips per R-484",
        "A frame of a segment that does not morph is a plain view",
    ] {
        assert!(
            r.contains(want),
            "the frame record by value: no {want:?} in {r}"
        );
    }
}

#[test]
fn qa_m260_a_morph_frame_is_recorded_by_value() {
    check_record_by_value(&read(EXPORT));
}

negative_control!(
    qa_m260_a_morph_frame_is_recorded_by_value,
    "the end charts recorded by name, not by value",
    expected = "the frame record by value",
    check_record_by_value(&edited(
        &read(EXPORT),
        "each the whole end chart and\n  view, never a name",
        "each the end chart's name",
    ))
);

fn check_s_bits(export: &str) {
    let r = flat(&item(&part3(export), "- **A morph's frames are recorded").join("\n"));
    assert!(
        r.contains("`s` the path parameter as its f64 bits") && r.contains("the eased `s` itself"),
        "the recorded s: not the eased s as its f64 bits"
    );
}

#[test]
fn qa_m260_the_recorded_s_is_its_f64_bits() {
    check_s_bits(&read(EXPORT));
}

negative_control!(
    qa_m260_the_recorded_s_is_its_f64_bits,
    "s recorded at f32 (REQ-CHART-113's control, s at f32)",
    expected = "the recorded s",
    check_s_bits(&edited(&read(EXPORT), "as its f64 bits", "as an f32"))
);

// ---- fix round 1: a morph is tested against the block line, its own family (R-850; design § 17.3) ------------------

fn check_morph_family(export: &str) {
    let s = current(&sub(export, "Two tests per operation")).to_owned();
    assert!(
        s.contains("a morph against the block line"),
        "a morph's family: not the block line in {s}"
    );
    assert!(
        !s.contains("the latent line"),
        "a morph's family: the latent line, across families, in {s}"
    );
}

#[test]
fn qa_m260_a_morph_is_tested_against_the_block_line() {
    check_morph_family(&read(EXPORT));
}

negative_control!(
    qa_m260_a_morph_is_tested_against_the_block_line,
    "a morph tested against the latent line (\"blend in Z only\", withdrawn by RQ-736)",
    expected = "a morph's family",
    check_morph_family(&edited(
        &read(EXPORT),
        "a morph against the block line",
        "a morph against the latent line",
    ))
);

// ---- fix round 1: the great circle's tie, and what is left open, not guessed (design § 17.2; R-82) ------------------

fn check_tie_and_open(export: &str) {
    let b = flat(&chart_path(export).join("\n"));
    for want in [
        "at `n̂_B = −n̂_A` the great circle is not unique",
        "a declared seam",
        "broken by a pure function of the pixel in R-82's pattern",
    ] {
        assert!(b.contains(want), "the great circle's tie: no {want:?}");
    }
    let seg = current(&sub(export, "In a segment.")).to_owned();
    assert!(
        !seg.contains("and the playhead follow this Part's rules"),
        "a morph segment: the playhead's rule assumed for the time block: {seg}"
    );
}

#[test]
fn qa_m260_the_tie_is_a_declared_seam_and_the_playhead_left_open() {
    check_tie_and_open(&read(EXPORT));
}

negative_control!(
    qa_m260_the_tie_is_a_declared_seam_and_the_playhead_left_open,
    "the time block's meeting with the playhead filled by the generic rule (RQ-831 guessed; deferred to M8 by R-1026)",
    expected = "a morph segment: the playhead's rule assumed",
    check_tie_and_open(&edited(
        &read(EXPORT),
        "the render uniforms follow this Part's rules for every segment, and how the\n    playhead's policy combines with the time block is RQ-831's.",
        "the render uniforms and the playhead follow this Part's rules for every segment.",
    ))
);

// ---- fix round 1: a mid-morph frame never claims a held value (R-850 item 3; R-965 item 4; design § 17.11) ----------

fn check_no_held_claim(export: &str) {
    let r = flat(&item(&part3(export), "- **A morph's frames are recorded").join("\n"));
    for want in [
        "**A mid-morph frame never claims a held value**",
        "save where its path holds it",
        "is not held and is labelled `hold off mid-path`",
    ] {
        assert!(r.contains(want), "a mid-morph hold: no {want:?}");
    }
}

#[test]
fn qa_m260_a_mid_morph_frame_never_claims_a_held_value() {
    check_no_held_claim(&read(EXPORT));
}

negative_control!(
    qa_m260_a_mid_morph_frame_never_claims_a_held_value,
    "a value on at one end only claimed as held mid-path",
    expected = "a mid-morph hold",
    check_no_held_claim(&edited(
        &read(EXPORT),
        "is not held and is labelled `hold off mid-path`",
        "is held as at that end",
    ))
);

// ---- re-check: the mass block saturates on the default path, never blank (R-975, closing RQ-787 (a)) ----------------

fn check_mass_saturation(export: &str) {
    let b = flat(&chart_path(export).join("\n"));
    for want in [
        "R-975",
        "where the blended masses raise `M01_TINY` between two valid ends",
        "the pixel saturates and is never blank",
        "holds the blended `m₁ : m₀`",
        "`M₀₁ = ε_M01`",
        "the rest of the blend unchanged",
        "continuous in `s` and across pixels",
        "R-936's log-ratio path stands for every pair",
        "is the building task's definition under physics review",
    ] {
        assert!(b.contains(want), "the mass block's saturation: no {want:?}");
    }
}

#[test]
fn qa_m260_the_mass_block_saturates_never_blank() {
    check_mass_saturation(&read(EXPORT));
}

negative_control!(
    qa_m260_the_mass_block_saturates_never_blank,
    "a blend's interior pixel below M01_TINY left blank (RQ-787's rejected reading)",
    expected = "the mass block's saturation",
    check_mass_saturation(&edited(
        &read(EXPORT),
        "the pixel saturates and is never\n    blank",
        "the pixel is\n    blank",
    ))
);
