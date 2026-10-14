//! `cargo xtask lint wgsl`'s occupant rules (TASK-M1-15; render contract Part 4, "The occupant rules `cargo xtask lint
//! wgsl` checks"):
//! - REQ-RENDER-015: no `isnan` or NaN self-comparison (the float rules), and every use of a read of `ftle` guarded by
//!   `has_ftle && ftle_valid` for its sample, aggregates included (R-255), and of `ensemble_spread` by `has_ensemble()`
//!   (R-145), in an occupant that is not a debug view (`lint_wgsl_occupant_fixtures_*`);
//! - REQ-RENDER-018: no fragment-stage WGSL reads `r_sh` or `p_sh` but the read side's `sample_read`
//!   (`lint_wgsl_occupant_shadow_*`);
//! - REQ-COL-005: a colour occupant names no payload buffer, decodes no packed word by hand, and reads no chart,
//!   integrator or scheduler identifier (`lint_wgsl_occupant_fixtures_*`, `lint_wgsl_occupant_deny_list_*`);
//! - PIT-3: one fixture with an `isnan`, a `raw != raw`, a raw payload read and an `r_sh` read fails each rule
//!   (`lint_wgsl_occupant_pit3_*`);
//! - the exceptions the definitions name are each needed, and each only for its rule (`lint_wgsl_occupant_exceptions_*`);
//!   the repository, the checked-in runtime WGSL among it, lints clean (`lint_wgsl_occupant_repository_*`).
//!
//! Each fixture under `tests/fixtures/lint_wgsl/occupant/` marks every finding it must give with `// fires <rule>` on
//! its line; a fixture without marks must give none. Each test registers its negative control (R-176).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;
use xtask::lint_wgsl::{
    accessors, check, check_fragment, check_fragment_after, check_occupant_after, excepted, lint,
    occupancy, uniform_block, view_context, Occupancy, Rule, DEBUG_OCCUPANTS, DEBUG_VIEWS,
    EXCEPTIONS, GENERATED, LIB_FILES, OCCUPANT_DIR, PRELUDE, READ_SIDE_FILE, REDUCTION_VIEWS,
    RUNTIME_DEBUG, RUNTIME_DIR, SOURCE_DIR, STAIN_DIR,
};

/// The fixtures' directory.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lint_wgsl/occupant")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The workspace this crate is in.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

/// A scratch workspace holding the repository's prelude, library, generated layer, read side and stain context, and
/// `occupant` at `rel`.
fn workspace(rel: &str, occupant: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "lint_wgsl_occupant_rules_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let write = |rel: &str, text: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        std::fs::write(&path, text).expect("the file");
    };
    write("Cargo.toml", "");
    let repo = repo();
    for file in [PRELUDE, GENERATED, READ_SIDE_FILE] {
        write(file, &read(&repo.join(file)));
    }
    for dir in [LIB_FILES, STAIN_DIR] {
        for entry in std::fs::read_dir(repo.join(dir)).expect("the directory") {
            let path = entry.expect("an entry").path();
            let name = path
                .file_name()
                .expect("a name")
                .to_string_lossy()
                .into_owned();
            if name.ends_with(".wgsl") {
                write(&format!("{dir}/{name}"), &read(&path));
            }
        }
    }
    write(rel, occupant);
    root
}

/// The findings `cargo xtask lint wgsl` gives `occupant` placed at `rel`, each `(line, rule)`; every finding is at
/// `rel`.
fn findings_at(rel: &str, occupant: &str) -> BTreeSet<(u32, String)> {
    let root = workspace(rel, occupant);
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let reports = result.unwrap_or_else(|e| panic!("{rel} did not lint: {e}"));
    let mut out = BTreeSet::new();
    for r in &reports {
        for f in &r.findings {
            assert_eq!(r.file, rel, "a finding off the occupant: {}", f.at(&r.file));
            out.insert((f.line.unwrap_or(0), f.rule.to_string()));
        }
    }
    out
}

/// The findings `text` marks: each line's `// fires <rule>, <rule>`.
fn marks(text: &str) -> BTreeSet<(u32, String)> {
    let mut out = BTreeSet::new();
    for (k, line) in (1u32..).zip(text.lines()) {
        if let Some((_, rules)) = line.split_once("// fires ") {
            for rule in rules.split(", ") {
                out.insert((k, rule.trim().to_owned()));
            }
        }
    }
    out
}

/// Where a fixture is placed: a `debug_` fixture as a hand-written debug view, a `source_` fixture as a built-in
/// source, any other as a colour occupant.
fn placed(name: &str) -> String {
    if name.starts_with("debug_") {
        format!("{DEBUG_OCCUPANTS}/test.wgsl")
    } else if name.starts_with("source_") {
        format!("{SOURCE_DIR}/test.wgsl")
    } else {
        format!("{OCCUPANT_DIR}/colour/test.wgsl")
    }
}

/// The fixture `name`, linted where it is placed, gives exactly the findings it marks.
fn check_fixture(name: &str, text: &str) {
    let found = findings_at(&placed(name), text);
    let want = marks(text);
    assert!(
        found == want,
        "{name} does not fire as marked: found {found:?}, marked {want:?}"
    );
}

/// Every fixture's names, sorted.
fn fixture_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixtures())
        .expect("the fixtures")
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| n.ends_with(".wgsl"))
        .collect();
    names.sort();
    names
}

/// Every fixture fires as marked, and every occupant rule is marked somewhere: each is shown able to fire (PIT-3).
fn check_all_fixtures(transform: fn(&str) -> String) {
    let mut fired = BTreeSet::new();
    for name in fixture_names() {
        let text = transform(&read(&fixtures().join(&name)));
        check_fixture(&name, &text);
        fired.extend(marks(&text).into_iter().map(|(_, r)| r));
    }
    for rule in [
        Rule::ShadowRead,
        Rule::PayloadBuffer,
        Rule::RawWord,
        Rule::ChartState,
        Rule::IntegratorState,
        Rule::SchedulerState,
        Rule::Guard,
    ] {
        assert!(fired.contains(&rule.to_string()), "no fixture fires {rule}");
    }
}

#[test]
fn lint_wgsl_occupant_fixtures_fire_as_marked() {
    check_all_fixtures(str::to_owned);
}

negative_control!(
    lint_wgsl_occupant_fixtures_fire_as_marked,
    "the fixtures with their marks removed, as a rule that fires nowhere would leave them",
    expected = "does not fire as marked",
    check_all_fixtures(|t| t.replace("// fires ", "// "))
);

/// The PIT-3 fixture breaks the isnan, self-comparison, payload-buffer and shadow rules, each.
fn check_pit3(text: &str) {
    let rules: BTreeSet<String> = findings_at(&placed("pit3"), text)
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    for rule in [
        Rule::IsInfNan,
        Rule::SelfCompare,
        Rule::PayloadBuffer,
        Rule::ShadowRead,
    ] {
        assert!(
            rules.contains(&rule.to_string()),
            "the PIT-3 fixture does not fail {rule}: {rules:?}"
        );
    }
}

#[test]
fn lint_wgsl_occupant_pit3_fixture_fails_each_rule() {
    check_pit3(&read(&fixtures().join("pit3.wgsl")));
}

negative_control!(
    lint_wgsl_occupant_pit3_fixture_fails_each_rule,
    "the PIT-3 fixture with its shadow read made a read of r",
    expected = "does not fail shadow-read",
    check_pit3(&read(&fixtures().join("pit3.wgsl")).replace("r_sh[0]", "r[0]"))
);

// ── The deny-list, member by member ───────────────────────────────────────────────────────────────────────────────

/// A colour occupant reading `ctx.<path>`, its third line the read.
fn reading(path: &str) -> String {
    format!(
        "// reads ctx.{path}\nfn colour(ctx: Ctx) -> vec3<f32> {{\n    let x = ctx.{path};\n    return vec3<f32>(0.0);\n}}\n"
    )
}

/// Every member of the quad lane and of the read side's parameters gives the deny-list's rule, or none.
fn check_deny_list(table: &[(&str, Option<Rule>)]) {
    let rel = placed("deny");
    for &(path, rule) in table {
        let found = findings_at(&rel, &reading(path));
        let want: BTreeSet<(u32, String)> = rule.into_iter().map(|r| (3, r.to_string())).collect();
        assert!(found == want, "ctx.{path}: found {found:?}, not {want:?}");
    }
}

/// `ctx.quad`'s members (the stain context's `CtxQuad`) and `ctx.params`' (`ReadParams`), with their rules.
const DENY: [(&str, Option<Rule>); 20] = [
    ("quad.index", None),
    ("quad.tl", Some(Rule::ChartState)),
    ("quad.centre", Some(Rule::ChartState)),
    ("quad.half_width", Some(Rule::ChartState)),
    ("quad.uv", None),
    ("quad.sample_count", None),
    ("quad.depth", Some(Rule::SchedulerState)),
    ("quad.state", Some(Rule::SchedulerState)),
    ("quad.impurity", Some(Rule::SchedulerState)),
    ("quad.spread", Some(Rule::SchedulerState)),
    ("quad.suspect_frac", Some(Rule::SchedulerState)),
    ("quad.priority", Some(Rule::SchedulerState)),
    ("quad.cache_age", Some(Rule::SchedulerState)),
    ("quad.coherence", Some(Rule::SchedulerState)),
    ("quad.ancestor_gap", Some(Rule::SchedulerState)),
    ("quad.dominant_outcome", Some(Rule::SchedulerState)),
    ("params.dt_macro", Some(Rule::IntegratorState)),
    ("params.delta_0", Some(Rule::IntegratorState)),
    ("params.n_renorm", Some(Rule::IntegratorState)),
    ("params.horizon_steps", None),
];

#[test]
fn lint_wgsl_occupant_deny_list_member_by_member() {
    check_deny_list(&DENY);
}

negative_control!(
    lint_wgsl_occupant_deny_list_member_by_member,
    "the quad-local coordinate taken for the scheduler's",
    expected = "ctx.quad.uv: found",
    check_deny_list(&[("quad.uv", Some(Rule::SchedulerState))])
);

/// A context whose `Ctx` has a chart lane and a `RenderQuad`, as the harness's lanes do: each read gives `want`.
fn check_chart_lane(read_member: &str, want: Option<Rule>) {
    let prefix = "struct Chart { z: f32 }\nstruct RenderQuad { quad_depth: u32 }\n\
                  struct Ctx { chart: Chart, quad: RenderQuad, x: f32 }\n";
    let source = format!("fn colour(ctx: Ctx) -> f32 {{\n    return f32({read_member});\n}}\n");
    let found: Vec<Rule> =
        check_occupant_after(prefix, &source, Occupancy::Guarded, &BTreeSet::new())
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .map(|f| f.rule)
            .collect();
    let want: Vec<Rule> = want.into_iter().collect();
    assert_eq!(found, want, "{read_member} gives {found:?}, not {want:?}");
}

#[test]
fn lint_wgsl_occupant_deny_list_chart_lane_and_render_quad() {
    check_chart_lane("ctx.chart.z", Some(Rule::ChartState));
    check_chart_lane("ctx.quad.quad_depth", Some(Rule::SchedulerState));
    check_chart_lane("ctx.x", None);
}

negative_control!(
    lint_wgsl_occupant_deny_list_chart_lane_and_render_quad,
    "a plain member of Ctx taken for the chart's",
    expected = "ctx.x gives",
    check_chart_lane("ctx.x", Some(Rule::ChartState))
);

/// The deny-list finding for `read_member`, under a context with a chart lane, a `RenderQuad` and the read params,
/// names `whose` state it reads.
fn check_whose(read_member: &str, whose: &str) {
    let prefix = "struct Chart { z: f32 }\nstruct RenderQuad { quad_depth: u32 }\nstruct ReadParams { dt_macro: f32 }\n\
                  struct Ctx { chart: Chart, quad: RenderQuad, params: ReadParams }\n";
    let source = format!("fn colour(ctx: Ctx) -> f32 {{\n    return f32({read_member});\n}}\n");
    let found = check_occupant_after(prefix, &source, Occupancy::Guarded, &BTreeSet::new())
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        found.len() == 1 && found[0].what.contains(&format!("{whose} state")),
        "{read_member} is not named {whose} state: {found:?}"
    );
}

#[test]
fn lint_wgsl_occupant_deny_list_names_whose_state() {
    check_whose("ctx.chart.z", "the chart's");
    check_whose("ctx.params.dt_macro", "the integrator's");
    check_whose("ctx.quad.quad_depth", "the scheduler's");
}

negative_control!(
    lint_wgsl_occupant_deny_list_names_whose_state,
    "the chart's state named the scheduler's",
    expected = "is not named the scheduler's state",
    check_whose("ctx.chart.z", "the scheduler's")
);

// ── The shadow rule outside the occupants ─────────────────────────────────────────────────────────────────────────

/// A hand-written fragment file with a stored `SimState*` holding the shadow, its function `f` reading `r_sh`.
const SHADOW_FILE: &str =
    "struct SimStateTest { r: array<vec2<f32>, 3>, r_sh: array<vec2<f32>, 3> }\n\
                           fn f(s: SimStateTest) -> f32 {\n    return s.r_sh[0].x + s.r[0].x;\n}\n";

/// The shadow rule holds in every fragment-stage file, the read side's `sample_read` its one reader: the generated layer
/// and read side give no shadow finding, a function of another name reading it there does, and so does a hand-written
/// file, alone or after a prefix.
fn check_shadow(reader: &str) {
    let layer = read(&repo().join(GENERATED));
    let side = read(&repo().join(READ_SIDE_FILE));
    let shadow = |found: Vec<xtask::lint_wgsl::Finding>| {
        found
            .into_iter()
            .filter(|f| f.rule == Rule::ShadowRead)
            .count()
    };
    let joined = format!("{layer}\n{side}");
    assert_eq!(
        shadow(check(&joined).unwrap_or_else(|e| panic!("{e}"))),
        0,
        "the read side's {reader} reads the shadow as a sample"
    );
    let renamed = joined.replace(&format!("fn {reader}("), "fn other_reader(");
    assert_eq!(
        shadow(check(&renamed).unwrap_or_else(|e| panic!("{e}"))),
        4,
        "another function's reads of r_sh and p_sh are not found"
    );
    assert_eq!(
        shadow(check_fragment(SHADOW_FILE).unwrap_or_else(|e| panic!("{e}"))),
        1
    );
    let after = check_fragment_after("const K: f32 = 1.0;\n", SHADOW_FILE)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        after.iter().map(|f| (f.rule, f.line)).collect::<Vec<_>>(),
        vec![(Rule::ShadowRead, Some(3))]
    );
}

#[test]
fn lint_wgsl_occupant_shadow_read_only_by_the_read_side() {
    check_shadow("sample_read");
}

negative_control!(
    lint_wgsl_occupant_shadow_read_only_by_the_read_side,
    "a reader the read side does not have: its sample_read keeps reading the shadow under its own name",
    expected = "another function's reads of r_sh and p_sh are not found",
    check_shadow("no_such_reader")
);

// ── Occupancy, the exceptions and the accessors ───────────────────────────────────────────────────────────────────

/// Each kind of file is the occupant it should be.
fn check_occupancy(table: &[(String, Option<Occupancy>)]) {
    for (rel, want) in table {
        assert_eq!(occupancy(rel), *want, "{rel}");
    }
}

fn occupancy_table() -> Vec<(String, Option<Occupancy>)> {
    vec![
        (format!("{DEBUG_VIEWS}/ftle.wgsl"), Some(Occupancy::Debug)),
        (
            format!("{REDUCTION_VIEWS}/r_dircos.wgsl"),
            Some(Occupancy::Debug),
        ),
        (
            format!("{DEBUG_OCCUPANTS}/s_depth.wgsl"),
            Some(Occupancy::Debug),
        ),
        (
            format!("{RUNTIME_DEBUG}/presets/uv_quad_colour.wgsl"),
            Some(Occupancy::Debug),
        ),
        (
            format!("{RUNTIME_DIR}/field_ramp/ftle.wgsl"),
            Some(Occupancy::Guarded),
        ),
        (
            format!("{OCCUPANT_DIR}/colour/outcome_state.wgsl"),
            Some(Occupancy::Guarded),
        ),
        (
            format!("{OCCUPANT_DIR}/combiner/multiply.wgsl"),
            Some(Occupancy::Guarded),
        ),
        (format!("{SOURCE_DIR}/ftle.wgsl"), Some(Occupancy::Source)),
        (
            format!("{OCCUPANT_DIR}/sources/ftle.wgsl"),
            Some(Occupancy::Guarded),
        ),
        (GENERATED.to_owned(), None),
        (READ_SIDE_FILE.to_owned(), None),
        (PRELUDE.to_owned(), None),
        (format!("{STAIN_DIR}/context.wgsl"), None),
        (
            "crates/render/shaders/wgsl/compositor/blur.wgsl".to_owned(),
            None,
        ),
        (format!("{RUNTIME_DIR}.wgsl"), None),
    ]
}

#[test]
fn lint_wgsl_occupant_occupancy_by_path() {
    check_occupancy(&occupancy_table());
}

negative_control!(
    lint_wgsl_occupant_occupancy_by_path,
    "a field ramp taken for a debug view",
    expected = "field_ramp/ftle.wgsl",
    check_occupancy(&[(
        format!("{RUNTIME_DIR}/field_ramp/ftle.wgsl"),
        Some(Occupancy::Debug)
    )])
);

/// Every `.wgsl` file of the repository whose path begins with `prefix`.
fn files_from(prefix: &str) -> Vec<String> {
    let root = repo();
    let (dir, _) = prefix.rsplit_once('/').expect("a directory");
    let mut out: Vec<String> = std::fs::read_dir(root.join(dir))
        .unwrap_or_else(|e| panic!("{dir}: {e}"))
        .map(|e| {
            format!(
                "{dir}/{}",
                e.expect("an entry").file_name().to_string_lossy()
            )
        })
        .filter(|rel| rel.starts_with(prefix) && rel.ends_with(".wgsl"))
        .collect();
    out.sort();
    out
}

/// Each exception is needed: every repository file it covers breaks its rule without it; and it excepts that rule only.
fn check_exceptions(exceptions: &[(&str, Rule)]) {
    let root = repo();
    let context = view_context(&root).unwrap_or_else(|e| panic!("{e}"));
    let layer = read(&root.join(GENERATED));
    let side = read(&root.join(READ_SIDE_FILE));
    let accessors = accessors(&layer, &side).unwrap_or_else(|e| panic!("{e}"));
    for &(prefix, rule) in exceptions {
        let files = files_from(prefix);
        assert!(!files.is_empty(), "the exception {prefix} covers no file");
        for rel in files {
            let source = read(&root.join(&rel));
            let occupancy = occupancy(&rel).expect("an occupant");
            let prefix_text = format!(
                "{context}{}",
                uniform_block(&source).unwrap_or_else(|e| panic!("{e}"))
            );
            let found = check_occupant_after(&prefix_text, &source, occupancy, &accessors)
                .unwrap_or_else(|e| panic!("{rel}: {e}"));
            assert!(
                found.iter().any(|f| f.rule == rule),
                "{rel} does not break {rule} without its exception"
            );
            assert!(excepted(&rel, rule), "{rel} is not excepted from {rule}");
            assert!(
                !excepted(&rel, Rule::Guard),
                "{rel} is excepted from the guard rule"
            );
        }
    }
}

#[test]
fn lint_wgsl_occupant_exceptions_are_each_needed() {
    check_exceptions(&EXCEPTIONS);
}

negative_control!(
    lint_wgsl_occupant_exceptions_are_each_needed,
    "an exception for the r_sh view's rule given as the scheduler rule",
    expected = "does not break scheduler-state without its exception",
    check_exceptions(&[(
        "crates/render/frag/debug/generated/r_sh.wgsl",
        Rule::SchedulerState
    )])
);

/// The generated accessors are the unpack layer's and the read side's functions, and nothing else.
fn check_accessors(want: &str) {
    let layer = read(&repo().join(GENERATED));
    let side = read(&repo().join(READ_SIDE_FILE));
    let names = accessors(&layer, &side).unwrap_or_else(|e| panic!("{e}"));
    for name in [
        "fgw_length_raw",
        "fgw_symbol",
        "sd_last_symbol_valid",
        "sample_read",
        "ic_read",
        want,
    ] {
        assert!(names.contains(name), "`{name}` is not a generated accessor");
    }
    for name in ["dbg_hash_word", "colour", "debug_invalid"] {
        assert!(
            !names.contains(name),
            "`{name}` is taken for a generated accessor"
        );
    }
    assert!(
        accessors("fn (", "").is_err(),
        "a layer that does not parse gives accessors"
    );
}

#[test]
fn lint_wgsl_occupant_accessors_are_the_generated_functions() {
    check_accessors("benettin_delta");
}

negative_control!(
    lint_wgsl_occupant_accessors_are_the_generated_functions,
    "a presentation function taken for a generated accessor",
    expected = "`dbg_hash_word` is not a generated accessor",
    check_accessors("dbg_hash_word")
);

// ── The repository ────────────────────────────────────────────────────────────────────────────────────────────────

/// The repository lints clean, the checked-in runtime WGSL (`want`) among the files it lints.
fn check_repository(want: &[&str]) {
    let reports = lint(&repo()).unwrap_or_else(|e| panic!("the repository does not lint: {e}"));
    for r in &reports {
        assert!(r.findings.is_empty(), "{}: {:?}", r.file, r.findings);
    }
    for file in want {
        assert!(
            reports.iter().any(|r| r.file == *file),
            "{file} is not linted"
        );
    }
}

#[test]
fn lint_wgsl_occupant_repository_lints_clean() {
    check_repository(&[
        "crates/render/frag/runtime/field_ramp/ftle.wgsl",
        "crates/render/frag/runtime/debug/presets/coordinate_uv_source.wgsl",
        "crates/render/frag/debug/generated/r_sh.wgsl",
    ]);
}

negative_control!(
    lint_wgsl_occupant_repository_lints_clean,
    "a runtime file that is not there",
    expected = "is not linted",
    check_repository(&["crates/render/frag/runtime/field_ramp/no_such_field.wgsl"])
);

// ── A tier constant in a `const` declaration ─────────────────────────────────────────────────────────────────────

/// A context with the prelude's tier constant, a read-side `SimState` and `Ctx`.
const TIER_PREFIX: &str =
    "const has_ftle: bool = true;\nstruct SimState { ftle: f32, ftle_valid: bool }\n\
                           struct Ctx { sample: SimState }\n";

/// An occupant whose guard is `guard`, after `declarations`: with a tier constant in a `const`, it lints, and its one
/// read of `ftle` is reported as a guard finding that names why it could not be checked.
fn check_const_tier(declarations: &str, guard: &str) {
    let source = format!(
        "{declarations}\nfn colour(ctx: Ctx) -> f32 {{\n    if ({guard} && ctx.sample.ftle_valid) {{\n        \
         return ctx.sample.ftle;\n    }}\n    return 0.0;\n}}\n"
    );
    let found = check_occupant_after(TIER_PREFIX, &source, Occupancy::Guarded, &BTreeSet::new())
        .unwrap_or_else(|e| panic!("the occupant did not lint: {e}"));
    assert!(
        found.len() == 1
            && found[0].rule == Rule::Guard
            && found[0].line == Some(4)
            && found[0].what.contains("its guard cannot be checked")
            && found[0].what.contains("const"),
        "the read under a const tier constant is not reported unchecked: {found:?}"
    );
}

#[test]
fn lint_wgsl_occupant_const_tier_is_reported_unchecked() {
    check_const_tier("const USE_FTLE: bool = has_ftle;", "USE_FTLE");
}

negative_control!(
    lint_wgsl_occupant_const_tier_is_reported_unchecked,
    "the tier constant read in the branch condition itself, which the guard checks and finds guarded",
    expected = "is not reported unchecked",
    check_const_tier("", "has_ftle")
);
