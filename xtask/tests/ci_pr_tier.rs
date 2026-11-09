//! The PR tier, the owner's CI model (approved 9 Nov 2026, item 1): "Every PR (required; target < 30 min), in parallel,
//! fmt first (fail fast) … Product tests: changed crates + reverse deps …, excluding the slow tier (tests > 60 s by
//! name) and in-depth GPU tests … lavapipe GPU smoke set; Metal smoke set on GPU-path PRs … Process tests only when the
//! PR changes .github/, xtask/, scripts/, plan/, decisions.md, REVIEW_QUEUE.md or docs/".
//!
//! - `ci_pr_tier_slow_tier_is_named_with_its_time`: `.config/nextest.toml`'s `pr` and `process` profiles each leave out
//!   their slow tier by name, every test of it under a table naming it with its measured time, over 60 s, and the
//!   table and the filter name the same tests.
//! - `ci_pr_tier_slow_tier_holds_only_the_approved_physics_tests`: the `pr` profile's slow tier holds xtask's tests,
//!   the in-depth GPU tests and exactly the 34 physics, validation and render tests the owner moved into it (his ruling
//!   of 9 Nov 2026, verbatim: "1: move them."), answering R-904's "moving any physics or validation test into the slow
//!   tier comes to me first"; no other enters it without a ruling. `ci_pr_tier_slow_tier_keeps_the_approved_physics_tests`
//!   runs the same check, its control dropping one of the 34.
//! - `ci_pr_tier_profiles_split_the_tests`: what `pr` leaves out by binary is exactly the `process` profile's binaries,
//!   the `gpu-kernel` profile's (the in-depth GPU tests) and the golden and screenshot runners' three (the owner,
//!   9 Nov 2026: "per merge, never retired"), so no binary falls out of both `pr` and `process` unnamed.
//! - `ci_pr_tier_smoke_tests_exist`: each test the `gpu-smoke` profile names by `test(=…)` is a test of its binary, so a
//!   renamed test cannot leave a GPU stage silently untested.
//! - `ci_pr_tier_smoke_sets_run_the_lattice_tests`: the `gpu-smoke` profile selects gui's lattice tests and both GPU
//!   smoke sets, `gpu-lavapipe` and `gpu-metal`, run them under it with nothing letting a failure through
//!   (REQ-GUI-412: "CI's lavapipe and Metal block").
//! - `ci_pr_tier_required_checks_skip_inside_the_job`: in `ci.yml` every job bearing a required check of the PR tier
//!   runs on every pull request (`if: always() && github.event_name == 'pull_request'`, or `if: always()` for the GPU
//!   smoke sets, which run on a push too), needs `changes`, and fails at
//!   its first step unless `changes` succeeded (R-656: the skips are inside the job), and no step of the workflow
//!   sleeps or polls.
//!
//! Each test has its registered negative control (R-176), which feeds the same check a configuration or workflow
//! differing in the one respect it turns on.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use validation::negative_control;

/// The PR tier's required checks `ci.yml` bears, by job name.
const REQUIRED: [&str; 6] = ["fmt", "clippy", "build", "ci", "gpu-lavapipe", "gpu-metal"];

/// The three binaries the owner moved to each merge (9 Nov 2026).
const PER_MERGE: [&str; 3] = [
    "xtask::qa_TASK-M0-06_list",
    "xtask::qa_TASK-M0-06_runner",
    "xtask::qa_TASK-M0-20",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `rel`'s text with its first `from` replaced by `to`; a control whose edit target is gone fails here.
#[cfg(feature = "controls")]
fn edited(rel: &str, from: &str, to: &str) -> String {
    let text = read(rel);
    assert!(
        text.contains(from),
        "ci_pr_tier: {rel} has no {from:?} to edit"
    );
    text.replacen(from, to, 1)
}

/// A profile of `.config/nextest.toml`: the comment lines just above its header and its `default-filter`.
struct Profile {
    comments: Vec<String>,
    filter: String,
}

fn profile(toml: &str, name: &str) -> Profile {
    let lines: Vec<&str> = toml.lines().collect();
    let header = format!("[profile.{name}]");
    let at = lines
        .iter()
        .position(|l| l.trim() == header)
        .unwrap_or_else(|| panic!("ci_pr_tier: no {header}"));
    let mut comments = Vec::new();
    let mut i = at;
    while i > 0 && lines[i - 1].starts_with('#') {
        i -= 1;
        comments.push(lines[i].to_owned());
    }
    comments.reverse();
    let start = lines[at..]
        .iter()
        .position(|l| l.starts_with("default-filter = \"\"\""))
        .map(|p| at + p + 1)
        .unwrap_or_else(|| panic!("ci_pr_tier: {header} has no default-filter"));
    let end = lines[start..]
        .iter()
        .position(|l| l.trim() == "\"\"\"")
        .map(|p| start + p)
        .unwrap_or_else(|| panic!("ci_pr_tier: {header}'s filter is not closed"));
    Profile {
        comments,
        filter: lines[start..end].join("\n"),
    }
}

/// Each `(binary_id(=B) & test(=T))` of a filter, as (B, T).
fn named_tests(filter: &str) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    let mut rest = filter;
    while let Some(at) = rest.find("(binary_id(=") {
        rest = &rest[at + "(binary_id(=".len()..];
        let Some(close) = rest.find(')') else { break };
        let binary = rest[..close].to_owned();
        let after = &rest[close + 1..];
        if let Some(t) = after.strip_prefix(" & test(=") {
            if let Some(end) = t.find(')') {
                out.insert((binary, t[..end].to_owned()));
            }
        }
    }
    out
}

/// Each bare `binary_id(=B)` of a filter, one not joined to a `test(…)`.
fn bare_binaries(filter: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in filter.lines() {
        let t = line.trim().trim_start_matches("| ").trim();
        if let Some(b) = t.strip_prefix("binary_id(=") {
            if let Some(b) = b.strip_suffix(')') {
                if !b.contains(')') {
                    out.insert(b.to_owned());
                }
            }
        }
    }
    out
}

/// The table under a profile's comments: each `#   <seconds> s  <binary> <test>` line.
fn timed(comments: &[String]) -> BTreeMap<(String, String), f64> {
    let mut out = BTreeMap::new();
    for line in comments {
        let parts: Vec<&str> = line.trim_start_matches('#').split_whitespace().collect();
        if let [secs, "s", binary, test] = parts.as_slice() {
            if let Ok(secs) = secs.parse::<f64>() {
                out.insert((binary.to_string(), test.to_string()), secs);
            }
        }
    }
    out
}

fn check_slow_tier(toml: &str) {
    for name in ["pr", "process"] {
        let p = profile(toml, name);
        let table = timed(&p.comments);
        let named = named_tests(&p.filter);
        assert!(
            !table.is_empty(),
            "ci_pr_tier: profile {name} names no slow tier"
        );
        for ((binary, test), secs) in &table {
            assert!(
                *secs > 60.0,
                "ci_pr_tier: profile {name}'s slow tier lists {binary} {test} at {secs} s, not over 60 s"
            );
            assert!(
                named.contains(&(binary.clone(), test.clone())),
                "ci_pr_tier: profile {name}'s slow tier lists {binary} {test}, which its filter does not leave out"
            );
        }
        for (binary, test) in &named {
            assert!(
                table.contains_key(&(binary.clone(), test.clone())),
                "ci_pr_tier: profile {name}'s filter leaves out {binary} {test}, which its slow tier does not list with its time"
            );
        }
    }
}

#[test]
fn ci_pr_tier_slow_tier_is_named_with_its_time() {
    check_slow_tier(&read(".config/nextest.toml"));
}

negative_control!(
    ci_pr_tier_slow_tier_is_named_with_its_time,
    "A slow-tier test the pr profile leaves out with no time in its table fails the check.",
    expected = "which its slow tier does not list with its time",
    check_slow_tier(&edited(
        ".config/nextest.toml",
        "#     689.6 s  xtask::build_kernel build_kernel_runs_in_ci\n",
        "",
    ))
);

/// The physics, validation and render tests the owner moved into the `pr` profile's slow tier, his ruling of 9 Nov 2026,
/// verbatim: "1: move them." (the 34 of PR #457's list, 4,981 s measured), answering R-904's "moving any physics or
/// validation test into the slow tier comes to me first". No other test outside xtask and the in-depth GPU tests enters
/// the slow tier without a ruling.
const APPROVED_SLOW: [(&str, &str); 34] = [
    ("kernel::gallery_sweep", "gallery_sweep_unit_square"),
    (
        "kernel::qa_TASK-M3-03",
        "qa_nsub_bucket_floor_pairs_match_the_specification",
    ),
    (
        "ledger::derived",
        "current_drift_property_matches_the_reference",
    ),
    (
        "render::compaction_ramps",
        "field_either_role_every_form_and_ramp_assembles",
    ),
    ("render::debug_views", "debug_views_render_far_from_ties"),
    ("render::decode_preset", "decode_preset_vs_decode_only"),
    ("render::embed_survives", "embed_survives_the_list"),
    (
        "render::observation_taps",
        "observation_taps_leave_every_bound_buffer_unchanged",
    ),
    (
        "render::observation_taps",
        "observation_taps_leave_the_payload_unchanged",
    ),
    (
        "render::prelude",
        "dbg_helpers_match_the_cpu_mirror_on_the_gpu",
    ),
    ("render::prelude", "prelude_luts_colour_space_maps"),
    (
        "render::presentation_no_hatch",
        "presentation_no_hatch_field_ramps",
    ),
    (
        "render::presentation_no_hatch",
        "presentation_no_hatch_override",
    ),
    ("render::qa_TASK-M1-03", "qa_dbg_helpers_cat"),
    (
        "render::qa_TASK-M1-03",
        "qa_prelude_generated_custom_node_compiles",
    ),
    (
        "render::qa_TASK-M1-08",
        "qa_tool020_each_view_assembles_reading_only_its_field",
    ),
    (
        "render::qa_TASK-M1-15",
        "qa_m1_15_debug_views_bind_the_payload_read_only",
    ),
    ("render::qa_TASK-M2-34", "qa_m2_34_presentation_field_ramps"),
    (
        "render::qa_TASK-M2-34",
        "qa_m2_34_presentation_field_ramps_overridden",
    ),
    ("render::qa_TASK-M7-05", "qa_field_either_role_any_field"),
    ("render::qa_TASK-M7-29", "embed_survives_qa_property"),
    (
        "render::qa_TASK-M7-30",
        "embed_hybrid_rotation_qa_angles_at_1024",
    ),
    (
        "validation::debug_scenes",
        "debug_scenes_render_their_references",
    ),
    (
        "validation::decode_preset_harness",
        "decode_preset_harness_probe",
    ),
    (
        "validation::decode_preset_shape_poles",
        "decode_preset_shape_poles",
    ),
    (
        "validation::expected_message",
        "control_panicking_in_setup_fails_and_one_reaching_its_check_passes",
    ),
    ("validation::golden_scene", "golden_scene_renders_its_twin"),
    ("validation::oklab_roundtrip_gate", "oklab_gate_fixtures"),
    (
        "validation::qa_TASK-M1-13",
        "qa_structural_views_colour_by_the_cpu_struct",
    ),
    (
        "validation::qa_TASK-M1-14",
        "qa_m1_14_each_golden_is_its_render_and_can_fail",
    ),
    (
        "validation::qa_TASK-M1-14",
        "qa_m1_14_each_tile_shows_its_own_sample",
    ),
    ("validation::qa_TASK-M1-19", "qa_quad_fields_all_9_shown"),
    (
        "validation::qa_TASK-M2-25",
        "qa_m225_chart_z_is_the_definition",
    ),
    (
        "validation::qa_TASK-M2-25",
        "qa_m225_decode_off_lattice_and_pole",
    ),
];

/// The `pr` profile's slow tier, less xtask's tests and the in-depth GPU tests (the `gpu-kernel` profile's binaries,
/// which the owner moved off the pull request's path, R-908), is exactly [`APPROVED_SLOW`].
fn check_slow_tier_held(toml: &str) {
    let gpu_kernel = bare_binaries(&profile(toml, "gpu-kernel").filter);
    let table = timed(&profile(toml, "pr").comments);
    let approved: BTreeSet<(String, String)> = APPROVED_SLOW
        .iter()
        .map(|(b, t)| (b.to_string(), t.to_string()))
        .collect();
    let held: BTreeSet<(String, String)> = table
        .keys()
        .filter(|(binary, _)| !binary.starts_with("xtask::") && !gpu_kernel.contains(binary))
        .cloned()
        .collect();
    if let Some((binary, test)) = held.difference(&approved).next() {
        panic!(
            "ci_pr_tier: the pr profile's slow tier leaves out {binary} {test}, a physics or validation test the owner \
             has not ruled off the pull request"
        );
    }
    if let Some((binary, test)) = approved.difference(&held).next() {
        panic!(
            "ci_pr_tier: the pr profile's slow tier lacks {binary} {test}, which the owner moved into it (9 Nov 2026)"
        );
    }
}

#[test]
fn ci_pr_tier_slow_tier_holds_only_the_approved_physics_tests() {
    check_slow_tier_held(&read(".config/nextest.toml"));
}

negative_control!(
    ci_pr_tier_slow_tier_holds_only_the_approved_physics_tests,
    "A kernel test the owner did not rule on, put into the pr profile's slow tier, fails the check.",
    expected = "leaves out kernel::monitor monitor_reports_each_failure, a physics or validation test",
    check_slow_tier_held(&edited(
        ".config/nextest.toml",
        "#     689.6 s  xtask::build_kernel build_kernel_runs_in_ci\n",
        "#     689.6 s  xtask::build_kernel build_kernel_runs_in_ci\n#      61.0 s  kernel::monitor monitor_reports_each_failure\n",
    ))
);

#[test]
fn ci_pr_tier_slow_tier_keeps_the_approved_physics_tests() {
    check_slow_tier_held(&read(".config/nextest.toml"));
}

negative_control!(
    ci_pr_tier_slow_tier_keeps_the_approved_physics_tests,
    "One of the 34 the owner moved, dropped from the pr profile's slow tier, fails the check.",
    expected =
        "lacks kernel::gallery_sweep gallery_sweep_unit_square, which the owner moved into it",
    check_slow_tier_held(&edited(
        ".config/nextest.toml",
        "#      64.7 s  kernel::gallery_sweep gallery_sweep_unit_square\n",
        "",
    ))
);

fn check_split(toml: &str) {
    let pr = bare_binaries(&profile(toml, "pr").filter);
    let process = bare_binaries(&profile(toml, "process").filter);
    let gpu_kernel = bare_binaries(&profile(toml, "gpu-kernel").filter);
    let mut want: BTreeSet<String> = process.clone();
    want.extend(gpu_kernel);
    want.extend(PER_MERGE.iter().map(|s| s.to_string()));
    for b in &pr {
        assert!(
            want.contains(b),
            "ci_pr_tier: the pr profile leaves out {b}, which neither the process profile runs nor the per-merge list names"
        );
    }
    for b in &want {
        assert!(
            pr.contains(b),
            "ci_pr_tier: the pr profile runs {b}, a process, in-depth GPU or per-merge binary"
        );
    }
    assert!(
        !process.is_empty(),
        "ci_pr_tier: the process profile names no binary"
    );
}

#[test]
fn ci_pr_tier_profiles_split_the_tests() {
    check_split(&read(".config/nextest.toml"));
}

negative_control!(
    ci_pr_tier_profiles_split_the_tests,
    "A process binary dropped from the process profile but still left out of pr fails the check.",
    expected = "which neither the process profile runs nor the per-merge list names",
    check_split(&edited(
        ".config/nextest.toml",
        "    (\n        binary_id(=validation::qa_TASK-M0-04_controls)\n",
        "    (\n        binary_id(=validation::qa_TASK-M0-04_nothing)\n",
    ))
);

/// The source file of an integration-test binary `crate::name`.
fn test_source(binary: &str) -> Option<PathBuf> {
    let (krate, name) = binary.split_once("::")?;
    let dir = if krate == "xtask" {
        root().join("xtask")
    } else {
        root().join("crates").join(krate)
    };
    Some(dir.join("tests").join(format!("{name}.rs")))
}

fn check_smoke(toml: &str) {
    let named = named_tests(&profile(toml, "gpu-smoke").filter);
    assert!(
        named.len() >= 10,
        "ci_pr_tier: the gpu-smoke profile names {} tests",
        named.len()
    );
    for (binary, test) in &named {
        let path = test_source(binary)
            .unwrap_or_else(|| panic!("ci_pr_tier: gpu-smoke names {binary}, not a test binary"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "ci_pr_tier: gpu-smoke names {binary}: {}: {e}",
                path.display()
            )
        });
        assert!(
            text.contains(&format!("fn {test}(")),
            "ci_pr_tier: gpu-smoke names {binary} {test}, no test of that binary"
        );
    }
}

#[test]
fn ci_pr_tier_smoke_tests_exist() {
    check_smoke(&read(".config/nextest.toml"));
}

negative_control!(
    ci_pr_tier_smoke_tests_exist,
    "A smoke test renamed out from under the profile fails the check.",
    expected = "no test of that binary",
    check_smoke(&edited(
        ".config/nextest.toml",
        "test(=decode_stage_tanh_range)",
        "test(=decode_stage_tanh_range_renamed)",
    ))
);

/// The `gpu-smoke` profile's selection of the mock noise's lattice tests in gui's library (REQ-GUI-412).
const LATTICE: &str = "(binary_id(=gui) & test(/^tests::mock_lattice::lattice_/))";

/// The step both GPU smoke sets run the lattice tests by.
const LATTICE_STEP: &str = "        run: cargo nextest run -p gui --lib --no-capture";

/// REQ-GUI-412: "on every backend the regression test runs on: CI's lavapipe and Metal block". The `gpu-smoke` profile
/// selects gui's lattice tests, and each smoke set, a required check, runs them under that profile with nothing letting
/// a failure through.
fn check_lattice_smoke(toml: &str, workflow: &str) {
    assert!(
        profile(toml, "gpu-smoke").filter.contains(LATTICE),
        "ci_pr_tier: the gpu-smoke profile does not select gui's lattice tests"
    );
    let jobs = jobs(workflow);
    for check in ["gpu-lavapipe", "gpu-metal"] {
        let (id, lines) = jobs
            .iter()
            .find(|(id, l)| job_name(id, l) == check)
            .unwrap_or_else(|| panic!("ci_pr_tier: no job bears {check}"));
        let at = lines
            .iter()
            .position(|l| l == LATTICE_STEP)
            .unwrap_or_else(|| {
                panic!("ci_pr_tier: {check} ({id}) does not run gui's lattice tests")
            });
        let start = lines[..at]
            .iter()
            .rposition(|l| l.starts_with("      - "))
            .unwrap_or_else(|| panic!("ci_pr_tier: {check} ({id})'s lattice step has no start"));
        let step = &lines[start..=at];
        assert!(
            step.iter()
                .any(|l| l.trim() == "NEXTEST_PROFILE: gpu-smoke"),
            "ci_pr_tier: {check} ({id}) runs gui's lattice tests outside the gpu-smoke profile"
        );
        assert!(
            !step
                .iter()
                .any(|l| l.trim().starts_with("continue-on-error")),
            "ci_pr_tier: {check} ({id}) lets a lattice failure through"
        );
    }
}

#[test]
fn ci_pr_tier_smoke_sets_run_the_lattice_tests() {
    check_lattice_smoke(
        &read(".config/nextest.toml"),
        &read(".github/workflows/ci.yml"),
    );
}

negative_control!(
    ci_pr_tier_smoke_sets_run_the_lattice_tests,
    "The Metal smoke set without its lattice step fails the check.",
    expected = "gpu-metal (gpu-metal-smoke) does not run gui's lattice tests",
    check_lattice_smoke(
        &read(".config/nextest.toml"),
        &without_last(
            &read(".github/workflows/ci.yml"),
            &format!("{LATTICE_STEP}\n")
        ),
    )
);

/// `text` without the last `cut` in it, the Metal smoke set's step coming after lavapipe's.
#[cfg(feature = "controls")]
fn without_last(text: &str, cut: &str) -> String {
    let at = text
        .rfind(cut)
        .unwrap_or_else(|| panic!("ci_pr_tier: no {cut:?} to cut"));
    format!("{}{}", &text[..at], &text[at + cut.len()..])
}

/// Each job of `ci.yml`, as (id, its lines), from its id (two spaces in, under `jobs:`) to the next job's.
fn jobs(workflow: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        if !line.starts_with(' ') && !line.is_empty() {
            in_jobs = line.trim_end() == "jobs:";
        } else if in_jobs && line.starts_with("  ") && !line[2..].starts_with([' ', '#']) {
            out.push((line.trim().trim_end_matches(':').to_owned(), Vec::new()));
        } else if let Some((_, lines)) = out.last_mut().filter(|_| in_jobs) {
            lines.push(line.to_owned());
        }
    }
    out
}

/// A job's own `name:` (four spaces in), or its id.
fn job_name(id: &str, lines: &[String]) -> String {
    lines
        .iter()
        .find_map(|l| l.strip_prefix("    name: "))
        .map(str::to_owned)
        .unwrap_or_else(|| id.to_owned())
}

fn check_required(workflow: &str) {
    let jobs = jobs(workflow);
    for check in REQUIRED {
        let bearing: Vec<&(String, Vec<String>)> = jobs
            .iter()
            .filter(|(id, l)| job_name(id, l) == check)
            .collect();
        assert_eq!(
            bearing.len(),
            1,
            "ci_pr_tier: {} jobs of ci.yml bear {check}",
            bearing.len()
        );
        let (id, lines) = bearing[0];
        let has = |want: &str| lines.iter().any(|l| l == want);
        // `always()` alone for the GPU smoke sets, which run on a push too, to save their caches on `main`.
        assert!(
            has("    if: always() && github.event_name == 'pull_request'")
                || has("    if: always()"),
            "ci_pr_tier: {check} ({id}) does not run on every pull request"
        );
        assert!(
            has("    needs: changes"),
            "ci_pr_tier: {check} ({id}) does not need changes"
        );
        let first = lines
            .iter()
            .position(|l| l == "    steps:")
            .and_then(|at| lines.get(at + 1..at + 6))
            .unwrap_or_else(|| panic!("ci_pr_tier: {check} ({id}) has no steps"));
        assert!(
            first[0] == "      - name: The classification ran"
                && first
                    .iter()
                    .any(|l| l.trim() == "CHANGES: ${{ needs.changes.result }}")
                && first
                    .iter()
                    .any(|l| l.contains("[ \"$CHANGES\" = success ] ||")),
            "ci_pr_tier: {check} ({id}) does not fail first when changes did not succeed"
        );
    }
    for line in workflow.lines() {
        let code = line.trim();
        assert!(
            code.starts_with('#') || !(code.contains("sleep ") || code.contains("gh run watch")),
            "ci_pr_tier: ci.yml waits or polls: {code}"
        );
    }
}

#[test]
fn ci_pr_tier_required_checks_skip_inside_the_job() {
    check_required(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_pr_tier_required_checks_skip_inside_the_job,
    "A required check's job skipped at job level on the GPU verdict fails the check.",
    expected = "gpu-lavapipe (gpu-lavapipe-smoke) does not run on every pull request",
    check_required(&edited(
        ".github/workflows/ci.yml",
        "    name: gpu-lavapipe\n    needs: changes\n    if: always()\n",
        "    name: gpu-lavapipe\n    needs: changes\n    if: needs.changes.outputs.gpu == 'true'\n",
    ))
);
