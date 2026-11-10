//! qa's tests for TASK-M7-54's declaration lines (gui_state_contract "The declaration format": an occupant declares
//! its uniforms and field inputs on `// @uniform` and `// @input` comment lines), written from WGSL's rule, not from
//! the implementation: the text before a line's `//` is WGSL source, and WGSL's blankspace is the space, the tab, the
//! line breaks (line feed, vertical tab, form feed, carriage return, U+0085, U+2028, U+2029) and the marks U+200E and
//! U+200F (WGSL, "Blankspace and line breaks"). Unicode White_Space outside that set (U+00A0, U+2000, U+3000) is no
//! WGSL blankspace, so a line it opens is not a comment line, and is no declaration.
//!
//! - A declaration line opened by any run of WGSL blankspace, the marks included, is read: its uniform keeps its
//!   name, type, default and range, its input its name and domain, and a second declaration of a name already
//!   declared on such a line is refused.
//! - A line opened by Unicode White_Space that is no WGSL blankspace declares nothing.
//!
//! Each test registers its negative control (R-176).

use render::assemble::{Declaration, UniformType};
use validation::negative_control;

/// Runs of WGSL blankspace, each opening a declaration line; the marks alone, mixed, and after a line break.
const BLANK: [&str; 6] = [
    "\u{200E}",
    "\u{200F}",
    "\u{200E}\u{200F}",
    " \u{200F}\t",
    "\t\u{200E} ",
    "\u{2028}\u{200F}",
];

/// Unicode White_Space that WGSL does not read as blankspace.
const NOT_BLANK: [&str; 4] = ["\u{A0}", "\u{2000}", "\u{3000}", "\u{200E}\u{A0}"];

/// Each of `blank` opening the declaration lines declares them, and each of `not_blank` declares nothing; every
/// failure panics "qa decl:".
fn check(blank: &[&str], not_blank: &[&str]) {
    for p in blank {
        let source = format!(
            "{p}// @uniform gain: f32 = 1.0 [0.0, 2.0]\n{p}// @input a [0.0, 1.0]\nfn colour() {{}}"
        );
        let d = Declaration::parse(&source)
            .unwrap_or_else(|e| panic!("qa decl: {p:?} opening the lines, refused: {e}"));
        let u: Vec<_> = d
            .uniforms
            .iter()
            .map(|u| (u.name.as_str(), u.ty, u.default.clone()))
            .collect();
        assert_eq!(
            u,
            vec![("gain", UniformType::F32, vec![1.0])],
            "qa decl: {p:?} opening the line, the uniform is not read"
        );
        let i: Vec<_> = d
            .inputs
            .iter()
            .map(|i| (i.name.as_str(), i.domain))
            .collect();
        assert_eq!(
            i,
            vec![("a", Some((0.0, 1.0)))],
            "qa decl: {p:?} opening the line, the input is not read"
        );
        let twice = format!("// @input a\n{p}// @input a");
        assert!(
            Declaration::parse(&twice).is_err(),
            "qa decl: {p:?} opening the line, its second declaration of `a` is not refused"
        );
    }
    for p in not_blank {
        let source = format!("{p}// @uniform gain: f32 = 1.0\n{p}// @input a\nfn colour() {{}}");
        if let Ok(d) = Declaration::parse(&source) {
            assert!(
                d.uniforms.is_empty() && d.inputs.is_empty(),
                "qa decl: {p:?} is no WGSL blankspace, yet the lines it opens declared {d:?}"
            );
        }
    }
}

#[test]
fn qa_declaration_lines_opened_by_wgsl_blankspace() {
    check(&BLANK, &NOT_BLANK);
}

negative_control!(
    qa_declaration_lines_opened_by_wgsl_blankspace,
    "the no-break space and its kin taken for WGSL blankspace: the lines they open are expected to declare",
    expected = "qa decl:",
    check(&NOT_BLANK, &[])
);
