//! QA tests for TASK-M1-15, written from the requirements it closes and the definitions it writes (render contract
//! Part 4, "The occupant rules `cargo xtask lint wgsl` checks"), not from the lint's code:
//! - REQ-RENDER-015 / REQ-COL-069 "guarded": in a colour, brightness or post occupant that is not a debug view, every
//!   use of a read of `ftle` sits where `has_ftle && s.ftle_valid` holds for the sample `s` it was read from (in the
//!   branch an `if` on that conjunction takes, with further conjuncts allowed, or after an `if` on its negation that
//!   returns); `ensemble_spread` is guarded by `has_ensemble()`; the one unguarded use is the absence test, a
//!   `bitcast<u32>` compared by `==` or `!=`; an aggregate relying on NaN propagation or `min`/`max`/`clamp` on NaN
//!   fails (R-255); a debug view is not held to the guard; the checked-in runtime WGSL is covered; `isnan` and NaN
//!   self-comparisons fail in an occupant (`qa_m1_15_guard_*`, `qa_m1_15_nan_*`).
//! - REQ-RENDER-018 / REQ-RENDER-086: no fragment-stage WGSL reads `r_sh` or `p_sh` of a stored or read-side
//!   `SimState` but the read side's `sample_read`; the shadow's own field views are excepted from that rule only
//!   (`qa_m1_15_shadow_*`).
//! - REQ-COL-005 / REQ-COL-069: a colour occupant names no `var<storage>` global, uses `ctx.sample.word` only whole
//!   as a generated accessor's argument, and reads none of the chart, integrator or scheduler identifiers Part 4 lists,
//!   while the identifiers Part 4 says are not those stay clean (`qa_m1_15_seams_*`).
//!
//! Each case is linted by `xtask::lint_wgsl::lint` over a scratch copy of the repository's fragment and shader trees
//! with the probe files added, so the real context, exceptions and occupancy apply. Each test registers its negative
//! control (R-176).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;
use xtask::lint_wgsl::{lint, FRAG_DIR, LIB_DIR, READ_SIDE_FILE};

/// The workspace root.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a directory");
    for entry in std::fs::read_dir(from).expect("a directory") {
        let path = entry.expect("an entry").path();
        let dest = to.join(path.file_name().expect("a name"));
        if path.is_dir() {
            copy_tree(&path, &dest);
        } else {
            std::fs::copy(&path, &dest).expect("a copy");
        }
    }
}

/// The marker a probe line carries for each finding it must give: `// qa: <rule>, <rule>`.
const MARK: &str = "// qa: ";

/// A probe file: its path relative to the root, and its text. `append` adds the text to the repository's file at that
/// path instead of replacing it.
struct Probe {
    rel: String,
    text: String,
    append: bool,
}

fn put(rel: &str, text: &str) -> Probe {
    Probe {
        rel: rel.to_owned(),
        text: text.to_owned(),
        append: false,
    }
}

fn append(rel: &str, text: &str) -> Probe {
    Probe {
        rel: rel.to_owned(),
        text: text.to_owned(),
        append: true,
    }
}

/// Each file's findings as `(line, rule)`, for the repository's trees with `probes` written in, by `lint`.
fn lint_with(probes: &[Probe]) -> BTreeMap<String, BTreeSet<(u32, String)>> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_m1_15_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the root");
    std::fs::write(root.join("Cargo.toml"), "").expect("Cargo.toml");
    let repo = repo();
    copy_tree(&repo.join(FRAG_DIR), &root.join(FRAG_DIR));
    copy_tree(&repo.join(LIB_DIR), &root.join(LIB_DIR));
    for p in probes {
        let path = root.join(&p.rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        let text = if p.append {
            let base = std::fs::read_to_string(&path).expect("the file appended to");
            format!("{base}\n{}", p.text)
        } else {
            p.text.clone()
        };
        std::fs::write(&path, text).expect("the probe");
    }
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let reports = result.unwrap_or_else(|e| panic!("the probes did not lint: {e}"));
    reports
        .into_iter()
        .map(|r| {
            let set = r
                .findings
                .iter()
                .map(|f| (f.line.unwrap_or(0), f.rule.to_string()))
                .collect();
            (r.file, set)
        })
        .collect()
}

/// The findings a probe's text marks, with `offset` added to each line (an appended probe's lines follow the file's).
fn marked(text: &str, offset: u32) -> BTreeSet<(u32, String)> {
    let mut out = BTreeSet::new();
    for (k, line) in (1u32..).zip(text.lines()) {
        if let Some((_, rules)) = line.split_once(MARK) {
            for rule in rules.split(',') {
                out.insert((k + offset, rule.trim().to_owned()));
            }
        }
    }
    out
}

/// Every probe gives exactly the findings it marks, and every file of the repository's trees stays clean.
fn check_probes(probes: &[Probe]) {
    let found = lint_with(probes);
    let mut seen = BTreeSet::new();
    for p in probes {
        let offset = if p.append {
            let base = std::fs::read_to_string(repo().join(&p.rel)).expect("the file appended to");
            u32::try_from(format!("{base}\n").lines().count()).expect("a line count")
        } else {
            0
        };
        let want = marked(&p.text, offset);
        let got = found.get(&p.rel).cloned().unwrap_or_default();
        assert!(
            got == want,
            "{}: the lint does not find what Part 4 says: found {got:?}, expected {want:?}",
            p.rel
        );
        seen.insert(p.rel.clone());
    }
    for (file, f) in &found {
        if !seen.contains(file) {
            assert!(
                f.is_empty(),
                "{file}, a repository file, has findings: {f:?}"
            );
        }
    }
}

/// `probes` with every mark dropped: the lint, unchanged, then disagrees with the probe, so the test can fail.
#[cfg(feature = "controls")]
fn unmarked(probes: Vec<Probe>) -> Vec<Probe> {
    probes
        .into_iter()
        .map(|p| Probe {
            text: p.text.replace(MARK, "// was: "),
            ..p
        })
        .collect()
}

// ── REQ-RENDER-015: the guard ───────────────────────────────────────────────────────────────────────────────────

/// Uses that Part 4 calls guarded: the conjunction with a further conjunct, the negation that returns first, the
/// absence test, an aggregate that skips invalid samples, and `has_ensemble()`.
const GUARDED: &str = r"
fn colour(ctx: Ctx) -> vec3<f32> {
    if (has_ftle && ctx.sample.ftle_valid && ctx.sample.t_end_step > 0u) {
        return ramp_viridis(ctx.sample.ftle);
    }
    if (bitcast<u32>(ctx.sample.ftle) == 0x7fc00000u) {
        return vec3<f32>(0.0);
    }
    if (bitcast<u32>(ctx.sample.ftle) != 0x7fc00000u) {
        return vec3<f32>(0.5);
    }
    return vec3<f32>(1.0);
}

fn negation_returns(ctx: Ctx) -> vec3<f32> {
    if (!has_ftle || !ctx.sample.ftle_valid) {
        return vec3<f32>(0.0);
    }
    return ramp_viridis(ctx.sample.ftle);
}

fn negated_conjunction_returns(ctx: Ctx) -> f32 {
    if (!(has_ftle && ctx.sample.ftle_valid)) {
        return 0.0;
    }
    var keep = ctx.sample.ftle;
    return keep * 2.0;
}

fn aggregate_max(ctx: Ctx) -> f32 {
    var hi = 0.0;
    for (var k = 0u; k < 4u; k = k + 1u) {
        let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        if (has_ftle && s.ftle_valid) {
            hi = max(hi, s.ftle);
        }
    }
    return hi;
}

fn spread(ctx: Ctx) -> vec3<f32> {
    if (!has_ensemble()) {
        return vec3<f32>(0.0);
    }
    return ramp_viridis(ctx.sample.ensemble_spread);
}
";

/// Uses that Part 4 calls unguarded, each marked.
const UNGUARDED: &str = r"
fn colour(ctx: Ctx) -> vec3<f32> {
    if (has_ftle || ctx.sample.ftle_valid) {
        return ramp_viridis(ctx.sample.ftle); // qa: guard
    }
    return vec3<f32>(0.0);
}

fn select_is_not_a_branch(ctx: Ctx) -> vec3<f32> {
    return ramp_viridis(select(0.0, ctx.sample.ftle, has_ftle && ctx.sample.ftle_valid)); // qa: guard
}

fn another_samples_gate(ctx: Ctx) -> f32 {
    let s0 = sample_read(0u, 0.0, false, vec3<f32>(1.0), ctx.params);
    let s1 = sample_read(1u, 0.0, false, vec3<f32>(1.0), ctx.params);
    if (has_ftle && s0.ftle_valid) {
        return s1.ftle; // qa: guard
    }
    return 0.0;
}

fn ordered_bits_are_not_the_absence_test(ctx: Ctx) -> f32 {
    if (bitcast<u32>(ctx.sample.ftle) > 0x7f800000u) { // qa: guard
        return 1.0;
    }
    return 0.0;
}

fn bits_as_a_value(ctx: Ctx) -> f32 {
    return f32(bitcast<u32>(ctx.sample.ftle)); // qa: guard
}

fn negation_that_does_not_return(ctx: Ctx) -> f32 {
    var x = 1.0;
    if (!(has_ftle && ctx.sample.ftle_valid)) {
        x = 0.0;
    }
    return x * ctx.sample.ftle; // qa: guard
}

fn stored_before_the_guard(ctx: Ctx) -> f32 {
    var keep = ctx.sample.ftle; // qa: guard
    if (has_ftle && ctx.sample.ftle_valid) {
        return keep;
    }
    return 0.0;
}

fn twice(x: f32) -> f32 {
    return 2.0 * x;
}

fn handed_to_a_function(ctx: Ctx) -> f32 {
    let y = twice(ctx.sample.ftle); // qa: guard
    return y;
}

fn clamp_aggregate(ctx: Ctx) -> f32 {
    var acc = 0.0;
    for (var k = 0u; k < 4u; k = k + 1u) {
        let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
        acc = clamp(acc + s.ftle, 0.0, 1.0); // qa: guard
    }
    return acc;
}

fn aggregate_gated_on_the_wrong_sample(ctx: Ctx) -> f32 {
    var acc = 0.0;
    if (has_ftle && ctx.sample.ftle_valid) {
        for (var k = 0u; k < 4u; k = k + 1u) {
            let s = sample_read(k, 0.0, false, vec3<f32>(1.0), ctx.params);
            acc = acc + s.ftle; // qa: guard
        }
    }
    return acc;
}

fn spread_under_the_ftle_guard(ctx: Ctx) -> f32 {
    if (has_ftle && ctx.sample.ftle_valid) {
        return ctx.sample.ensemble_spread; // qa: guard
    }
    return 0.0;
}

fn spread_where_the_uniform_is_false(ctx: Ctx) -> f32 {
    if (!has_ensemble()) {
        return ctx.sample.ensemble_spread; // qa: guard
    }
    return 0.0;
}
";

/// A debug view's unguarded read: not held to the guard (Part 4; R-79, R-114).
const DEBUG_UNGUARDED: &str = r"
fn colour(ctx: Ctx) -> vec3<f32> {
    return vec3<f32>(ctx.sample.ftle, ctx.sample.ensemble_spread, 0.0);
}
";

/// The same read marked, where an occupant that is not a debug view holds it.
const OCCUPANT_UNGUARDED: &str = r"
fn colour(ctx: Ctx) -> vec3<f32> {
    return vec3<f32>(ctx.sample.ftle, 0.0, 0.0); // qa: guard
}
";

fn guard_probes() -> Vec<Probe> {
    vec![
        put(
            "crates/render/shaders/wgsl/frag/colour/qa_guarded.wgsl",
            GUARDED,
        ),
        put(
            "crates/render/shaders/wgsl/frag/colour/qa_unguarded.wgsl",
            UNGUARDED,
        ),
        // Every non-debug occupant directory that reads the context, and the runtime WGSL's field ramps.
        put(
            "crates/render/shaders/wgsl/frag/post/qa_unguarded.wgsl",
            OCCUPANT_UNGUARDED,
        ),
        put(
            "crates/render/shaders/wgsl/frag/brightness/qa_unguarded.wgsl",
            OCCUPANT_UNGUARDED,
        ),
        put(
            "crates/render/frag/runtime/field_ramp/qa_unguarded.wgsl",
            OCCUPANT_UNGUARDED,
        ),
        // Every debug directory.
        put(
            "crates/render/shaders/wgsl/frag/debug/qa_unguarded.wgsl",
            DEBUG_UNGUARDED,
        ),
        put(
            "crates/render/frag/debug/generated/qa_unguarded.wgsl",
            DEBUG_UNGUARDED,
        ),
        put(
            "crates/render/frag/debug/reductions/qa_unguarded.wgsl",
            DEBUG_UNGUARDED,
        ),
        put(
            "crates/render/frag/runtime/debug/presets/qa_unguarded.wgsl",
            DEBUG_UNGUARDED,
        ),
    ]
}

#[test]
fn qa_m1_15_guard_follows_part4() {
    check_probes(&guard_probes());
}

negative_control!(
    qa_m1_15_guard_follows_part4,
    "the guard probes with their marks dropped, as a guard rule that fires nowhere would leave them",
    expected = "does not find what Part 4 says",
    check_probes(&unmarked(guard_probes()))
);

/// `isnan` and NaN self-comparisons in an occupant and in the runtime WGSL (REQ-RENDER-015's grep).
const NAN_TESTS: &str = r"
fn isnan(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7fffffffu) > 0x7f800000u;
}

fn colour(ctx: Ctx) -> vec3<f32> {
    let v = ctx.sample.d_min;
    let a = isnan(v); // qa: isinf-isnan
    let b = v != v; // qa: self-compare
    let c = !(v == v); // qa: self-compare
    return vec3<f32>(select(0.0, 1.0, a || b || c));
}
";

fn nan_probes() -> Vec<Probe> {
    vec![
        put(
            "crates/render/shaders/wgsl/frag/colour/qa_nan.wgsl",
            NAN_TESTS,
        ),
        put(
            "crates/render/frag/runtime/field_ramp/qa_nan.wgsl",
            NAN_TESTS,
        ),
        put(
            "crates/render/frag/runtime/debug/presets/qa_nan.wgsl",
            NAN_TESTS,
        ),
    ]
}

#[test]
fn qa_m1_15_nan_tests_fail_everywhere() {
    check_probes(&nan_probes());
}

negative_control!(
    qa_m1_15_nan_tests_fail_everywhere,
    "the NaN-test probes with their marks dropped",
    expected = "does not find what Part 4 says",
    check_probes(&unmarked(nan_probes()))
);

// ── REQ-RENDER-018: the shadow ──────────────────────────────────────────────────────────────────────────────────

const SHADOW_IN_OCCUPANT: &str = r"
fn colour(ctx: Ctx) -> vec3<f32> {
    let a = ctx.sample.r_sh[0].x; // qa: shadow-read
    let b = ctx.sample.p_sh[2].y; // qa: shadow-read
    return vec3<f32>(a, b, 0.0);
}
";

/// The shadow read through a function the stain's context declares, from a stored `SimState` and a read-side one.
const SHADOW_IN_CONTEXT: &str = r"
fn qa_stored_shadow(i: u32) -> f32 {
    return simstate_buffer[i].r_sh[1].x; // qa: shadow-read
}

fn qa_read_side_shadow(s: SimState) -> f32 {
    return s.p_sh[0].y; // qa: shadow-read
}
";

/// A function of the read side other than `sample_read` reading the shadow.
const SHADOW_IN_READ_SIDE: &str = r"fn qa_peek(s: SimState) -> f32 {
    return s.r_sh[2].y; // qa: shadow-read
}
";

/// The shadow's own field view is excepted from the shadow rule only: a payload-buffer and an integrator read there
/// still fail.
const SHADOW_VIEW_EXTRA: &str = r"fn qa_extra(ctx: Ctx) -> f32 {
    let a = ctx.sample.r_sh[0].x;
    let b = f32(word_buffer[0u].x); // qa: payload-buffer
    return a + b + ctx.params.dt_macro; // qa: integrator-state
}
";

fn shadow_probes() -> Vec<Probe> {
    vec![
        put(
            "crates/render/shaders/wgsl/frag/colour/qa_shadow.wgsl",
            SHADOW_IN_OCCUPANT,
        ),
        // A debug view that is not the shadow's own field view is held to the rule.
        put(
            "crates/render/frag/debug/generated/qa_shadow.wgsl",
            SHADOW_IN_OCCUPANT,
        ),
        put(
            "crates/render/frag/runtime/field_ramp/qa_shadow.wgsl",
            SHADOW_IN_OCCUPANT,
        ),
        put(
            "crates/render/shaders/wgsl/stain/qa_shadow.wgsl",
            SHADOW_IN_CONTEXT,
        ),
        append(READ_SIDE_FILE, SHADOW_IN_READ_SIDE),
        append(
            "crates/render/frag/debug/generated/r_sh.wgsl",
            SHADOW_VIEW_EXTRA,
        ),
    ]
}

#[test]
fn qa_m1_15_shadow_read_only_by_sample_read() {
    check_probes(&shadow_probes());
}

negative_control!(
    qa_m1_15_shadow_read_only_by_sample_read,
    "the shadow probes with their marks dropped",
    expected = "does not find what Part 4 says",
    check_probes(&unmarked(shadow_probes()))
);

// ── REQ-COL-005: seams 5 and 6 ──────────────────────────────────────────────────────────────────────────────────

/// Part 4's deny-list member by member, and the identifiers it says are not the chart's, integrator's or scheduler's.
const DENY_LIST: &str = r"
fn colour(ctx: Ctx) -> vec3<f32> {
    var acc = 0.0;
    acc += ctx.params.dt_macro; // qa: integrator-state
    acc += ctx.params.delta_0; // qa: integrator-state
    acc += f32(ctx.params.n_renorm); // qa: integrator-state
    acc += ctx.quad.tl.x; // qa: chart-state
    acc += ctx.quad.centre.y; // qa: chart-state
    acc += ctx.quad.half_width.x; // qa: chart-state
    acc += f32(ctx.quad.depth); // qa: scheduler-state
    acc += f32(ctx.quad.state); // qa: scheduler-state
    acc += ctx.quad.coherence; // qa: scheduler-state
    acc += ctx.quad.impurity; // qa: scheduler-state
    acc += ctx.quad.spread; // qa: scheduler-state
    acc += ctx.quad.suspect_frac; // qa: scheduler-state
    acc += ctx.quad.priority; // qa: scheduler-state
    acc += f32(ctx.quad.cache_age); // qa: scheduler-state
    acc += f32(ctx.quad.ancestor_gap); // qa: scheduler-state
    acc += f32(ctx.quad.dominant_outcome); // qa: scheduler-state
    acc += f32(ctx.params.horizon_steps);
    acc += f32(ctx.quad.index) + ctx.quad.uv.x + f32(ctx.quad.sample_count);
    acc += ctx.frag_xy.x + ctx.screen.uv.y + ctx.tile.uv.x;
    acc += f32(ctx.sample.t_end_step) + ctx.ic.m0;
    acc += uniforms.qa_gain;
    acc += f32(fgw_length_raw(ctx.sample.word));
    return vec3<f32>(acc);
}

fn through_a_copy(q: CtxQuad) -> f32 {
    return q.priority; // qa: scheduler-state
}

fn through_a_local(ctx: Ctx) -> f32 {
    var q = ctx.quad;
    return q.half_width.y; // qa: chart-state
}
";

/// Non-accessor payload reads: storage globals, and the packed word other than whole into a generated accessor.
const PAYLOAD_READS: &str = r"
@group(3) @binding(0) var<storage, read> qa_extra_buffer: array<u32>;
var<private> qa_private: f32 = 0.0;

fn own(w: vec4<u32>) -> u32 {
    return fgw_length_raw(w);
}

fn colour(ctx: Ctx) -> vec3<f32> {
    let a = qa_extra_buffer[0u]; // qa: payload-buffer
    let b = ic_buffer[0u].m0; // qa: payload-buffer
    let c = word_buffer[1u].y; // qa: payload-buffer
    let d = ctx.sample.word.x; // qa: raw-word
    let e = (ctx.sample.word >> vec4<u32>(4u)).z; // qa: raw-word
    let f = (ctx.sample.word & vec4<u32>(0xffu)).w; // qa: raw-word
    let g = own(ctx.sample.word); // qa: raw-word
    let h = fgw_length_raw(ctx.sample.word);
    let k = qa_private + uniforms.qa_gain;
    return vec3<f32>(f32(a + c + d + e + f + g + h), b, k);
}
";

/// The uniform block both seam probes read.
const UNIFORM: &str = "// @uniform qa_gain: f32 = 1.0\n";

fn seam_probes() -> Vec<Probe> {
    vec![
        put(
            "crates/render/shaders/wgsl/frag/colour/qa_deny.wgsl",
            &format!("{UNIFORM}{DENY_LIST}"),
        ),
        put(
            "crates/render/frag/runtime/field_ramp/qa_deny.wgsl",
            &format!("{UNIFORM}{DENY_LIST}"),
        ),
        put(
            "crates/render/shaders/wgsl/frag/colour/qa_payload.wgsl",
            &format!("{UNIFORM}{PAYLOAD_READS}"),
        ),
        // A debug view is a colour occupant: seams 5 and 6 bind it too, outside the named exceptions.
        put(
            "crates/render/frag/debug/generated/qa_payload.wgsl",
            &format!("{UNIFORM}{PAYLOAD_READS}"),
        ),
    ]
}

#[test]
fn qa_m1_15_seams_5_and_6_follow_part4() {
    check_probes(&seam_probes());
}

negative_control!(
    qa_m1_15_seams_5_and_6_follow_part4,
    "the seam probes with their marks dropped",
    expected = "does not find what Part 4 says",
    check_probes(&unmarked(seam_probes()))
);

/// Each named exception is to its one rule: a structural view and the fallback tint still may not read the integrator's
/// parameters or decode the word; the coordinate preset excepted from the chart rule may not read the scheduler's
/// fields; the whole-word hash may not read the chart.
fn exception_probes() -> Vec<Probe> {
    vec![
        append(
            "crates/render/shaders/wgsl/frag/debug/s_depth.wgsl",
            "fn qa_x(ctx: Ctx) -> f32 {\n    return f32(ctx.quad.depth) + ctx.params.delta_0; // qa: integrator-state\n}\n",
        ),
        append(
            "crates/render/shaders/wgsl/frag/post/fallback_tint.wgsl",
            "fn qa_x(ctx: Ctx) -> u32 {\n    return ctx.quad.depth + ctx.sample.word.y; // qa: raw-word\n}\n",
        ),
        append(
            "crates/render/frag/runtime/debug/presets/coordinate_uv_source.wgsl",
            "fn qa_x(ctx: Ctx) -> f32 {\n    return ctx.quad.centre.x + ctx.quad.priority; // qa: scheduler-state\n}\n",
        ),
        append(
            "crates/render/shaders/wgsl/frag/debug/word/hash.wgsl",
            "fn qa_x(ctx: Ctx) -> f32 {\n    return ctx.quad.tl.x; // qa: chart-state\n}\n",
        ),
        append(
            "crates/render/shaders/wgsl/frag/debug/accumulators/ftle_running.wgsl",
            "fn qa_x(ctx: Ctx) -> f32 {\n    return ctx.params.dt_macro + ctx.sample.r_sh[0].x; // qa: shadow-read\n}\n",
        ),
    ]
}

#[test]
fn qa_m1_15_exceptions_are_each_to_one_rule() {
    check_probes(&exception_probes());
}

negative_control!(
    qa_m1_15_exceptions_are_each_to_one_rule,
    "the exception probes with their marks dropped",
    expected = "does not find what Part 4 says",
    check_probes(&unmarked(exception_probes()))
);
