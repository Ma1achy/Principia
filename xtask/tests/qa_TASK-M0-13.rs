//! QA tests for TASK-M0-13's `cargo xtask lint wgsl`, written from REQ-RENDER-001 (render contract Part 5, "Unpack
//! layer" and its WGSL traps; payload §6; R-86, R-343), not from the implementation. Each test takes the real
//! generated WGSL, breaks one rule the requirement states, and asserts the lint fires naming that rule; the clean file
//! passes. The rules: the u32 overload of `extractBits`; no f64; no `enable f16`; `r`, `p`, `r_sh`, `p_sh` as
//! `array<vec2<f32>, 3>`; the word buffer its own binding, `array<vec4<u32>>`, never inside `SimState`;
//! `simstate_buffer: array<SimStateFTLE>` at `@group(1) @binding(0)` and `word_buffer` at `@group(1) @binding(1)`,
//! equal to the generated constants, nothing in group 0; and no function but the read side's `sample_read` using either
//! buffer (R-343 as R-378 amends it: `sample_state`/`sample_word` are replaced by `sample_read`'s per-member loads), the
//! read side (`read_side.wgsl`) linted as the unpack layer's continuation.
//!
//! Each test has a registered negative control (R-176): the unbroken file, on which the rule must not fire.

use std::path::Path;

use validation::negative_control;
use xtask::lint_wgsl::{check, check_read_side, Finding, Rule, GENERATED, READ_SIDE_FILE};

fn generated() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(GENERATED);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// `src` is `generated()` with `from` replaced by `to`; the pattern must be there, so a control cannot pass vacuously.
fn edit(from: &str, to: &str) -> String {
    let g = generated();
    assert!(
        g.contains(from),
        "the edit's pattern is not in the generated WGSL: {from}"
    );
    g.replacen(from, to, 1)
}

/// The generated read side with `extra` appended, linted as the unpack layer's continuation (R-378).
fn read_side_with(extra: &str) -> Vec<Finding> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(READ_SIDE_FILE);
    let read_side = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    check_read_side(&generated(), &format!("{read_side}\n{extra}"))
        .expect("the edited WGSL parses and validates")
}

/// The lint fires on `src` with at least one finding of `rule`, its text naming the rule.
fn fires(src: &str, rule: Rule) {
    fires_in(
        check(src).expect("the edited WGSL parses and validates"),
        rule,
    );
}

/// `found` has at least one finding of `rule`, its text naming the rule.
fn fires_in(found: Vec<Finding>, rule: Rule) {
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

#[test]
fn qa_lint_wgsl_generated_file_is_clean() {
    let found = check(&generated()).expect("the generated WGSL parses and validates");
    assert!(
        found.is_empty(),
        "lint wgsl findings on the generated file: {found:?}"
    );
}

negative_control!(
    qa_lint_wgsl_generated_file_is_clean,
    "the generated file with an f64 helper appended is not clean",
    expected = "lint wgsl findings on the generated file",
    {
        let found = check(&format!(
            "{}\nfn wide(x: f64) -> f64 {{ return x; }}\n",
            generated()
        ))
        .expect("parses");
        assert!(
            found.is_empty(),
            "lint wgsl findings on the generated file: {found:?}"
        );
    }
);

const SD_STATE: &str = "fn sd_state(w: u32) -> u32 { return extractBits(w, 0u, 3u); }";

#[test]
fn qa_lint_wgsl_i32_extract_bits_fires() {
    fires(
        &edit(
            SD_STATE,
            "fn sd_state(w: u32) -> u32 { return bitcast<u32>(extractBits(bitcast<i32>(w), 0u, 3u)); }",
        ),
        Rule::ExtractBitsU32,
    );
}

negative_control!(
    qa_lint_wgsl_i32_extract_bits_fires,
    "the u32 accessor must not fire",
    expected = "rule extractBits-u32 did not fire",
    fires(&edit(SD_STATE, SD_STATE), Rule::ExtractBitsU32)
);

#[test]
fn qa_lint_wgsl_i32_vector_extract_bits_fires() {
    fires(
        &format!(
            "{}\nfn fgw_signed(word: vec4<u32>) -> vec4<i32> {{ return extractBits(bitcast<vec4<i32>>(word), 25u, 7u); }}\n",
            generated()
        ),
        Rule::ExtractBitsU32,
    );
}

negative_control!(
    qa_lint_wgsl_i32_vector_extract_bits_fires,
    "the vec4<u32> form must not fire",
    expected = "rule extractBits-u32 did not fire",
    fires(
        &format!(
            "{}\nfn fgw_unsigned(word: vec4<u32>) -> vec4<u32> {{ return extractBits(word, 25u, 7u); }}\n",
            generated()
        ),
        Rule::ExtractBitsU32
    )
);

#[test]
fn qa_lint_wgsl_f64_fires() {
    fires(
        &format!("{}\nfn wide(x: f64) -> f64 {{ return x; }}\n", generated()),
        Rule::NoF64,
    );
}

negative_control!(
    qa_lint_wgsl_f64_fires,
    "an f32 helper must not fire",
    expected = "rule no-f64 did not fire",
    fires(
        &format!(
            "{}\nfn narrow(x: f32) -> f32 {{ return x; }}\n",
            generated()
        ),
        Rule::NoF64
    )
);

#[test]
fn qa_lint_wgsl_enable_f16_fires() {
    fires(&format!("enable f16;\n{}", generated()), Rule::NoEnableF16);
}

negative_control!(
    qa_lint_wgsl_enable_f16_fires,
    "`enable f16` in a comment is no directive and must not fire",
    expected = "rule no-enable-f16 did not fire",
    fires(
        &format!("// enable f16;\n{}", generated()),
        Rule::NoEnableF16
    )
);

const R_SH: &str = "    r_sh: array<vec2<f32>, 3>,";

#[test]
fn qa_lint_wgsl_shadow_not_vec2_grouped_fires() {
    fires(&edit(R_SH, "    r_sh: array<f32, 6>,"), Rule::Vec2Groups);
}

negative_control!(
    qa_lint_wgsl_shadow_not_vec2_grouped_fires,
    "the vec2 shadow must not fire",
    expected = "rule vec2-groups did not fire",
    fires(&edit(R_SH, R_SH), Rule::Vec2Groups)
);

const CLOSURE_FTLE: &str = "    closure_min: f32,\n    closure_step_reserved: u32, // `closure_step` in bits 0–15, `_reserved` in bits 16–31 (WGSL has no u16; R-343)\n}\n\n// `SimStateBase`";

#[test]
fn qa_lint_wgsl_word_inside_simstate_fires() {
    fires(
        &edit(
            CLOSURE_FTLE,
            "    closure_min: f32,\n    closure_step_reserved: u32,\n    free_group_word: vec4<u32>,\n}\n\n// `SimStateBase`",
        ),
        Rule::WordBinding,
    );
}

negative_control!(
    qa_lint_wgsl_word_inside_simstate_fires,
    "the SimState without a word must not fire",
    expected = "rule word-binding did not fire",
    fires(&edit(CLOSURE_FTLE, CLOSURE_FTLE), Rule::WordBinding)
);

const STATE_BIND: &str =
    "@group(1) @binding(0) var<storage, read> simstate_buffer: array<SimStateFTLE>;";
const WORD_BIND: &str = "@group(1) @binding(1) var<storage, read> word_buffer: array<vec4<u32>>;";

#[test]
fn qa_lint_wgsl_state_in_group_0_fires() {
    fires(
        &edit(
            STATE_BIND,
            "@group(0) @binding(0) var<storage, read> simstate_buffer: array<SimStateFTLE>;",
        ),
        Rule::Bindings,
    );
}

negative_control!(
    qa_lint_wgsl_state_in_group_0_fires,
    "the state at @group(1) @binding(0) must not fire",
    expected = "rule bindings did not fire",
    fires(&edit(STATE_BIND, STATE_BIND), Rule::Bindings)
);

#[test]
fn qa_lint_wgsl_word_at_wrong_binding_fires() {
    fires(
        &edit(
            WORD_BIND,
            "@group(1) @binding(2) var<storage, read> word_buffer: array<vec4<u32>>;",
        ),
        Rule::Bindings,
    );
}

negative_control!(
    qa_lint_wgsl_word_at_wrong_binding_fires,
    "the word at @group(1) @binding(1) must not fire",
    expected = "rule bindings did not fire",
    fires(&edit(WORD_BIND, WORD_BIND), Rule::Bindings)
);

#[test]
fn qa_lint_wgsl_state_buffer_of_base_fires() {
    fires(
        &edit(
            STATE_BIND,
            "@group(1) @binding(0) var<storage, read> simstate_buffer: array<SimStateBase>;",
        )
        .replace(
            "fn sample_state(i: u32) -> SimStateFTLE",
            "fn sample_state(i: u32) -> SimStateBase",
        ),
        Rule::Bindings,
    );
}

negative_control!(
    qa_lint_wgsl_state_buffer_of_base_fires,
    "array<SimStateFTLE> must not fire",
    expected = "rule bindings did not fire",
    fires(&edit(STATE_BIND, STATE_BIND), Rule::Bindings)
);

const WORD_BINDING_CONST: &str = "const WORD_BINDING: u32 = 1u;";

#[test]
fn qa_lint_wgsl_binding_constant_differs_from_attribute_fires() {
    fires(
        &edit(WORD_BINDING_CONST, "const WORD_BINDING: u32 = 2u;"),
        Rule::Bindings,
    );
}

negative_control!(
    qa_lint_wgsl_binding_constant_differs_from_attribute_fires,
    "WORD_BINDING = 1u must not fire",
    expected = "rule bindings did not fire",
    fires(
        &edit(WORD_BINDING_CONST, WORD_BINDING_CONST),
        Rule::Bindings
    )
);

#[test]
fn qa_lint_wgsl_extra_group_0_binding_fires() {
    fires(
        &format!(
            "{}\n@group(0) @binding(0) var<uniform> frame: vec4<u32>;\n",
            generated()
        ),
        Rule::Bindings,
    );
}

negative_control!(
    qa_lint_wgsl_extra_group_0_binding_fires,
    "a uniform in group 2 must not fire",
    expected = "rule bindings did not fire",
    fires(
        &format!(
            "{}\n@group(2) @binding(0) var<uniform> frame: vec4<u32>;\n",
            generated()
        ),
        Rule::Bindings
    )
);

/// A read of `simstate_buffer` outside `sample_read`, beside the generated read side.
const STATE_OF_DIRECT: &str = "fn state_of(i: u32) -> u32 { return simstate_buffer[i].packed_a; }";
/// The same read through `sample_read`, the one reader (R-343, R-378).
#[cfg(feature = "controls")]
const STATE_OF_READER: &str = "fn state_of(i: u32) -> u32 { return sample_read(i, 0.0, false, vec3<f32>(1.0), ReadParams(1.0, 1.0, 1u, 1u)).state; }";

#[test]
fn qa_lint_wgsl_state_read_outside_sample_state_fires() {
    fires(
        &format!(
            "{}\nfn state_of(i: u32) -> u32 {{ return simstate_buffer[i].packed_a; }}\n",
            generated()
        ),
        Rule::SampleOnly,
    );
    fires_in(read_side_with(STATE_OF_DIRECT), Rule::SampleOnly);
}

negative_control!(
    qa_lint_wgsl_state_read_outside_sample_state_fires,
    "reading through sample_read must not fire",
    expected = "rule sample-only did not fire",
    fires_in(read_side_with(STATE_OF_READER), Rule::SampleOnly)
);

/// A fragment entry point reading the word itself, and one reading it through `sample_read`.
const FS_DIRECT: &str = "@fragment\nfn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> { let w = word_buffer[u32(p.x)]; return vec4<f32>(f32(w.w)); }";
#[cfg(feature = "controls")]
const FS_READER: &str = "@fragment\nfn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> { let w = sample_read(u32(p.x), 0.0, false, vec3<f32>(1.0), ReadParams(1.0, 1.0, 1u, 1u)).word; return vec4<f32>(f32(w.w)); }";

#[test]
fn qa_lint_wgsl_word_read_in_entry_point_fires() {
    fires(
        &format!(
            "{}\n@fragment\nfn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {{ let w = word_buffer[u32(p.x)]; return vec4<f32>(f32(w.w)); }}\n",
            generated()
        ),
        Rule::SampleOnly,
    );
    fires_in(read_side_with(FS_DIRECT), Rule::SampleOnly);
}

negative_control!(
    qa_lint_wgsl_word_read_in_entry_point_fires,
    "an entry point reading through sample_read must not fire",
    expected = "rule sample-only did not fire",
    fires_in(read_side_with(FS_READER), Rule::SampleOnly)
);
