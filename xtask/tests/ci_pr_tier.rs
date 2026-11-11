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
//!   runs on every pull request whose run is not cancelled (`if: ${{ !cancelled() && github.event_name ==
//!   'pull_request' }}`, or `if: ${{ !cancelled() }}` for the GPU smoke sets, which run on a push too), needs `changes`,
//!   and fails at
//!   its first step unless `changes` succeeded (R-656: the skips are inside the job), and no step of the workflow
//!   sleeps or polls.
//! - `ci_pr_tier_ci_runs_as_three_shards`, `ci_pr_tier_ci_fails_unless_every_shard_passed` and
//!   `ci_pr_tier_reader_binaries_hold_tests`: the required check `ci` is a verdict over the product tests' 3 shards (the
//!   owner, 11 Nov 2026: "split the ci job as planned"): each nextest step runs its `hash:` slice of 3, the doctests run
//!   in shard 1, `ci` passes only when the shards' matrix succeeded, and each reader binary a partitioned step names
//!   holds a test.
//! - `ci_pr_tier_crates_are_fetched_first`: `clippy`, `build` and `process` fetch the crates first, up to three tries
//!   over HTTP/1.1, then run offline, each keeping the registry cache under a key naming its own job (R-285; #481's
//!   `clippy` failed at crates.io, curl 56, before anything compiled).
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
        // `!cancelled()` alone for the GPU smoke sets, which run on a push too, to save their caches on `main`.
        assert!(
            has("    if: ${{ !cancelled() && github.event_name == 'pull_request' }}")
                || has("    if: ${{ !cancelled() }}"),
            "ci_pr_tier: {check} ({id}) does not run on every pull request"
        );
        // `needs: changes`, or a list naming it: `ci`'s verdict needs the shards beside it.
        assert!(
            lines.iter().any(
                |l| l.strip_prefix("    needs: ").is_some_and(|n| n == "changes"
                    || n.trim_matches(['[', ']'])
                        .split(',')
                        .any(|x| x.trim() == "changes"))
            ),
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
        "    name: gpu-lavapipe\n    needs: changes\n    if: ${{ !cancelled() }}\n",
        "    name: gpu-lavapipe\n    needs: changes\n    if: needs.changes.outputs.gpu == 'true'\n",
    ))
);

// ---- A cancelled run stops, every job has a time limit, apt is bounded (the CI diagnosis of 10 Nov 2026) -------------

/// The job-level `if:` of a job (four spaces in), or nothing.
fn job_if(lines: &[String]) -> &str {
    lines
        .iter()
        .find_map(|l| l.strip_prefix("    if: "))
        .unwrap_or("")
}

/// Whether the job `id` is of the per-merge tier alone: its `if:` leaves out every pull request, or, its `if:` holding
/// no status function, a job it needs (`needs: a` or `needs: [a, b]`) is of that tier alone, which skips it.
fn push_only(jobs: &[(String, Vec<String>)], id: &str) -> bool {
    let Some((_, lines)) = jobs.iter().find(|(job, _)| job == id) else {
        return false;
    };
    let cond = job_if(lines);
    if cond.contains("github.event_name != 'pull_request'") {
        return true;
    }
    let needs = lines
        .iter()
        .find_map(|l| l.strip_prefix("    needs: "))
        .unwrap_or("");
    !(cond.contains("always()") || cond.contains("!cancelled()"))
        && needs
            .trim_matches(['[', ']'])
            .split(',')
            .any(|need| push_only(jobs, need.trim()))
}

/// No job a pull request runs holds `always()` at job level, under which a cancelled run's jobs run to their end and
/// its jobs not yet started still start (runs 42753619619, 42753864312, 42710215419); each job bearing a required check
/// holds `!cancelled()` in its place, so it still runs whatever `changes` ended in.
fn check_cancelled_runs_stop(workflow: &str) {
    let jobs = jobs(workflow);
    for (id, lines) in &jobs {
        assert!(
            push_only(&jobs, id) || !job_if(lines).contains("always()"),
            "ci_pr_tier: {} ({id}) runs on in a cancelled run: `if: {}`",
            job_name(id, lines),
            job_if(lines)
        );
    }
    for check in REQUIRED {
        let (id, lines) = jobs
            .iter()
            .find(|(id, l)| job_name(id, l) == check)
            .unwrap_or_else(|| panic!("ci_pr_tier: no job of ci.yml bears {check}"));
        assert!(
            job_if(lines).starts_with("${{ !cancelled() "),
            "ci_pr_tier: {check} ({id}) does not run whatever `changes` ended in, short of a cancelled run: `if: {}`",
            job_if(lines)
        );
    }
}

#[test]
fn ci_pr_tier_cancelled_runs_stop() {
    check_cancelled_runs_stop(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_pr_tier_cancelled_runs_stop,
    "The product tests' verdict under `always()` again, which a cancelled run cannot stop.",
    expected = "ci (tests-verdict) runs on in a cancelled run: `if: always() && github.event_name == 'pull_request'`",
    check_cancelled_runs_stop(&edited(
        ".github/workflows/ci.yml",
        "    name: ci\n    needs: [changes, tests]\n    if: ${{ !cancelled() && github.event_name == 'pull_request' }}\n",
        "    name: ci\n    needs: [changes, tests]\n    if: always() && github.event_name == 'pull_request'\n",
    ))
);

mod a_required_job_without_a_status_function {
    use super::*;
    negative_control!(
        ci_pr_tier_cancelled_runs_stop,
        "The Metal smoke set with no status function, which a failed classification skips.",
        expected = "gpu-metal (gpu-metal-smoke) does not run whatever `changes` ended in",
        check_cancelled_runs_stop(&edited(
            ".github/workflows/ci.yml",
            "    name: gpu-metal\n    needs: changes\n    if: ${{ !cancelled() }}\n",
            "    name: gpu-metal\n    needs: changes\n    if: github.event_name != 'workflow_dispatch'\n",
        ))
    );
}

/// The PR tier's jobs, every job of `ci.yml` a pull request can run, each with its `timeout-minutes`: the larger of
/// 1.5 times the job's longest passing run (60 pull-request and 40 push runs before 10 Nov 2026), rounded up to 5
/// minutes, and the diagnosis's figure. `ci.yml`'s header gives each job's longest run.
const LIMITS: [(&str, u32); 10] = [
    ("changes", 5),
    ("fmt", 5),
    ("clippy", 10),
    ("build", 25),
    ("tests", 90),
    ("tests-verdict", 5),
    ("process", 60),
    ("gpu-lavapipe-smoke", 25),
    ("gpu-metal-smoke", 20),
    ("web-skeleton", 10),
];

/// Every job of `ci.yml` that a pull request can run has its own `timeout-minutes`, the one [`LIMITS`] gives, so none
/// runs to GitHub's 360 minutes (runs 42760984954 and 42759947849 did), and a job added to the PR tier is given one.
fn check_time_limits(workflow: &str) {
    let mut seen = BTreeSet::new();
    let jobs = jobs(workflow);
    for (id, lines) in &jobs {
        if push_only(&jobs, id) {
            continue;
        }
        let limit = lines
            .iter()
            .find_map(|l| l.strip_prefix("    timeout-minutes: "))
            .unwrap_or_else(|| panic!("ci_pr_tier: the PR tier's job {id} has no timeout-minutes"));
        let want = LIMITS
            .iter()
            .find(|(job, _)| *job == id.as_str())
            .unwrap_or_else(|| panic!("ci_pr_tier: the PR tier's job {id} has no measured limit"))
            .1;
        assert_eq!(
            limit.parse::<u32>().ok(),
            Some(want),
            "ci_pr_tier: the PR tier's job {id} is limited to {limit} minutes, not its measured {want}"
        );
        seen.insert(id.as_str());
    }
    for (job, _) in LIMITS {
        assert!(
            seen.contains(job),
            "ci_pr_tier: {job}, which has a measured limit, is no job of the PR tier"
        );
    }
}

#[test]
fn ci_pr_tier_every_job_has_a_time_limit() {
    check_time_limits(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_pr_tier_every_job_has_a_time_limit,
    "The product tests with no limit of their own, so GitHub's 360 minutes.",
    expected = "the PR tier's job tests has no timeout-minutes",
    check_time_limits(&edited(
        ".github/workflows/ci.yml",
        "    runs-on: ubuntu-24.04\n    timeout-minutes: 90\n",
        "    runs-on: ubuntu-24.04\n",
    ))
);

mod a_limit_off_its_measured_value {
    use super::*;
    negative_control!(
        ci_pr_tier_every_job_has_a_time_limit,
        "The product tests limited to a minute more than their measured limit.",
        expected = "the PR tier's job tests is limited to 91 minutes, not its measured 90",
        check_time_limits(&edited(
            ".github/workflows/ci.yml",
            "    runs-on: ubuntu-24.04\n    timeout-minutes: 90\n",
            "    runs-on: ubuntu-24.04\n    timeout-minutes: 91\n",
        ))
    );
}

mod a_pull_request_job_without_a_measured_limit {
    use super::*;
    negative_control!(
        ci_pr_tier_every_job_has_a_time_limit,
        "A job added to the PR tier with a limit nothing measured.",
        expected = "the PR tier's job qa_new has no measured limit",
        check_time_limits(&format!(
            "{}  qa_new:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 5\n    steps:\n      - run: true\n",
            read(".github/workflows/ci.yml")
        ))
    );
}

/// The shell function every apt step calls `apt-get` through: each call under `timeout`, the seconds its first argument
/// gives and a kill 10 s later, tried three times.
const BOUNDED: &str = "bounded() { limit=$1; shift; for attempt in 1 2 3; do sudo timeout -k 10 \"$limit\" \"$@\" && \
                       return; echo \"attempt $attempt of 3 failed or ran past $limit s, $*\"; done; return 1; }";

/// An apt step's update, through [`BOUNDED`] at 90 s: 1.5 times the longest passing update of the 654 Mesa steps
/// measured, each under 60 s.
const BOUNDED_UPDATE: &str = "bounded 90 apt-get update";

/// The start of an apt step's install, through [`BOUNDED`] at 420 s: 1.2 times the longest passing install of those
/// steps, 348 s (job 129595574396), and what [`APT_STEP_LIMIT`] leaves after a slow update. The packages follow it.
const BOUNDED_INSTALL: &str = "bounded 420 apt-get install -y ";

/// The end of an apt step's install: it waits up to 60 s for dpkg's lock, which a dpkg left by a call cut at its limit
/// holds until it finishes (run 42938963594).
const LOCK_WAIT: &str = " -oDPkg::Lock::Timeout=60";

/// The most minutes a step that installs from apt may take, the diagnosis's "Mesa step max ≤ 8 min": 1.36 times the
/// longest passing Mesa step, 354 s.
const APT_STEP_LIMIT: u32 = 8;

/// Each step of a workflow that names `apt-get` outside a comment, as (its lines to its `run:`, its script's lines).
fn apt_steps(workflow: &str) -> Vec<(Vec<String>, Vec<String>)> {
    let mut steps: Vec<Vec<&str>> = Vec::new();
    for line in workflow.lines() {
        if line.starts_with("      - ") {
            steps.push(vec![line]);
        } else if line.starts_with("        ") || line.trim().is_empty() {
            if let Some(step) = steps.last_mut() {
                step.push(line);
            }
        } else {
            // A job's or the workflow's own line, or a comment between steps: the step before it has ended.
            steps.push(Vec::new());
        }
    }
    steps
        .into_iter()
        .filter(|step| {
            step.iter()
                .any(|l| !l.trim_start().starts_with('#') && l.contains("apt-get"))
        })
        .map(|step| {
            let run = step
                .iter()
                .position(|l| l.trim_start().trim_start_matches("- ").starts_with("run:"))
                .unwrap_or(step.len());
            let own = |l: &&str| l.trim().to_owned();
            (
                step[..=run.min(step.len() - 1)].iter().map(own).collect(),
                step[(run + 1).min(step.len())..]
                    .iter()
                    .filter(|l| !l.trim().is_empty())
                    .map(own)
                    .collect(),
            )
        })
        .collect()
}

/// Whether a word after `apt-get install -y` is a package's name or an option, and no more of the shell.
fn package_or_option(word: &str) -> bool {
    word.chars()
        .all(|c| c.is_ascii_alphanumeric() || "-.+".contains(c))
}

/// Whether `rest`, what follows [`BOUNDED_INSTALL`], is one package or more, options among them, and then
/// [`LOCK_WAIT`].
fn packages_then_lock_wait(rest: &str) -> bool {
    rest.strip_suffix(LOCK_WAIT).is_some_and(|packages| {
        !packages.trim().is_empty() && packages.split_whitespace().all(package_or_option)
    })
}

/// In every workflow, a step that installs from apt has a limit of its own of at most [`APT_STEP_LIMIT`] minutes and
/// calls `apt-get` only through [`BOUNDED`], first to update and then to install: `sudo apt-get update` alone stalled
/// three jobs for six hours (jobs 129651506462, 129651506556, 129653679613).
fn check_apt_bounded(workflows: &[(String, String)]) {
    let mut found = 0;
    for (file, text) in workflows {
        for (head, script) in apt_steps(text) {
            let name = head[0].trim_start_matches("- name: ").to_owned();
            found += 1;
            let limit = head
                .iter()
                .find_map(|l| l.strip_prefix("timeout-minutes: "))
                .and_then(|m| m.parse::<u32>().ok());
            assert!(
                limit.is_some_and(|m| (1..=APT_STEP_LIMIT).contains(&m)),
                "ci_pr_tier: {file}'s step `{name}` has no limit of its own of 1 to {APT_STEP_LIMIT} minutes: {limit:?}"
            );
            // One line, the whole `run:`, or the first three lines of a `run: |` block.
            let one_line = head
                .last()
                .and_then(|l| l.strip_prefix("run: "))
                .and_then(|l| l.strip_prefix(BOUNDED))
                .and_then(|l| l.strip_prefix("; "))
                .and_then(|l| l.strip_prefix(BOUNDED_UPDATE))
                .and_then(|l| l.strip_prefix(" && "))
                .and_then(|l| l.strip_prefix(BOUNDED_INSTALL))
                .is_some_and(packages_then_lock_wait)
                && script.iter().all(|l| !l.contains("apt-get"));
            let block = head.last().is_some_and(|l| l == "run: |")
                && script.len() >= 3
                && script[0] == BOUNDED
                && script[1] == BOUNDED_UPDATE
                && script[2]
                    .strip_prefix(BOUNDED_INSTALL)
                    .is_some_and(packages_then_lock_wait)
                && script[3..].iter().all(|l| !l.contains("apt-get"));
            assert!(
                one_line || block,
                "ci_pr_tier: {file}'s step `{name}` calls apt-get outside `bounded`, update then install: {:?} {script:?}",
                head.last()
            );
        }
    }
    assert!(
        found > 0,
        "ci_pr_tier: no step of any workflow installs from apt"
    );
}

/// Every workflow, as (file name, text).
fn all_workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("ci_pr_tier: {}: {e}", dir.display()))
        .map(|entry| entry.expect("a workflow").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name.clone(), read(&format!(".github/workflows/{name}")))
        })
        .collect();
    out.sort();
    out
}

/// Every workflow, `file`'s first `from` replaced by `to`.
#[cfg(feature = "controls")]
fn all_workflows_edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut workflows = all_workflows();
    let at = workflows
        .iter()
        .position(|(f, _)| f == file)
        .unwrap_or_else(|| panic!("ci_pr_tier: no {file}"));
    workflows[at].1 = edited(&format!(".github/workflows/{file}"), from, to);
    workflows
}

#[test]
fn ci_pr_tier_apt_installs_are_bounded() {
    check_apt_bounded(&all_workflows());
}

negative_control!(
    ci_pr_tier_apt_installs_are_bounded,
    "The product tests' Mesa install as it was, two bare apt-get calls.",
    expected = "ci.yml's step `Install Mesa (lavapipe)` calls apt-get outside `bounded`",
    check_apt_bounded(&all_workflows_edited(
        "ci.yml",
        &format!("        run: {BOUNDED}; {BOUNDED_UPDATE} && {BOUNDED_INSTALL}mesa-vulkan-drivers{LOCK_WAIT}\n"),
        "        run: sudo apt-get update && sudo apt-get install -y mesa-vulkan-drivers\n",
    ))
);

mod an_apt_step_past_its_limit {
    use super::*;
    negative_control!(
        ci_pr_tier_apt_installs_are_bounded,
        "The pull request's screenshot job's Mesa install limited to a minute more than the most allowed.",
        expected = "screenshot.yml's step `Install Mesa (lavapipe)` has no limit of its own of 1 to 8 minutes: Some(9)",
        check_apt_bounded(&all_workflows_edited(
            "screenshot.yml",
            "        timeout-minutes: 8\n        run: bounded() {",
            "        timeout-minutes: 9\n        run: bounded() {",
        ))
    );
}

mod an_apt_step_without_a_limit {
    use super::*;
    negative_control!(
        ci_pr_tier_apt_installs_are_bounded,
        "The progress banner's librsvg install with no limit of its own.",
        expected = "progress.yml's step `Install rsvg-convert` has no limit of its own of 1 to 8 minutes: None",
        check_apt_bounded(&all_workflows_edited(
            "progress.yml",
            "        timeout-minutes: 8\n        run: |\n",
            "        run: |\n",
        ))
    );
}

mod an_apt_call_after_the_install {
    use super::*;
    negative_control!(
        ci_pr_tier_apt_installs_are_bounded,
        "A gate job's Mesa install followed by a bare apt-get call.",
        expected = "gate.yml's step `Install Mesa (lavapipe)` calls apt-get outside `bounded`",
        check_apt_bounded(&all_workflows_edited(
            "gate.yml",
            "mesa-vulkan-drivers -oDPkg::Lock::Timeout=60\n",
            "mesa-vulkan-drivers -oDPkg::Lock::Timeout=60 && sudo apt-get clean\n",
        ))
    );
}

mod an_install_cut_before_the_longest_passing_one {
    use super::*;
    negative_control!(
        ci_pr_tier_apt_installs_are_bounded,
        "A nightly job's Mesa install limited to 120 s, which 20 of the 654 passing steps measured ran past.",
        expected = "nightly.yml's step `Install Mesa (lavapipe)` calls apt-get outside `bounded`",
        check_apt_bounded(&all_workflows_edited(
            "nightly.yml",
            "bounded 420 apt-get install",
            "bounded 120 apt-get install",
        ))
    );
}

mod an_update_under_the_install_s_limit {
    use super::*;
    negative_control!(
        ci_pr_tier_apt_installs_are_bounded,
        "The soak's Mesa step with its update given the install's limit, a hung one then the whole of the step's 8 minutes.",
        expected = "stand-in-soak.yml's step `Install Mesa (lavapipe)` calls apt-get outside `bounded`",
        check_apt_bounded(&all_workflows_edited(
            "stand-in-soak.yml",
            "bounded 90 apt-get update",
            "bounded 420 apt-get update",
        ))
    );
}

mod an_install_that_does_not_wait_for_the_lock {
    use super::*;
    negative_control!(
        ci_pr_tier_apt_installs_are_bounded,
        "The progress banner's librsvg install with no wait for dpkg's lock.",
        expected = "progress.yml's step `Install rsvg-convert` calls apt-get outside `bounded`",
        check_apt_bounded(&all_workflows_edited(
            "progress.yml",
            "librsvg2-bin -oDPkg::Lock::Timeout=60\n",
            "librsvg2-bin\n",
        ))
    );
}

/// `script` run as a step is (`bash -eo pipefail`), `sudo` stood in by a function that records its arguments and
/// fails on the calls `fail` numbers, from 1: whether the step passed, and each call.
fn apt_step_run(script: &str, fail: &[u32]) -> (bool, Vec<String>) {
    let fail: Vec<String> = fail.iter().map(u32::to_string).collect();
    let stand_in = "calls=0\nsudo() { calls=$((calls + 1)); echo \"sudo $*\"; \
                    case \" $FAIL \" in *\" $calls \"*) return 1 ;; esac; }\n";
    let out = std::process::Command::new("bash")
        .args(["--noprofile", "--norc", "-eo", "pipefail", "-c"])
        .arg(format!("{stand_in}{script}"))
        .env("FAIL", fail.join(" "))
        .output()
        .expect("bash runs");
    let calls = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.strip_prefix("sudo "))
        .map(str::to_owned)
        .collect();
    (out.status.success(), calls)
}

/// An apt step's script, installing `packages`: each `apt-get` call runs under `timeout -k 10`, the update at 90 s and
/// the install at 420 s; a call that fails is tried again, three times in all; a third failure fails the step there,
/// the install not tried after a failed update.
fn check_apt_retries(script: &str, packages: &str) {
    let update = "timeout -k 10 90 apt-get update";
    let install =
        format!("timeout -k 10 420 apt-get install -y {packages} -oDPkg::Lock::Timeout=60");
    let install = install.as_str();
    assert_eq!(
        apt_step_run(script, &[]),
        (true, vec![update.to_owned(), install.to_owned()]),
        "ci_pr_tier: the Mesa step, every call passing, is not one bounded update and one bounded install"
    );
    assert_eq!(
        apt_step_run(script, &[1, 2]),
        (
            true,
            [update, update, update, install]
                .map(str::to_owned)
                .to_vec()
        ),
        "ci_pr_tier: the Mesa step does not pass on its update's third attempt"
    );
    assert_eq!(
        apt_step_run(script, &[1, 2, 3]),
        (false, [update, update, update].map(str::to_owned).to_vec()),
        "ci_pr_tier: the Mesa step does not fail at its update's third failure"
    );
    assert_eq!(
        apt_step_run(script, &[2, 3, 4]),
        (
            false,
            [update, install, install, install]
                .map(str::to_owned)
                .to_vec()
        ),
        "ci_pr_tier: the Mesa step does not fail at its install's third failure"
    );
}

/// The script of the product tests' Mesa step, as `ci.yml` has it: its one-line `run:`.
fn mesa_script() -> String {
    let ci = read(".github/workflows/ci.yml");
    let (head, _) = apt_steps(&ci)
        .into_iter()
        .next()
        .expect("ci_pr_tier: ci.yml has no apt step");
    let run = head
        .last()
        .and_then(|l| l.strip_prefix("run: "))
        .expect("ci_pr_tier: ci.yml's Mesa step is not a one-line `run:`");
    format!("{run}\n")
}

/// The apt calls of the progress banner's librsvg step, as `progress.yml` has them: the first three lines of its
/// `run: |` block, where the line after them would hide a failed one from a script that didn't stop at it.
fn rsvg_script() -> String {
    let progress = read(".github/workflows/progress.yml");
    let (_, script) = apt_steps(&progress)
        .into_iter()
        .next()
        .expect("ci_pr_tier: progress.yml has no apt step");
    format!("{}\necho after\n", script[..3].join("\n"))
}

const MESA: &str = "mesa-vulkan-drivers";

#[test]
fn ci_pr_tier_apt_calls_retry_under_a_timeout() {
    check_apt_retries(&mesa_script(), MESA);
    check_apt_retries(&rsvg_script(), "--no-install-recommends librsvg2-bin");
}

negative_control!(
    ci_pr_tier_apt_calls_retry_under_a_timeout,
    "The Mesa step trying each call twice.",
    expected = "the Mesa step does not pass on its update's third attempt",
    check_apt_retries(&mesa_script().replace("in 1 2 3;", "in 1 2;"), MESA)
);

mod an_apt_call_without_its_timeout {
    use super::*;
    negative_control!(
        ci_pr_tier_apt_calls_retry_under_a_timeout,
        "The Mesa step's calls with no timeout, as the ones that stalled six hours.",
        expected =
            "the Mesa step, every call passing, is not one bounded update and one bounded install",
        check_apt_retries(
            &mesa_script().replace("sudo timeout -k 10 \"$limit\" ", "sudo "),
            MESA
        )
    );
}

mod an_install_after_a_failed_update {
    use super::*;
    negative_control!(
        ci_pr_tier_apt_calls_retry_under_a_timeout,
        "The Mesa step going on to the install whatever the update ended in.",
        expected = "the Mesa step does not fail at its update's third failure",
        check_apt_retries(
            &mesa_script().replace(
                " && bounded 420 apt-get install",
                " || true; bounded 420 apt-get install"
            ),
            MESA
        )
    );
}

/// `script` run as a step is, with `apt-get` stood in by a program that records its arguments and, on the calls `hang`
/// numbers, from 1, sleeps 5 s before it passes; `sudo` only runs its arguments, and `timeout` is the machine's, or,
/// where it has none (macOS), a function that ends the command the same way, TERM at the limit and KILL after `-k`'s
/// seconds, and returns 124. Whether the step passed, each call, and whether any stand-in was still running at the end.
fn apt_step_run_hanging(case: &str, script: &str, hang: &[u32]) -> (bool, Vec<String>, bool) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ci_pr_tier_{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let apt_get = dir.join("apt-get");
    std::fs::write(
        &apt_get,
        "#!/bin/bash\necho \"$*\" >> \"$CALLS\"\necho $$ >> \"$CALLS.pids\"\n\
         case \" $HANG \" in *\" $(($(wc -l < \"$CALLS\"))) \"*) exec sleep 5 ;; esac\n",
    )
    .expect("the stand-in is written");
    let stand_ins = "chmod 755 \"$APT_GET\"\nsudo() { \"$@\"; }\n\
        command -v timeout > /dev/null || timeout() { local kill_after=$2 limit=$3 status=0; shift 3; \"$@\" & \
        local pid=$!; ( sleep \"$limit\"; kill -TERM \"$pid\"; sleep \"$kill_after\"; kill -KILL \"$pid\" ) \
        > /dev/null 2>&1 & local watch=$!; wait \"$pid\" || status=$?; kill \"$watch\" 2> /dev/null || true; \
        [ \"$status\" -lt 128 ] || status=124; return \"$status\"; }\n";
    let calls = dir.join("calls");
    let path = std::env::var("PATH").expect("a PATH");
    let hang: Vec<String> = hang.iter().map(u32::to_string).collect();
    let out = std::process::Command::new("bash")
        .args(["--noprofile", "--norc", "-eo", "pipefail", "-c"])
        .arg(format!("{stand_ins}{script}"))
        .env("PATH", format!("{}:{path}", dir.display()))
        .env("APT_GET", &apt_get)
        .env("CALLS", &calls)
        .env("HANG", hang.join(" "))
        .output()
        .expect("bash runs");
    let lines = |file: PathBuf| -> Vec<String> {
        std::fs::read_to_string(file)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    };
    let running = lines(dir.join("calls.pids")).iter().any(|pid| {
        std::process::Command::new("kill")
            .args(["-0", pid])
            .stderr(std::process::Stdio::null())
            .status()
            .expect("kill runs")
            .success()
    });
    (out.status.success(), lines(calls), running)
}

/// `script`, an apt step's, with each call's limit a second and its kill a second later, so that a stand-in that hangs
/// for 5 s is past it.
fn at_one_second(script: &str) -> String {
    for limit in ["bounded 90 ", "bounded 420 "] {
        assert!(
            script.contains(limit),
            "ci_pr_tier: the apt step has no `{limit}`: {script}"
        );
    }
    script
        .replace("-k 10 ", "-k 1 ")
        .replace("bounded 90 ", "bounded 1 ")
        .replace("bounded 420 ", "bounded 1 ")
}

/// An apt step's script, its limits at a second, with a call that really hangs: the call is ended at its limit and
/// tried again, the update or the install, and the step passes; three hung updates fail the step, the install not
/// tried; and no call is left running. `apt-get update` hung so for six hours in jobs 129651506462, 129651506556 and
/// 129653679613.
fn check_apt_cuts(case: &str, script: &str) {
    let script = at_one_second(script);
    let update = "update";
    let install = "install -y mesa-vulkan-drivers -oDPkg::Lock::Timeout=60";
    assert_eq!(
        apt_step_run_hanging(&format!("{case}_update"), &script, &[1]),
        (
            true,
            [update, update, install].map(str::to_owned).to_vec(),
            false
        ),
        "ci_pr_tier: a hung update is not ended at its limit and tried again"
    );
    assert_eq!(
        apt_step_run_hanging(&format!("{case}_install"), &script, &[2]),
        (
            true,
            [update, install, install].map(str::to_owned).to_vec(),
            false
        ),
        "ci_pr_tier: a hung install is not ended at its limit and tried again"
    );
    assert_eq!(
        apt_step_run_hanging(&format!("{case}_thrice"), &script, &[1, 2, 3]),
        (
            false,
            [update, update, update].map(str::to_owned).to_vec(),
            false
        ),
        "ci_pr_tier: three hung updates do not fail the step there, each ended at its limit"
    );
}

#[test]
fn ci_pr_tier_apt_call_that_hangs_is_cut_and_tried_again() {
    check_apt_cuts("cuts", &mesa_script());
}

negative_control!(
    ci_pr_tier_apt_call_that_hangs_is_cut_and_tried_again,
    "The Mesa step's calls under no timeout, the hung update running to its own end.",
    expected = "a hung update is not ended at its limit and tried again",
    check_apt_cuts(
        "cuts_control_no_timeout",
        &mesa_script().replace("sudo timeout -k 10 \"$limit\" ", "sudo ")
    )
);

mod a_limit_the_function_does_not_pass_on {
    use super::*;
    negative_control!(
        ci_pr_tier_apt_call_that_hangs_is_cut_and_tried_again,
        "The Mesa step's function ending every call at 120 s whatever limit it is given.",
        expected = "a hung update is not ended at its limit and tried again",
        check_apt_cuts(
            "cuts_control_fixed_limit",
            &mesa_script().replace("timeout -k 10 \"$limit\" ", "timeout -k 10 120 ")
        )
    );
}

// ---- `ci` over 3 shards (the owner, 11 Nov 2026: "split the ci job as planned") -------------------------------------

/// The number of shards the product tests run as.
const SHARDS: usize = 3;

/// A job's steps, each as its lines (six spaces in, `- ` first).
fn steps_of(lines: &[String]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut in_steps = false;
    for line in lines {
        if line == "    steps:" {
            in_steps = true;
        } else if in_steps && line.starts_with("      - ") {
            out.push(vec![line.clone()]);
        } else if in_steps && line.starts_with("        ") {
            if let Some(step) = out.last_mut() {
                step.push(line.clone());
            }
        } else if !line.trim().is_empty() && !line.trim_start().starts_with('#') {
            in_steps = in_steps && line.starts_with("      ");
        }
    }
    out
}

/// A step's value of `key` (its own, eight spaces in, or on its `- ` line).
fn step_key(step: &[String], key: &str) -> Option<String> {
    step.iter().find_map(|l| {
        l.trim_start()
            .trim_start_matches("- ")
            .strip_prefix(&format!("{key}: "))
            .map(str::to_owned)
    })
}

/// `ci` is a verdict over the product tests' shards: the job `tests` is a matrix of exactly [`SHARDS`] shards, each
/// nextest step in it runs its `--partition hash:${{ matrix.shard }}/<SHARDS>`, so together they run every test once,
/// the doctests, which have no partition, run in shard 1 alone, every other step of the job runs in every shard, and the
/// job bearing `ci` needs `tests` and reads its result.
fn check_shards(workflow: &str) {
    let jobs = jobs(workflow);
    let (_, tests) = jobs
        .iter()
        .find(|(id, _)| id == "tests")
        .expect("ci_pr_tier: no job tests");
    let matrix: Vec<String> = (1..=SHARDS).map(|k| k.to_string()).collect();
    assert!(
        tests
            .iter()
            .any(|l| l == &format!("        shard: [{}]", matrix.join(", "))),
        "ci_pr_tier: the product tests are not a matrix of {SHARDS} shards"
    );
    let partition = format!("--partition hash:${{{{ matrix.shard }}}}/{SHARDS}");
    let mut nextest = 0;
    for step in steps_of(tests) {
        let name = step[0].trim_start_matches("      - ").to_owned();
        let run = step_key(&step, "run").unwrap_or_default();
        let cond = step_key(&step, "if").unwrap_or_default();
        if run.starts_with("cargo nextest run") {
            nextest += 1;
            assert!(
                run.ends_with(&format!(" {partition}")) && run.matches("--partition").count() == 1,
                "ci_pr_tier: the shards' step `{name}` does not run its slice of {SHARDS}: {run}"
            );
        } else if run.starts_with("cargo test") {
            assert!(
                cond.ends_with(" && matrix.shard == 1"),
                "ci_pr_tier: the shards' step `{name}` does not run in shard 1 alone: {cond}"
            );
        }
        if !run.starts_with("cargo test") {
            assert!(
                !cond.contains("matrix."),
                "ci_pr_tier: the shards' step `{name}` runs in some shards only: {cond}"
            );
        }
    }
    assert!(nextest > 0, "ci_pr_tier: the shards run no nextest step");
    let (id, verdict) = jobs
        .iter()
        .find(|(id, l)| job_name(id, l) == "ci")
        .expect("ci_pr_tier: no job bears ci");
    assert!(
        verdict
            .iter()
            .any(|l| l.strip_prefix("    needs: ").is_some_and(|n| n
                .trim_matches(['[', ']'])
                .split(',')
                .any(|x| x.trim() == "tests"))),
        "ci_pr_tier: ci ({id}) does not need the shards"
    );
    assert!(
        verdict
            .iter()
            .any(|l| l.trim() == "SHARDS: ${{ needs.tests.result }}"),
        "ci_pr_tier: ci ({id}) does not read the shards' result"
    );
}

#[test]
fn ci_pr_tier_ci_runs_as_three_shards() {
    check_shards(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_pr_tier_ci_runs_as_three_shards,
    "The product tests' step run whole in every shard, every test three times.",
    expected = "does not run its slice of 3",
    check_shards(&edited(
        ".github/workflows/ci.yml",
        "        run: cargo nextest run $PACKAGES --partition hash:${{ matrix.shard }}/3\n",
        "        run: cargo nextest run $PACKAGES\n",
    ))
);

mod a_slice_of_four {
    use super::*;
    negative_control!(
        ci_pr_tier_ci_runs_as_three_shards,
        "A reader step cut into four slices on three shards, the fourth never run.",
        expected = "does not run its slice of 3",
        check_shards(&edited(
            ".github/workflows/ci.yml",
            "--test part6_catalogue --partition hash:${{ matrix.shard }}/3\n",
            "--test part6_catalogue --partition hash:${{ matrix.shard }}/4\n",
        ))
    );
}

mod two_shards {
    use super::*;
    negative_control!(
        ci_pr_tier_ci_runs_as_three_shards,
        "A matrix of two shards, the third slice never run.",
        expected = "the product tests are not a matrix of 3 shards",
        check_shards(&edited(
            ".github/workflows/ci.yml",
            "        shard: [1, 2, 3]\n    env:\n      PRIN_GPU_BACKEND: vulkan\n      RUNNER_DISK_TMPFS",
            "        shard: [1, 2]\n    env:\n      PRIN_GPU_BACKEND: vulkan\n      RUNNER_DISK_TMPFS",
        ))
    );
}

mod doctests_everywhere {
    use super::*;
    negative_control!(
        ci_pr_tier_ci_runs_as_three_shards,
        "The doctests in every shard, three times.",
        expected = "does not run in shard 1 alone",
        check_shards(&edited(
            ".github/workflows/ci.yml",
            " && needs.changes.outputs.doc_packages != '' && matrix.shard == 1\n",
            " && needs.changes.outputs.doc_packages != ''\n",
        ))
    );
}

mod a_verdict_that_waits_for_nothing {
    use super::*;
    negative_control!(
        ci_pr_tier_ci_runs_as_three_shards,
        "`ci` without the shards among its needs.",
        expected = "ci (tests-verdict) does not need the shards",
        check_shards(&edited(
            ".github/workflows/ci.yml",
            "    needs: [changes, tests]\n",
            "    needs: [changes]\n",
        ))
    );
}

/// The script of `ci`'s step `Every shard passed`, run under bash with `SHARDS` the shards' result: whether it passed.
fn verdict_passes(workflow: &str, shards: &str) -> bool {
    let jobs = jobs(workflow);
    let (_, lines) = jobs
        .iter()
        .find(|(id, l)| job_name(id, l) == "ci")
        .expect("ci_pr_tier: no job bears ci");
    let step = steps_of(lines)
        .into_iter()
        .find(|s| s[0] == "      - name: Every shard passed")
        .expect("ci_pr_tier: ci has no step `Every shard passed`");
    let run = step_key(&step, "run").expect("ci_pr_tier: the step has no run");
    let script = run
        .strip_prefix('\'')
        .and_then(|r| r.strip_suffix('\''))
        .unwrap_or(&run)
        .replace("''", "'");
    std::process::Command::new("bash")
        .args(["--noprofile", "--norc", "-e", "-c", &script])
        .env("SHARDS", shards)
        .output()
        .expect("bash runs")
        .status
        .success()
}

/// `ci` passes when the shards' matrix succeeded, and only then: a shard that failed, was cancelled or never ran (the
/// matrix's result `failure`, `cancelled` or `skipped`, or none) fails it.
fn check_verdict(workflow: &str) {
    assert!(
        verdict_passes(workflow, "success"),
        "ci_pr_tier: ci fails when every shard passed"
    );
    for result in ["failure", "cancelled", "skipped", ""] {
        assert!(
            !verdict_passes(workflow, result),
            "ci_pr_tier: ci passes when the shards ended {result:?}"
        );
    }
}

#[test]
fn ci_pr_tier_ci_fails_unless_every_shard_passed() {
    check_verdict(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_pr_tier_ci_fails_unless_every_shard_passed,
    "A verdict that passes unless a shard failed, so a skipped or cancelled shard passes.",
    expected = "ci passes when the shards ended \"cancelled\"",
    check_verdict(&edited(
        ".github/workflows/ci.yml",
        "[ \"$SHARDS\" = success ] ||",
        "[ \"$SHARDS\" != failure ] ||",
    ))
);

/// Each test binary a reader step of the shards names (`-p <pkg> … --test <name>`) holds a test: a partitioned step
/// passes on a slice with none (NEXTEST_NO_TESTS), so a binary emptied of its tests would pass unseen.
fn check_readers_hold_tests(workflow: &str) {
    let jobs = jobs(workflow);
    let (_, tests) = jobs
        .iter()
        .find(|(id, _)| id == "tests")
        .expect("ci_pr_tier: no job tests");
    let mut seen = 0;
    for step in steps_of(tests) {
        let run = step_key(&step, "run").unwrap_or_default();
        let words: Vec<&str> = run.split_whitespace().collect();
        let Some(pkg) = words.windows(2).find(|w| w[0] == "-p").map(|w| w[1]) else {
            continue;
        };
        for name in words.windows(2).filter(|w| w[0] == "--test").map(|w| w[1]) {
            let path = test_source(&format!("{pkg}::{name}")).expect("a binary");
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            assert!(
                text.contains("#[test]"),
                "ci_pr_tier: the reader binary {pkg}::{name} holds no test"
            );
            seen += 1;
        }
    }
    assert!(seen > 0, "ci_pr_tier: the shards name no reader binary");
}

#[test]
fn ci_pr_tier_reader_binaries_hold_tests() {
    check_readers_hold_tests(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_pr_tier_reader_binaries_hold_tests,
    "A reader step naming a binary that is no test file.",
    expected = "the reader binary validation::no_such_binary holds no test",
    check_readers_hold_tests(&edited(
        ".github/workflows/ci.yml",
        "--test part6_catalogue --partition",
        "--test no_such_binary --partition",
    ))
);

// ---- The crates fetched first, then offline (#481's `clippy`, curl 56; the CI diagnosis of 11 Nov 2026) -------------

/// The jobs that fetch the crates before their own work.
const FETCHING: [&str; 3] = ["clippy", "build", "process"];

const FETCH_STEP: &str = "      - name: cargo fetch --locked (up to three tries)";

/// Each of [`FETCHING`] fetches the crates in a step of its own before any other cargo step, over HTTP/1.1, and that
/// step alone turns the job's later cargo runs offline; each keeps its registry cache under a key naming its own job
/// (R-285, REQ-SYS-073), saved only on a push to `main` (R-326).
fn check_fetch_first(workflow: &str) {
    let jobs = jobs(workflow);
    for id in FETCHING {
        let (_, lines) = jobs
            .iter()
            .find(|(job, _)| job == id)
            .unwrap_or_else(|| panic!("ci_pr_tier: no job {id}"));
        let steps = steps_of(lines);
        let at = steps
            .iter()
            .position(|s| s[0] == FETCH_STEP)
            .unwrap_or_else(|| panic!("ci_pr_tier: {id} fetches no crates first"));
        let cargo_before = steps[..at].iter().any(|s| {
            step_key(s, "run").is_some_and(|r| r.contains("cargo ") && !r.starts_with("rustup"))
        });
        assert!(
            !cargo_before,
            "ci_pr_tier: {id} runs cargo before it fetches the crates"
        );
        let fetch = &steps[at];
        assert!(
            fetch
                .iter()
                .any(|l| l.trim() == "CARGO_HTTP_MULTIPLEXING: \"false\"")
                && step_key(fetch, "timeout-minutes").is_some(),
            "ci_pr_tier: {id}'s fetch is not over HTTP/1.1 under a limit"
        );
        let keys: Vec<&str> = lines
            .iter()
            .filter_map(|l| l.strip_prefix("          shared-key: "))
            .collect();
        assert!(
            keys == [id]
                && lines
                    .iter()
                    .any(|l| l.trim() == "save-if: ${{ github.event_name == 'push' && github.ref == 'refs/heads/main' }}"),
            "ci_pr_tier: {id} does not keep its registry under a key of its own, saved only on main"
        );
    }
}

/// The fetch step's script under bash, `cargo` a stand-in failing its first `fails` calls: whether it passed, the calls,
/// and what it wrote to GITHUB_ENV.
fn fetch_run(workflow: &str, fails: u32) -> (bool, u32, String) {
    let jobs = jobs(workflow);
    let (_, lines) = jobs
        .iter()
        .find(|(job, _)| job == "clippy")
        .expect("ci_pr_tier: no job clippy");
    let step = steps_of(lines)
        .into_iter()
        .find(|s| s[0] == FETCH_STEP)
        .expect("ci_pr_tier: clippy fetches no crates");
    let script = step_key(&step, "run").expect("a run");
    // A directory of this process's own: the test and its controls run at once, each in a process of its own.
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("ci_pr_tier_fetch_{}_{fails}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let env_file = dir.join("env");
    std::fs::write(&env_file, "").expect("env");
    let calls = dir.join("calls");
    let out = std::process::Command::new("bash")
        .args(["--noprofile", "--norc", "-eo", "pipefail", "-c"])
        .arg(format!(
            "cargo() {{ echo \"cargo $*\" >> \"$CALLS\"; [ \"$(wc -l < \"$CALLS\")\" -gt {fails} ]; }}\n{script}\n"
        ))
        .env("CALLS", &calls)
        .env("GITHUB_ENV", &env_file)
        .output()
        .expect("bash runs");
    let n = std::fs::read_to_string(&calls)
        .unwrap_or_default()
        .lines()
        .filter(|l| *l == "cargo fetch --locked")
        .count() as u32;
    (
        out.status.success(),
        n,
        std::fs::read_to_string(&env_file).unwrap_or_default(),
    )
}

/// The fetch passes at its first, second or third try, and only then turns the job offline; three failures fail it.
fn check_fetch_tries(workflow: &str) {
    for fails in 0..3 {
        assert_eq!(
            fetch_run(workflow, fails),
            (true, fails + 1, "CARGO_NET_OFFLINE=true\n".to_owned()),
            "ci_pr_tier: the fetch does not pass at try {} and turn the job offline",
            fails + 1
        );
    }
    assert_eq!(
        fetch_run(workflow, 3),
        (false, 3, String::new()),
        "ci_pr_tier: three failed fetches do not fail the step"
    );
}

#[test]
fn ci_pr_tier_crates_are_fetched_first() {
    check_fetch_first(&read(".github/workflows/ci.yml"));
    check_fetch_tries(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_pr_tier_crates_are_fetched_first,
    "A fetch tried twice.",
    expected = "the fetch does not pass at try 3",
    {
        let ci = read(".github/workflows/ci.yml").replace(
            "for attempt in 1 2 3; do cargo fetch --locked",
            "for attempt in 1 2; do cargo fetch --locked",
        );
        check_fetch_tries(&ci)
    }
);

mod fetch_on_http2 {
    use super::*;
    negative_control!(
        ci_pr_tier_crates_are_fetched_first,
        "The build's fetch over HTTP/2, whose framing errors failed it.",
        expected = "build's fetch is not over HTTP/1.1 under a limit",
        {
            let text = read(".github/workflows/ci.yml");
            let at = text.find("\n  build:\n").expect("the job");
            let (head, tail) = text.split_at(at);
            check_fetch_first(&format!(
                "{head}{}",
                tail.replacen("          CARGO_HTTP_MULTIPLEXING: \"false\"\n", "", 1)
            ))
        }
    );
}

mod own_registry {
    use super::*;
    negative_control!(
        ci_pr_tier_crates_are_fetched_first,
        "clippy restoring another job's registry key, which R-285 gives each job its own of.",
        expected = "clippy does not keep its registry under a key of its own",
        {
            let text = read(".github/workflows/ci.yml");
            let at = text.find("\n  clippy:\n").expect("the job");
            let (head, tail) = text.split_at(at);
            check_fetch_first(&format!(
                "{head}{}",
                tail.replacen(
                    "          shared-key: clippy\n",
                    "          shared-key: ci\n",
                    1
                )
            ))
        }
    );
}

mod fetch_after_the_lint {
    use super::*;
    negative_control!(
        ci_pr_tier_crates_are_fetched_first,
        "The process job with no fetch step.",
        expected = "process fetches no crates first",
        {
            let text = read(".github/workflows/ci.yml");
            let at = text.find("\n  process:\n").expect("the job");
            let (head, tail) = text.split_at(at);
            check_fetch_first(&format!(
                "{head}{}",
                tail.replacen(FETCH_STEP, "      - name: fetch", 1)
            ))
        }
    );
}
