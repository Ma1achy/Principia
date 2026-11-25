//! Declarations do not change pictures: the code every library row assembles to, its comments and blank lines
//! dropped, is the code `tests/data/declared_code.txt` holds, one FNV-1a digest per row and view. The rows are every
//! registry entry in its slot, named by id, every built-in source's field and every field's default ramp in either slot with the
//! brightness ramp's style post. Run with `PRINCIPIA_BLESS=1` to rewrite the file after a change that is meant to
//! change code.

use std::fmt::Write as _;
use std::path::PathBuf;

use render::assemble::{self, Kind, Node, Occupant, Stain, Tier, View};
use render::codegen::Slot;
use render::colour::defaults::{self, Resolve};
use render::registry::{self, ZERO};
use validation::negative_control;

const FIXTURE: &str = "tests/data/declared_code.txt";

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// A node of `kind` and `occupant`, every field input fed by node 0, the zero source; a post's colour by `colour`.
fn fed(kind: Kind, occupant: Occupant, colour: Option<usize>) -> Node {
    let d = assemble::declaration(kind, &occupant).unwrap_or_else(|e| panic!("{e}"));
    let mut inputs: Vec<Option<usize>> = vec![Some(0); d.inputs.len()];
    if kind == Kind::Post {
        inputs.insert(0, colour);
    }
    node(kind, occupant, &inputs)
}

const SHOW: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }";

fn zero() -> Node {
    node(Kind::Source, Occupant::BuiltIn(ZERO.to_owned()), &[])
}

fn pass_through() -> Occupant {
    Occupant::BuiltIn("pass_through".into())
}

/// The graph that shows `occupant` in a node of `kind`: the zero source into it, the rest of the backbone the
/// identity or the pass-through combiner.
fn showing(kind: Kind, occupant: Occupant) -> Vec<Node> {
    let mut g = vec![zero()];
    match kind {
        Kind::Source => {
            g[0] = node(Kind::Source, occupant, &[]);
            g.push(fed(Kind::Colour, Occupant::Custom(SHOW.into()), None));
            g.push(node(Kind::Combiner, pass_through(), &[Some(1), None]));
            g.push(node(Kind::Out, Occupant::None, &[Some(2)]));
        }
        Kind::Colour => {
            g.push(fed(Kind::Colour, occupant, None));
            g.push(node(Kind::Combiner, pass_through(), &[Some(1), None]));
            g.push(node(Kind::Out, Occupant::None, &[Some(2)]));
        }
        Kind::Brightness => {
            g.push(fed(Kind::Brightness, occupant, None));
            g.push(node(Kind::Combiner, pass_through(), &[None, Some(1)]));
            g.push(node(Kind::Out, Occupant::None, &[Some(2)]));
        }
        Kind::Combiner => {
            g.push(fed(Kind::Colour, Occupant::Custom(SHOW.into()), None));
            g.push(node(Kind::Combiner, occupant, &[Some(1), None]));
            g.push(node(Kind::Out, Occupant::None, &[Some(2)]));
        }
        Kind::Post => {
            g.push(fed(Kind::Colour, Occupant::Custom(SHOW.into()), None));
            g.push(node(Kind::Combiner, pass_through(), &[Some(1), None]));
            g.push(fed(Kind::Post, occupant, Some(2)));
            g.push(node(Kind::Out, Occupant::None, &[Some(3)]));
        }
        Kind::Out => unreachable!("OUT has no occupant"),
    }
    g
}

fn sim_key(name: &str) -> Option<f64> {
    matches!(name, "eps_E" | "eps_L").then_some(1.0 / 1_048_576.0)
}

/// Every row, by its name, as a graph.
fn rows() -> Vec<(String, Vec<Node>)> {
    let mut out = Vec::new();
    for e in registry::registry().unwrap_or_else(|e| panic!("{e}")) {
        let occupant = Occupant::BuiltIn(e.name().to_owned());
        out.push((format!("registry {}", e.id), showing(e.slot, occupant)));
    }
    for f in assemble::source_fields().unwrap_or_else(|e| panic!("{e}")) {
        out.push((
            format!("field {f}"),
            showing(Kind::Source, Occupant::Field(f)),
        ));
    }
    let l = ledger::payload::ledger();
    let resolve = Resolve { sim_key: &sim_key };
    for e in ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}")) {
        for slot in [Slot::Colour, Slot::Brightness] {
            let Ok(ramp) = defaults::field_ramp(&e, slot, &resolve) else {
                continue;
            };
            let (kind, occupant) = ramp.occupant();
            let mut g = showing(kind, occupant);
            if slot == Slot::Brightness {
                let out = g.pop().expect("OUT");
                let p = g.len();
                g.push(fed(
                    Kind::Post,
                    Occupant::Custom(ramp.style_post()),
                    out.inputs[0],
                ));
                g.push(node(Kind::Out, Occupant::None, &[Some(p)]));
            }
            out.push((format!("ramp {} {}", e.name, slot.kind().name()), g));
        }
    }
    out
}

/// `source` with every `//` comment, trailing white space and blank line dropped: WGSL has no string literal, so a
/// `//` always opens a comment.
fn code(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let line = line
            .split_once("//")
            .map_or(line, |(code, _)| code)
            .trim_end();
        if !line.trim().is_empty() {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Row `name`'s digest lines, `<view> <row>: <digest>`, with `edit` applied to every custom occupant's text.
fn digest(name: &str, mut g: Vec<Node>, edit: &(dyn Fn(&str) -> String + Sync)) -> String {
    let mut out = String::new();
    for n in &mut g {
        if let Occupant::Custom(text) = &n.occupant {
            n.occupant = Occupant::Custom(edit(text));
        }
    }
    let stain = Stain::new(g).unwrap_or_else(|e| panic!("{name}: {e}"));
    for view in [View::Presentation, View::Debug] {
        let f = assemble::assemble_view(&stain, Tier::FULL, view)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let _ = writeln!(
            out,
            "{view:?} {name}: {:016x}",
            ledger::version::fnv1a64(code(&f.source).as_bytes())
        );
    }
    out
}

/// Every row's digest lines, in the rows' order, the rows assembled on every core.
fn digests(edit: &(dyn Fn(&str) -> String + Sync)) -> String {
    let rows = rows();
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    let chunk = rows.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = rows
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    part.iter()
                        .map(|(name, g)| digest(name, g.clone(), edit))
                        .collect::<String>()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    })
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

fn check_code(got: &str) {
    if std::env::var_os("PRINCIPIA_BLESS").is_some() {
        std::fs::write(fixture(), got).unwrap_or_else(|e| panic!("{e}"));
    }
    let want = std::fs::read_to_string(fixture()).unwrap_or_else(|e| panic!("{e}"));
    let differ: Vec<&str> = got
        .lines()
        .filter(|l| !want.lines().any(|w| w == *l))
        .collect();
    assert!(
        differ.is_empty() && got.lines().count() == want.lines().count(),
        "the assembled code changed: {} row(s) differ, first {:?}; {} rows now, {} stored",
        differ.len(),
        differ.first(),
        got.lines().count(),
        want.lines().count()
    );
}

#[test]
fn declared_code_is_unchanged() {
    check_code(&digests(&|t| t.to_owned()));
}

negative_control!(
    declared_code_is_unchanged,
    "one statement more in every custom occupant changes the code",
    expected = "the assembled code changed",
    check_code(&digests(&|t| format!(
        "{t}\nconst DECLARED_CONTROL: f32 = 1.0;\n"
    )))
);

// ── The blanked text is the text as written ───────────────────────────────────────────────────────────────────────

/// Colour occupants built to break the blanking: declaration-looking lines inside block comments, nested and
/// unclosed, for every word and with every kind of line break, beside the worked example's real declarations.
fn hostile() -> Vec<(String, String)> {
    let in_block = "fn colour(ctx: Ctx) -> vec3<f32> {\n    var c = vec3<f32>(1.0, 0.0, 0.0);\n    /* see the note\n    \
                    // @name x */\n    c = vec3<f32>(0.0, 1.0, 0.0); // */\n    return c;\n}\n";
    let nested = "fn colour(ctx: Ctx) -> vec3<f32> {\n    var c = vec3<f32>(1.0, 0.0, 0.0);\n    /* outer /* inner */\n    \
                  // @control k slider \"K\" */\n    c = vec3<f32>(0.0, 1.0, 0.0); // */\n    return c;\n}\n";
    let declares_in_block = "/*\n// @uniform k: f32 = 1.0\n// @input other\n// @glyph sparkle\n// @version 9\n*/\n\
                             fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }\n";
    let unclosed = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5); }\n/* never closed\n// @name x\n";
    let after_code = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5); } // @name x\n";
    let mut out = vec![
        ("in a block comment".to_owned(), in_block.to_owned()),
        ("in a nested block comment".to_owned(), nested.to_owned()),
        (
            "every word in a block comment".to_owned(),
            declares_in_block.to_owned(),
        ),
        (
            "in an unclosed block comment".to_owned(),
            unclosed.to_owned(),
        ),
        ("after code on its line".to_owned(), after_code.to_owned()),
        (
            "the worked example".to_owned(),
            include_str!("data/every_control.wgsl").to_owned(),
        ),
    ];
    for brk in [
        "\r\n", "\r", "\u{b}", "\u{c}", "\u{85}", "\u{2028}", "\u{2029}",
    ] {
        for (name, text) in [
            ("in a block comment", in_block),
            ("in a nested one", nested),
        ] {
            out.push((
                format!("{name}, broken by {brk:?}"),
                text.replace('\n', brk),
            ));
        }
    }
    out
}

/// `source` as naga's module, written out as SPIR-V, or why naga refused it.
fn module(source: &str) -> Result<Vec<u32>, String> {
    use naga::valid::{Capabilities, ValidationFlags, Validator};
    let m = naga::front::wgsl::parse_str(source).map_err(|e| e.to_string())?;
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&m)
        .map_err(|e| e.to_string())?;
    naga::back::spv::write_vec(&m, &info, &naga::back::spv::Options::default(), None)
        .map_err(|e| e.to_string())
}

/// What `g` assembles to, blanked and as written, as naga's modules; each refusal as one, so a source both refuse
/// agrees.
fn modules(g: &[Node]) -> [Result<Vec<u32>, String>; 2] {
    let stain = match Stain::new(g.to_vec()) {
        Ok(s) => s,
        Err(e) => return [Err(e.to_string()), Err(e.to_string())],
    };
    let blanked = assemble::assemble(&stain, Tier::FULL).map_err(|e| e.to_string());
    let written = assemble::assemble_written(&stain, Tier::FULL).map_err(|e| e.to_string());
    [blanked, written].map(|f| {
        f.and_then(|f| module(&f.source))
            .map_err(|_| "refused".to_owned())
    })
}

/// The rows whose blanked text and text as written naga makes into different modules.
fn differing(rows: &[(String, Vec<Node>)]) -> Vec<String> {
    rows.iter()
        .filter(|(_, g)| {
            let [a, b] = modules(g);
            a != b
        })
        .map(|(name, _)| name.clone())
        .collect()
}

/// For every row and every hostile source, naga's module of the blanked text is the module of the text as written,
/// the rows split over `threads` threads; one runs them on this thread, where a control's probe reaches.
fn check_blanked_is_written(rows: &[(String, Vec<Node>)], threads: usize) {
    let differ: Vec<String> = if threads <= 1 {
        differing(rows)
    } else {
        let chunk = rows.len().div_ceil(threads).max(1);
        std::thread::scope(|s| {
            let handles: Vec<_> = rows
                .chunks(chunk)
                .map(|part| s.spawn(move || differing(part)))
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
                .collect()
        })
    };
    assert!(
        differ.is_empty(),
        "blanking changed naga's module of {} source(s): {differ:?}",
        differ.len()
    );
}

/// The hostile rows of `rows`.
fn hostile_rows(rows: &[(String, Vec<Node>)]) -> impl Iterator<Item = &(String, Vec<Node>)> {
    rows.iter().filter(|(n, _)| n.starts_with("hostile: "))
}

fn every_row() -> Vec<(String, Vec<Node>)> {
    let mut all = rows();
    for (name, text) in hostile() {
        all.push((
            format!("hostile: {name}"),
            showing(Kind::Colour, Occupant::Custom(text)),
        ));
    }
    all
}

#[test]
fn declared_lines_blanked_are_the_text_as_written() {
    let rows = every_row();
    let refused: Vec<&str> = hostile_rows(&rows)
        .filter(|(_, g)| modules(g)[0].is_err())
        .map(|(n, _)| n.as_str())
        .collect();
    eprintln!(
        "{} sources compared as naga modules; refused both ways: {refused:?}",
        rows.len()
    );
    assert!(
        refused.len() <= 1,
        "the hostile sources are refused, so they test nothing: {refused:?}"
    );
    check_blanked_is_written(
        &rows,
        std::thread::available_parallelism().map_or(1, usize::from),
    );
}

negative_control!(
    declared_lines_blanked_are_the_text_as_written,
    "every line read by its own text alone, as before, on the hostile sources that rule accepts",
    expected = "blanking changed naga's module",
    assemble::declare::probe::without(assemble::declare::Check::Comment, || {
        check_blanked_is_written(
            &hostile()
                .into_iter()
                .map(|(n, t)| (n, Occupant::Custom(t)))
                .filter(|(_, o)| assemble::declaration(Kind::Colour, o).is_ok())
                .map(|(n, o)| (n, showing(Kind::Colour, o)))
                .collect::<Vec<_>>(),
            1,
        )
    })
);
