//! qa's tests for TASK-M7-54's names (REQ-RENDER-103: every registered entry carries its passes, and the generated
//! WGSL of a graph compiles under naga and holds each node's passes), written from the requirement and WGSL's own
//! rules, not from the implementation:
//!
//! - WGSL's identifiers are Unicode's XID_Start (or `_`) then XID_Continue (WGSL, "Identifiers"): a helper named
//!   beyond ASCII, a combining mark or a digit inside it, is one name, so two nodes of its pass generate a module
//!   that validates under naga, each node's helper its own.
//! - An attribute's name is no expression (`@align(align)` beside a helper `align`), a directive word inside a
//!   comment is no directive, a `@diagnostic` attribute with a comment after its `@` is no directive, and the lab's
//!   names read (never declared) in a helper are the module's: each such pass registers and its module validates.
//! - A pass the module cannot hold — a directive anywhere, or a declaration of one of the lab's names (`run`, `tex`,
//!   `res`, `px`, `prev`, `in_<port>`, `p_<param>`) other than `fn run`, at module level, as a local, a loop's `var`
//!   or a later parameter, the name beyond ASCII included — fails to register, the refusal naming the pass's file
//!   and the name or directive.
//!
//! Each test registers its negative control (R-176).

use naga::valid::{Capabilities, ValidationFlags, Validator};
use render::print::codegen::generate;
use render::print::graph::{expand, Graph};
use render::print::registry::Registry;
use render::print::shipped;
use validation::negative_control;

fn compile(source: &str) {
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|e| {
        panic!(
            "qa names: the generated WGSL does not parse: {}",
            e.emit_to_string(source)
        )
    });
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| {
            panic!(
                "qa names: the generated WGSL does not validate: {}",
                e.emit_to_string(source)
            )
        });
}

fn decl(id: &str) -> String {
    serde_json::json!({
        "id": id,
        "label": format!("QA {id}"),
        "category": "op",
        "doc": "A qa test entry.",
        "inputs": [{ "name": "in", "type": "image", "default": "render" }],
        "outputs": [{ "name": "out", "type": "image" }],
        "params": [{ "name": "amount", "kind": "number", "default": 0.5, "min": 0, "max": 1, "step": 0.01, "unit": "fraction" }],
        "passes": [format!("{id}.wgsl")],
        "cost": "light",
        "motion": "full",
        "temporally_stable": true,
        "screen_medium": false,
        "deviations": ["A qa test entry."]
    })
    .to_string()
}

/// The shipped registry with `qa_names` registered, its one pass `pass`, or the refusal.
fn registry(pass: &str) -> Result<Registry, String> {
    let mut r: Registry = shipped().registry.clone();
    let text = pass.to_owned();
    let read = move |file: &str| (file == "qa_names.wgsl").then(|| text.clone());
    r.register(&decl("qa_names"), &read)
        .map(|_| ())
        .map_err(|e| format!("{e}"))?;
    Ok(r)
}

/// Two `qa_names` nodes in a chain, the Render node feeding the first and the output reading the second.
const TWO: &str = r#"{"format":"principia.print/1","name":"Two qa_names","nodes":{"img":{"type":"render"},
  "a":{"type":"qa_names","params":{"amount":0.3},"in":{"in":"img"}},
  "b":{"type":"qa_names","params":{"amount":0.7},"in":{"in":"a"}},
  "out":{"type":"output","in":{"in":"b"}}},"out":"out"}"#;

/// The module of `TWO` over `r`, or the codegen's refusal.
fn module(r: &Registry) -> Result<String, String> {
    let e = expand(
        &Graph::parse(TWO).unwrap_or_else(|e| panic!("parse: {e}")),
        &|_: &str| None,
    )
    .expect("expands");
    generate(&e, r)
        .map(|g| g.source)
        .map_err(|e| format!("{e}"))
}

// ----- passes the module can hold -----

/// Helpers named beyond ASCII: a decomposed `é` (a combining mark, XID_Continue), Greek letters, and a digit after
/// one; each used in `run`.
const UNICODE: &str = "const cafe\u{301}: f32 = 0.25;
const αβ: f32 = 0.5;
fn π2(x: f32) -> f32 { return x * 2.0; }
fn run(uv: vec2<f32>) -> vec4<f32> {
    return tex(in_in, uv) * (cafe\u{301} + αβ * π2(p_amount));
}";

/// A helper named as the attribute beside it, `align`; the attribute's name is kept, its argument is the helper.
const ATTRIBUTE: &str = "const align: u32 = 16u;
struct QaA { @align(align) v: f32, w: f32 }
fn run(uv: vec2<f32>) -> vec4<f32> {
    let a = QaA(uv.x, uv.y);
    return vec4<f32>(a.v, a.w, 0.0, 1.0) * p_amount;
}";

/// Directive words only inside comments, inside a longer name, and a `@diagnostic` attribute with a comment between
/// its `@` and its name.
const DIRECTIVE_WORDS: &str = "// enable f16;
/* requires readonly_and_readwrite_storage_textures; /* diagnostic(off, derivative_uniformity); */ */
const enabled: f32 = 1.0;
@ /* an attribute */ diagnostic(off, derivative_uniformity) fn requires_one(c: f32) -> f32 { return c * enabled; }
fn run(uv: vec2<f32>) -> vec4<f32> { return tex(in_in, uv) * requires_one(p_amount); }";

/// The lab's names read, never declared, in a helper and in `run`, beside names that hold `p_`, `in_` or `res` only
/// inside them.
const LAB_READS: &str =
    "fn shade(c: vec4<f32>) -> vec4<f32> { return c * p_amount + vec4<f32>(px, res) * 0.0; }
fn cap_in_(x: f32) -> f32 { return x; }
fn run(uv: vec2<f32>) -> vec4<f32> {
    let resin = cap_in_(uv.x);
    let prevy = tex(in_in, uv);
    return shade(prevy) * resin;
}";

const ADMITTED: [(&str, &str); 4] = [
    ("names beyond ASCII", UNICODE),
    ("an attribute's name", ATTRIBUTE),
    ("directive words that are no directive", DIRECTIVE_WORDS),
    ("the lab's names read", LAB_READS),
];

/// Each of `cases` registers, and the two-node module, as `emit` writes it, validates under naga and holds the pass's
/// helpers once per node, each its node's own (`helpers` per case: the names that must be renamed).
fn check_admitted(cases: &[(&str, &str)], helpers: &[&[&str]], emit: fn(String) -> String) {
    for ((what, pass), names) in cases.iter().zip(helpers) {
        let r = registry(pass).unwrap_or_else(|e| panic!("qa names: {what}: refused: {e}"));
        let source = emit(module(&r).unwrap_or_else(|e| panic!("qa names: {what}: {e}")));
        for name in *names {
            for k in [1, 2] {
                let own = format!("print_n{k}_pass0_h_{name}");
                assert!(
                    source.contains(&own),
                    "qa names: {what}: no `{own}` in {source}"
                );
            }
        }
        compile(&source);
    }
}

const HELPERS: [&[&str]; 4] = [
    &["cafe\u{301}", "αβ", "π2"],
    &["align", "QaA"],
    &["enabled", "requires_one"],
    &["shade", "cap_in_"],
];

#[test]
fn qa_print_admitted_names_generate_a_valid_module() {
    check_admitted(&ADMITTED, &HELPERS, |s| s);
}

negative_control!(
    qa_print_admitted_names_generate_a_valid_module,
    "two nodes' helpers given one name: the module declares each twice",
    expected = "qa names:",
    // The second node's helpers named as the first's, as a renaming that stopped at the first character beyond
    // ASCII would leave two passes declaring one name.
    check_admitted(&ADMITTED[..1], &[&[][..]], |s| s
        .replace("print_n2_pass0_h_", "print_n1_pass0_h_"))
);

// ----- passes the module cannot hold -----

const RUN: &str = "fn run(uv: vec2<f32>) -> vec4<f32> { return tex(in_in, uv) * p_amount; }";

/// Each pass of `cases` fails to register, the refusal naming `qa_names.wgsl` and holding its text.
fn check_refused(cases: &[(String, &str)]) {
    for (pass, says) in cases {
        match registry(pass) {
            Ok(_) => panic!("qa names: a pass the module cannot hold registered: {pass}"),
            Err(e) => assert!(
                e.contains("qa_names.wgsl") && e.contains(says),
                "qa names: the refusal {e:?} does not name the file and {says:?}"
            ),
        }
    }
}

fn refused_cases() -> Vec<(String, &'static str)> {
    let with = |head: &str| format!("{head}\n{RUN}");
    let body = |stmt: &str| {
        format!("fn run(uv: vec2<f32>) -> vec4<f32> {{ {stmt} return tex(in_in, uv) * p_amount; }}")
    };
    vec![
        (format!("{RUN}\nenable f16;"), "`enable`"),
        (
            with("fn shade(a: f32, /* second */ px: f32) -> f32 { return a; }"),
            "`px`",
        ),
        (body("for (var p_i: i32 = 0; p_i < 2; p_i++) { }"), "`p_i`"),
        (body("let res: f32 = 1.0;"), "`res`"),
        (with("const tex: f32 = 1.0;"), "`tex`"),
        (with("var<private> in_x: f32;"), "`in_x`"),
        (with("const p_π: f32 = 1.0;"), "`p_π`"),
    ]
}

#[test]
fn qa_print_unholdable_passes_refused() {
    check_refused(&refused_cases());
}

negative_control!(
    qa_print_unholdable_passes_refused,
    "a pass that declares only `fn run` of the lab's names, its words in comments: the module can hold it",
    expected = "qa names: a pass the module cannot hold registered",
    check_refused(&[(DIRECTIVE_WORDS.to_owned(), "")])
);
