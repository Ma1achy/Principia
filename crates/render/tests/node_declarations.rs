//! A node's own source describes it (`render::assemble::declare`): each refusal names its line, the descriptive lines
//! change neither the code nor the fragment key, every shipped source names itself and its group, the outcome
//! palette's swatches are its colour controls, and the worked example parses and assembles.

use render::assemble::{
    self, AssembleError, Control, Declaration, Glyph, Kind, Node, Occupant, Stain, Tier,
};
use render::codegen::Slot;
use render::colour::defaults::{self, Resolve};
use render::colour::outcome;
use render::registry::{self, ZERO_SOURCE};
use validation::negative_control;

const EXAMPLE: &str = include_str!("data/every_control.wgsl");

/// The line, 1-based, of `source` that `parse` refuses, or `None` where it parses.
fn refused_line(source: &str) -> Option<usize> {
    match Declaration::parse(source) {
        Ok(_) => None,
        Err(AssembleError::Declaration { line, .. }) => Some(line),
        Err(other) => panic!("refused, but not as a declaration: {other}"),
    }
}

/// Each case's source is refused at its line.
fn check_refusals(cases: &[(&str, usize)]) {
    for (source, line) in cases {
        assert_eq!(
            refused_line(source),
            Some(*line),
            "the source is not refused at line {line}:\n{source}"
        );
    }
}

const HEAD: &str =
    "// @name N\n// @uniform a: f32 = 0.5 [0, 1]\n// @uniform c: vec3<f32> = (1.0, 0.0, 0.0)\n";

fn word_cases() -> Vec<(&'static str, usize)> {
    vec![
        ("// @name N\n// @colour red\n", 2),
        ("// @name N\n// @title N\n", 2),
    ]
}

#[test]
fn declarations_refuse_an_unknown_word() {
    check_refusals(&word_cases());
}

negative_control!(
    declarations_refuse_an_unknown_word,
    "with the word check lifted, an unknown word is accepted",
    expected = "is not refused",
    assemble::declare::probe::without(assemble::declare::Check::Word, || check_refusals(
        &word_cases()
    ))
);

fn control_cases() -> Vec<(String, usize)> {
    vec![
        (format!("{HEAD}// @control a dial \"A\"\n"), 4),
        (format!("{HEAD}// @control a knob \"A\"\n"), 4),
    ]
}

fn check_owned(cases: &[(String, usize)]) {
    let cases: Vec<(&str, usize)> = cases.iter().map(|(s, l)| (s.as_str(), *l)).collect();
    check_refusals(&cases);
}

#[test]
fn declarations_refuse_an_unknown_control() {
    check_owned(&control_cases());
}

negative_control!(
    declarations_refuse_an_unknown_control,
    "with the control check lifted, an unknown control is accepted",
    expected = "is not refused",
    assemble::declare::probe::without(assemble::declare::Check::Control, || check_owned(
        &control_cases()
    ))
);

fn glyph_cases() -> Vec<(String, usize)> {
    vec![
        (format!("{HEAD}// @glyph sparkle\n"), 4),
        // A glyph with nothing to draw: no colour control for its chips, no gradient.
        (format!("{HEAD}// @glyph swatches\n"), 4),
        (format!("{HEAD}// @glyph gradient\n"), 4),
        (
            format!("{HEAD}// @control c colour \"C\" in \"One\"\n// @glyph swatches \"Two\"\n"),
            5,
        ),
    ]
}

#[test]
fn declarations_refuse_an_unknown_or_empty_glyph() {
    check_owned(&glyph_cases());
}

negative_control!(
    declarations_refuse_an_unknown_or_empty_glyph,
    "with the glyph check lifted, an unknown or empty glyph is accepted",
    expected = "is not refused",
    assemble::declare::probe::without(assemble::declare::Check::Glyph, || check_owned(
        &glyph_cases()
    ))
);

fn malformed_cases() -> Vec<(String, usize)> {
    vec![
        (format!("{HEAD}// @control a slider A\n"), 4),
        (format!("{HEAD}// @control a slider \"\"\n"), 4),
        (format!("{HEAD}// @control a slider \"A\" order x\n"), 4),
        (format!("{HEAD}// @control a slider \"A\" then\n"), 4),
        (format!("{HEAD}// @control b slider \"B\"\n"), 4),
        (format!("{HEAD}// @control a slider \"A\"\n// @control a slider \"B\"\n"), 5),
        (format!("{HEAD}// @control a choice(\"x\" \"y\") \"A\"\n"), 4),
        ("// @name N\n// @name M\n".to_owned(), 2),
        ("// @name\n".to_owned(), 1),
        ("// @about a \"quoted\" word\n".to_owned(), 1),
        ("// @version one\n".to_owned(), 1),
        ("// @version 0\n".to_owned(), 1),
        (format!("{HEAD}// @glyph name extra\n"), 4),
        (
            format!(
                "{HEAD}// @uniform d: vec3<f32> = (0.0, 0.0, 0.0)\n// @control c gradient(d) \"G\"\n\
                 // @control d colour \"D\"\n"
            ),
            5,
        ),
        (
            format!(
                "{HEAD}// @uniform d: vec3<f32> = (0.0, 0.0, 0.0)\n// @uniform e: vec3<f32> = (0.0, 0.0, 0.0)\n\
                 // @control c gradient(d) \"G\"\n// @control e gradient(d) \"H\"\n"
            ),
            7,
        ),
    ]
}

#[test]
fn declarations_refuse_a_malformed_line() {
    check_owned(&malformed_cases());
}

negative_control!(
    declarations_refuse_a_malformed_line,
    "with the malformed-line check lifted, a malformed line is accepted",
    expected = "is not refused",
    assemble::declare::probe::without(assemble::declare::Check::Malformed, || check_owned(
        &malformed_cases()
    ))
);

fn fit_cases() -> Vec<(String, usize)> {
    let u = |decl: &str, control: &str| {
        format!("// @name N\n// @uniform x: {decl}\n// @control x {control} \"X\"\n")
    };
    vec![
        (u("vec3<f32> = (0.0, 0.0, 0.0)", "slider"), 3),
        (u("f32 = 0.5", "slider"), 3),
        (u("f32 = 0.5 [0, 1]", "log_slider"), 3),
        (u("f32 = 1.0", "stepper"), 3),
        (u("u32 = 0 [0, 2]", "toggle"), 3),
        (u("u32 = 0 [0, 2]", "choice(\"a\", \"b\")"), 3),
        (u("f32 = 0.0", "choice(\"a\", \"b\")"), 3),
        (u("u32 = 0", "choice(\"a\")"), 3),
        (u("i32 = 0", "flags(\"a\", \"b\")"), 3),
        (u("vec2<f32> = (0.0, 0.0)", "colour"), 3),
        (u("u32 = 0", "angle"), 3),
        (u("vec3<f32> = (0.0, 0.0, 0.0)", "gradient(x)"), 3),
        (u("vec3<f32> = (0.0, 0.0, 0.0)", "gradient(nowhere)"), 3),
        (
            "// @name N\n// @uniform x: vec3<f32> = (0.0, 0.0, 0.0)\n// @uniform y: f32 = 0.0\n\
             // @control x gradient(y) \"X\"\n"
                .to_owned(),
            4,
        ),
    ]
}

#[test]
fn declarations_refuse_a_control_that_does_not_fit() {
    check_owned(&fit_cases());
}

negative_control!(
    declarations_refuse_a_control_that_does_not_fit,
    "with the fit check lifted, a control on the wrong type is accepted",
    expected = "is not refused",
    assemble::declare::probe::without(assemble::declare::Check::Fit, || check_owned(&fit_cases()))
);

fn version_cases() -> Vec<(&'static str, usize)> {
    vec![
        ("// @name N\n// @version 2\n", 2),
        ("// @version 7\n", 1),
        // A later grammar's new word above its version line is refused as a later grammar.
        ("// @name N\n// @sparkle on\n// @version 2\n", 3),
    ]
}

#[test]
fn declarations_refuse_a_later_grammar() {
    check_refusals(&version_cases());
}

negative_control!(
    declarations_refuse_a_later_grammar,
    "with the version check lifted, a later grammar is read as this one",
    expected = "is not refused",
    assemble::declare::probe::without(assemble::declare::Check::Version, || check_refusals(
        &version_cases()
    ))
);

// ── The worked example ─────────────────────────────────────────────────────────────────────────────────────────────

fn example_graph(text: &str) -> Vec<Node> {
    vec![
        Node {
            kind: Kind::Source,
            occupant: Occupant::Field("d_min".into()),
            inputs: vec![],
        },
        Node {
            kind: Kind::Colour,
            occupant: Occupant::Custom(text.to_owned()),
            inputs: vec![Some(0)],
        },
        Node {
            kind: Kind::Combiner,
            occupant: Occupant::BuiltIn("pass_through".into()),
            inputs: vec![Some(1), None],
        },
        Node {
            kind: Kind::Out,
            occupant: Occupant::None,
            inputs: vec![Some(2)],
        },
    ]
}

/// The example declares every control and the swatches glyph, parses, and assembles.
fn check_example(text: &str) {
    let d = Declaration::parse(text).unwrap_or_else(|e| panic!("{e}"));
    let words: Vec<&str> = d
        .described
        .controls
        .iter()
        .map(|c| c.control.word())
        .collect();
    for want in [
        "slider",
        "log_slider",
        "stepper",
        "toggle",
        "choice",
        "flags",
        "colour",
        "angle",
        "gradient",
        "fields",
        "hidden",
    ] {
        assert!(words.contains(&want), "the example has no `{want}` control");
    }
    assert_eq!(d.described.name.as_deref(), Some("Banded tint"));
    assert_eq!(
        d.described.glyph.as_ref().map(|(g, _)| g),
        Some(&Glyph::Swatches(Some("Tints".into())))
    );
    let stain = Stain::new(example_graph(text)).unwrap_or_else(|e| panic!("{e}"));
    assemble::assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
}

/// The example with its glyph replaced by `glyph`.
fn with_glyph(glyph: &str) -> String {
    EXAMPLE.replace(
        "// @glyph swatches \"Tints\"",
        &format!("// @glyph {glyph}"),
    )
}

#[test]
fn declarations_worked_example_uses_every_control_and_glyph() {
    check_example(EXAMPLE);
    for glyph in ["name", "dropdown", "swatches", "ramp", "gradient", "none"] {
        let text = with_glyph(glyph);
        assert!(
            Declaration::parse(&text).is_ok(),
            "the example does not take the glyph `{glyph}`"
        );
    }
}

negative_control!(
    declarations_worked_example_uses_every_control_and_glyph,
    "the example without its angle control misses a control",
    expected = "the example has no `angle` control",
    check_example(&EXAMPLE.replace("// @control hue angle \"Hue turn\"\n", ""))
);

// ── Not code ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// The source `text` with every line `declare` reads removed.
fn bare(text: &str) -> String {
    text.lines()
        .filter(|l| {
            !assemble::declare::WORDS
                .iter()
                .any(|w| l.trim_start().starts_with(&format!("// @{w}")))
        })
        .map(|l| format!("{l}\n"))
        .collect()
}

/// The example with and without its descriptive lines: one fragment key, one uniform layout and the same code once the
/// comments are dropped; the canonical text of the bare one is the text it had.
fn check_not_code(with: &str, without: &str) {
    let a = Stain::new(example_graph(with)).unwrap_or_else(|e| panic!("{e}"));
    let b = Stain::new(example_graph(without)).unwrap_or_else(|e| panic!("{e}"));
    let fa = assemble::assemble(&a, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let fb = assemble::assemble(&b, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let code = |s: &str| -> Vec<String> {
        s.lines()
            .map(|l| {
                l.split_once("//")
                    .map_or(l, |(c, _)| c)
                    .trim_end()
                    .to_owned()
            })
            .filter(|l| !l.is_empty())
            .collect()
    };
    assert!(
        fa.uniforms == fb.uniforms
            && code(&fa.source) == code(&fb.source)
            && a.canonical().fragment_key() == b.canonical().fragment_key(),
        "the descriptive lines changed the code, the uniforms or the key"
    );
}

#[test]
fn declarations_are_not_code() {
    check_not_code(EXAMPLE, &bare(EXAMPLE));
    // A label edit keeps the key: nothing recompiles.
    check_not_code(EXAMPLE, &EXAMPLE.replace("\"Strength\"", "\"Amount\""));
    assert_eq!(assemble::key_text(&bare(EXAMPLE)), bare(EXAMPLE));
}

negative_control!(
    declarations_are_not_code,
    "a changed uniform default is code",
    expected = "the descriptive lines changed",
    check_not_code(
        EXAMPLE,
        &bare(EXAMPLE).replace("strength: f32 = 0.5", "strength: f32 = 0.25")
    )
);

// ── Every shipped source names itself ─────────────────────────────────────────────────────────────────────────────

fn sim_key(name: &str) -> Option<f64> {
    matches!(name, "eps_E" | "eps_L").then_some(1.0 / 1_048_576.0)
}

/// Every source the app ships, by where it is from: the registry's files, the built-in sources' fields, the default
/// field ramps in either slot with the lightness's post, and the zero source.
fn shipped() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for e in registry::registry().unwrap_or_else(|e| panic!("{e}")) {
        out.push((e.id.clone(), e.source.clone()));
    }
    for f in assemble::source_fields().unwrap_or_else(|e| panic!("{e}")) {
        let text = assemble::occupant_source(Kind::Source, &Occupant::Field(f.clone()))
            .unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_default();
        out.push((format!("field {f}"), text));
    }
    let l = ledger::payload::ledger();
    let resolve = Resolve { sim_key: &sim_key };
    for e in ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}")) {
        for slot in [Slot::Colour, Slot::Brightness] {
            if let Ok(ramp) = defaults::field_ramp(&e, slot, &resolve) {
                out.push((format!("ramp {} {:?}", e.name, slot), ramp.wgsl()));
                if slot == Slot::Brightness {
                    out.push((format!("styles {}", e.name), ramp.style_post()));
                }
            }
        }
    }
    out.push(("zero source".into(), ZERO_SOURCE.to_owned()));
    out
}

fn check_named(sources: &[(String, String)]) {
    assert!(sources.len() > 100, "only {} sources", sources.len());
    for (from, text) in sources {
        let d = Declaration::parse(text).unwrap_or_else(|e| panic!("{from}: {e}"));
        let n = &d.described;
        assert!(
            n.name.is_some()
                && n.group.is_some()
                && n.about.is_some()
                && n.maps.is_some()
                && n.glyph.is_some(),
            "{from} leaves its name, group, about, maps or glyph to a default"
        );
    }
}

#[test]
fn declarations_every_shipped_source_names_itself() {
    check_named(&shipped());
}

negative_control!(
    declarations_every_shipped_source_names_itself,
    "the outcome palette without its name falls back to a default",
    expected = "leaves its name, group",
    check_named(
        &shipped()
            .into_iter()
            .map(|(f, t)| (f, t.replace("// @name Outcome palette\n", "")))
            .collect::<Vec<_>>()
    )
);

// ── The outcome palette's swatches ────────────────────────────────────────────────────────────────────────────────

/// The colour controls of `group` in the outcome palette's source, in order: each its uniform and label.
fn swatch_controls(text: &str, group: &str) -> Vec<(String, String)> {
    let d = Declaration::parse(text).unwrap_or_else(|e| panic!("{e}"));
    d.described
        .controls
        .iter()
        .filter(|c| c.control == Control::Colour && c.group.as_deref() == Some(group))
        .map(|c| (c.uniform.clone(), c.label.clone()))
        .collect()
}

/// The declared sets are the CPU twin's: each swatch's param and its class's display name, in the table's order.
fn check_swatches(text: &str) {
    for (group, set) in [
        ("sRGB palette", &outcome::SWATCHES),
        ("OKLab-uniform", &outcome::OKLAB_UNIFORM),
    ] {
        let want: Vec<(String, String)> = set
            .iter()
            .map(|s| (s.param.to_owned(), s.label.to_owned()))
            .collect();
        assert_eq!(
            swatch_controls(text, group),
            want,
            "the {group} controls are not the twin's swatches"
        );
    }
}

#[test]
fn declarations_outcome_swatches_are_its_colour_controls() {
    check_swatches(outcome::WGSL);
}

negative_control!(
    declarations_outcome_swatches_are_its_colour_controls,
    "two swatches' labels swapped",
    expected = "controls are not the twin's swatches",
    check_swatches(
        &outcome::WGSL
            .replace("\"collision 1–2\" in \"sRGB", "\"SWAP\" in \"sRGB")
            .replace("\"collision 1–3\" in \"sRGB", "\"collision 1–2\" in \"sRGB")
    )
);

// ── A declaration is a real line comment ──────────────────────────────────────────────────────────────────────────

/// As WGSL this returns green: the block comment closes at the `*/` on the `// @name` line, which is that comment's
/// text, not a declaration.
const IN_BLOCK: &str = "fn colour(ctx: Ctx) -> vec3<f32> {
    var c = vec3<f32>(1.0, 0.0, 0.0);
    /* see the note
    // @name x */
    c = vec3<f32>(0.0, 1.0, 0.0); // */
    return c;
}
";

/// The source with a declaration-looking line inside a block comment parses with nothing declared, and assembles with
/// that line, and so the green assignment after it, as written.
fn check_block_comment(text: &str) {
    let d = Declaration::parse(text).unwrap_or_else(|e| panic!("{e}"));
    let stain = Stain::new(example_graph(text)).unwrap_or_else(|e| panic!("{e}"));
    let f = assemble::assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        d.described.name.is_none()
            && f.source
                .contains("    // @name x */\n    c = vec3<f32>(0.0, 1.0, 0.0); // */")
            && assemble::key_text(text) == text,
        "a line inside a block comment was read as a declaration and blanked"
    );
}

#[test]
fn declarations_inside_a_block_comment_are_its_text() {
    check_block_comment(IN_BLOCK);
}

negative_control!(
    declarations_inside_a_block_comment_are_its_text,
    "every line read by its own text alone, as before",
    expected = "was read as a declaration and blanked",
    assemble::declare::probe::without(assemble::declare::Check::Comment, || check_block_comment(
        IN_BLOCK
    ))
);

/// With each of WGSL's line breaks `brk` in place of the example's: the descriptive lines are read and removed from
/// the key text, which is the bare text's.
fn check_breaks(breaks: &[&str]) {
    for brk in breaks {
        let with = EXAMPLE.replace('\n', brk);
        let without = bare(EXAMPLE).replace('\n', brk);
        let d = Declaration::parse(&with).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            d.described.name.as_deref() == Some("Banded tint")
                && assemble::key_text(&with) == without,
            "with the line break {brk:?} the declarations are not read as lines"
        );
    }
}

#[test]
fn declarations_read_every_wgsl_line_break() {
    check_breaks(&["\r\n", "\r", "\u{2028}", "\u{2029}", "\u{85}"]);
}

negative_control!(
    declarations_read_every_wgsl_line_break,
    "an invisible separator, which is no line break",
    expected = "are not read as lines",
    check_breaks(&["\u{2063}"])
);
