//! qa's tests for TASK-M7-04's lint context of a built-in occupant (`xtask::lint_wgsl::occupant_context`), written
//! from its contract: a built-in occupant is linted as the assembler presents it, after the prelude, the library files
//! and its `// @uniform` block declared as the struct `uniforms`; a `// @uniform` line without a name before `:` or a
//! type between `:` and `=` is an error. They kill the three mutants CI's shards left alive on the PR head
//! (`lint_wgsl.rs` `occupant_context`: `&&` → `||` in the name/type filter, and each `!` of the two newline guards).
//!
//! Each test takes its subject as an argument, and its negative control runs it on a wrong one (R-176).

use validation::negative_control;
use xtask::lint_wgsl::{check_fragment_after, occupant_context};

/// An occupant that reads a uniform and calls a library function.
const OCCUPANT: &str = "// @uniform k: f32 = 0.5\n\
fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return lib_scale(rgb) * uniforms.k * b; }\n";

const LIBRARY: &str = "fn lib_scale(c: vec3<f32>) -> vec3<f32> { return c * 2.0; }\n";

/// The context `ctx` gives for (prelude, library, occupant) must let the occupant parse and validate after it: the
/// library's function and the `uniforms` struct reachable, each piece on its own lines, whether or not the piece before
/// it ends in a newline.
fn check_presented(ctx: fn(&str, &[String], &str) -> Result<String, String>) {
    let cases: [(&str, Vec<String>); 4] = [
        // The prelude ends in a comment and no newline: the library must start a new line.
        (
            "// prelude, no newline at the end",
            vec![LIBRARY.to_owned()],
        ),
        // The library ends in a comment and no newline: the uniform block must start a new line.
        (
            "// prelude\n",
            vec![format!("{LIBRARY}// library, no newline at the end")],
        ),
        // Two library files, the first without a final newline.
        (
            "// prelude\n",
            vec![
                "// first library file, no newline".to_owned(),
                LIBRARY.to_owned(),
            ],
        ),
        // Well-formed newlines throughout.
        ("// prelude\n", vec![LIBRARY.to_owned()]),
    ];
    for (prelude, library) in cases {
        let context = ctx(prelude, &library, OCCUPANT)
            .unwrap_or_else(|e| panic!("occupant context refused: {e}"));
        let result = check_fragment_after(&context, OCCUPANT);
        assert!(
            result.is_ok(),
            "occupant not presented as the assembler does: {:?}\n--- context ---\n{context}",
            result.err()
        );
    }
}

#[test]
fn qa_occupant_context_presents_each_piece_on_its_own_lines() {
    check_presented(occupant_context);
}

negative_control!(
    qa_occupant_context_presents_each_piece_on_its_own_lines,
    "pieces joined without the newline a piece lacks",
    expected = "not presented",
    check_presented(|prelude, library, _| {
        Ok(format!(
        "{prelude}{}struct U {{\n    k: f32,\n}}\n@group(0) @binding(1) var<uniform> uniforms: U;\n",
        library.concat()
    ))
    })
);

/// Every `// @uniform` line lacking a name before `:` or a type between `:` and `=` is refused.
fn check_malformed(ctx: fn(&str, &[String], &str) -> Result<String, String>) {
    for line in [
        "// @uniform : f32 = 0.0",
        "// @uniform k: = 0.0",
        "// @uniform   :   = 0.0",
        "// @uniform k f32 = 0.0",
        "// @uniform k: f32",
    ] {
        let occupant =
            format!("{line}\nfn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> {{ return rgb; }}\n");
        assert!(
            ctx("// prelude\n", &[], &occupant).is_err(),
            "malformed uniform line accepted: `{line}`"
        );
    }
}

#[test]
fn qa_occupant_context_refuses_a_uniform_without_name_or_type() {
    check_malformed(occupant_context);
}

negative_control!(
    qa_occupant_context_refuses_a_uniform_without_name_or_type,
    "a context that accepts every line",
    expected = "malformed uniform line accepted",
    check_malformed(|prelude, _, _| Ok(prelude.to_owned()))
);
