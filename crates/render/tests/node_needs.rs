//! What a node needs from the frame, read from its source (node SDK gap 7), whether its picture is a function of the
//! sample alone (gap 8), and what the assembler wraps (gap 9, the structure; the pictures are in `what_matters.rs`).
//!
//! - Every shipped source's derived needs are a list written by hand (`derived_needs_match_the_hand_list`).
//! - The derivation follows what reaches the result, not what the text names, on fixtures written to be hard
//!   (`needs_follow_what_reaches_the_result`, `purity_follows_what_reaches_the_result`).
//! - Which nodes the assembler wraps in the standard treatment (`the_assembler_wraps_what_does_not_handle_validity`).
//!
//! Each property has a control arm: the proof broken one way, or the wrapper removed.

use std::collections::BTreeMap;

#[cfg(feature = "controls")]
use render::assemble::matters::Forget;
use render::assemble::matters::{self, Needs};
use render::assemble::{self, Kind, Node, Occupant, Stain, Tier, View};
use render::registry::{self, ZERO};
use validation::negative_control;

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

fn needs_of(kind: Kind, text: &str) -> Needs {
    matters::shader(kind, text)
        .unwrap_or_else(|e| panic!("{e}"))
        .needs()
}

fn colour(body: &str) -> Needs {
    needs_of(
        Kind::Colour,
        &format!("fn colour(ctx: Ctx) -> vec3<f32> {{ {body} }}"),
    )
}

// ── The shipped sources, against a list written by hand ───────────────────────────────────────────────────────────

/// What each shipped source needs, from reading its file: the words it asks for, as `ftle`, `word`, `range`, `time`,
/// `quad` and `pure`. A view of a field that can be non-finite draws its invalid pixels by the pixel's place
/// (`debug_invalid(ctx.frag_xy)`), so it is not pure; a view of a field that cannot, a flag or a count, is. The
/// registry ids not named here need nothing and are not pure.
fn hand_list() -> BTreeMap<&'static str, &'static [&'static str]> {
    let mut m: BTreeMap<&'static str, &'static [&'static str]> = BTreeMap::new();
    for id in [
        "colour/margin",
        "combiner/multiply",
        "combiner/pass_through",
        "combiner/replace_l",
        "source/field",
        "source/zero",
        "debug/derived/retrograde",
        "debug/derived/total_substeps_log2",
        "debug/generated/budget_exhausted",
        "debug/generated/departed",
        "debug/generated/detail",
        "debug/generated/saturated",
        "debug/generated/state",
    ] {
        m.insert(id, &["pure"]);
    }
    // The generated views of a field with a measured range, which the host writes into the view's `u_range`.
    for id in [
        "C_ty",
        "E_0",
        "I_max",
        "K_0",
        "Lz_0",
        "S",
        "V_0",
        "cap_hits",
        "closure_min",
        "closure_step",
        "coll_floor",
        "dE_max",
        "dE_reg_final",
        "dE_reg_max",
        "dLz_max",
        "d_min",
        "diffusion",
        "floor_hits",
        "lambda_mag",
        "m0",
        "m1",
        "m2",
        "mean_y",
        "q_mass",
        "r_min_pair_0",
        "rho_angle",
        "rho_mag",
        "rho_ratio",
        "theta",
        "total_substeps",
        "virial_ratio",
    ] {
        m.insert(leak(format!("debug/generated/{id}")), &["range"]);
    }
    m.insert("debug/generated/ftle", &["ftle", "range"]);
    m.insert("debug/derived/ftle", &["ftle"]);
    m.insert("debug/generated/length", &["word", "range"]);
    m.insert("debug/generated/payload", &["word", "range"]);
    m.insert("debug/generated/last_symbol", &["word"]);
    m.insert("debug/word/hash", &["word"]);
    m.insert("debug/word/reduced_length", &["word"]);
    m.insert("debug/word/symbol_at_k", &["word"]);
    m.insert("debug/word/truncated", &["word", "pure"]);
    m.insert("debug/generated/t_dmin_step", &["range", "time"]);
    m.insert("debug/generated/t_end_step", &["range", "time"]);
    m.insert("debug/accumulators/diffusion_slope", &["time"]);
    m.insert("debug/accumulators/ftle_running", &["time"]);
    for id in [
        "s_ancestor_gap",
        "s_cache_age",
        "s_coherence",
        "s_depth",
        "s_impurity",
        "s_priority",
        "s_spread",
        "s_suspect",
    ] {
        m.insert(leak(format!("debug/{id}")), &["range", "quad"]);
    }
    m.insert("debug/s_state", &["quad"]);
    for id in ["edge_line", "fallback_tint", "pending_hatch"] {
        m.insert(leak(format!("post/{id}")), &["quad"]);
    }
    // A field ramp as the shipped file holds it, on the field it defaults to, the FTLE with its measured range; each
    // field's own ramp needs what that field is.
    m.insert("colour/field_ramp", &["ftle", "range"]);
    m.insert("brightness/field_ramp", &["ftle", "range"]);
    m.insert("post/field_ramp", &["ftle"]);
    m
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// A source's needs as the words the hand list uses.
fn words(n: &Needs) -> Vec<&'static str> {
    [
        ("ftle", n.ftle),
        ("spread", n.spread),
        ("word", n.word),
        ("range", n.measured_range),
        ("time", n.time),
        ("previous", n.previous_class),
        ("quad", n.quad),
        ("pure", n.pure_in_n),
    ]
    .into_iter()
    .filter(|(_, on)| *on)
    .map(|(w, _)| w)
    .collect()
}

fn check_hand_list(planted: Option<(&str, &'static [&'static str])>) {
    let mut list = hand_list();
    if let Some((id, w)) = planted {
        list.insert(leak(id.to_owned()), w);
    }
    let mut wrong = Vec::new();
    let entries = registry::registry().unwrap_or_else(|e| panic!("{e}"));
    for e in &entries {
        let got = words(&needs_of(e.slot, &e.source));
        let want = list.get(e.id.as_str()).copied().unwrap_or(&[]);
        if got != want {
            wrong.push(format!("{}: derived {got:?}, listed {want:?}", e.id));
        }
    }
    for id in list.keys() {
        if !entries.iter().any(|e| e.id == *id) {
            wrong.push(format!("{id} is listed and no shipped source"));
        }
    }
    // The source of each read-side field needs what the field is: the FTLE, the spread, the step times.
    for f in assemble::source_fields().unwrap_or_else(|e| panic!("{e}")) {
        let text = assemble::occupant_source(Kind::Source, &Occupant::Field(f.clone()))
            .unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_default();
        let n = needs_of(Kind::Source, &text);
        let want = match f.as_str() {
            "ftle" | "ftle_valid" => vec!["ftle", "pure"],
            "ensemble_spread" => vec!["spread", "pure"],
            "t_end_step" | "t_dmin_step" | "t_end_fraction" | "t_dmin_fraction" => {
                vec!["time", "pure"]
            }
            _ => vec!["pure"],
        };
        if words(&n) != want {
            wrong.push(format!(
                "source {f}: derived {:?}, listed {want:?}",
                words(&n)
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "derived needs differ from the hand list: {}",
        wrong.join("; ")
    );
}

#[test]
fn derived_needs_match_the_hand_list() {
    // Pass criterion, written before the first run: every shipped source's derived needs are the hand list's entry for
    // it, the empty one where it has none; no listed id is missing from the registry.
    check_hand_list(None);
}

negative_control!(
    derived_needs_match_the_hand_list,
    "the hand list claims the outcome palette needs the FTLE",
    expected = "derived needs differ from the hand list",
    check_hand_list(Some(("colour/outcome_state", &["ftle"])))
);

// ── Fixtures written to be hard ───────────────────────────────────────────────────────────────────────────────────

/// (name, a colour's body, the words it needs): a read the result depends on only through a branch, a read into
/// nothing, and a read through a pointer.
const NEEDS: [(&str, &str, &[&str]); 7] = [
    (
        "ftle",
        "return vec3<f32>(ctx.sample.ftle);",
        &["ftle", "pure"],
    ),
    (
        "ftle only in a branch",
        "if (ctx.sample.ftle > 0.0) { return vec3<f32>(1.0); } return vec3<f32>(0.0);",
        &["ftle", "pure"],
    ),
    (
        "ftle read into nothing",
        "let x = ctx.sample.ftle; return vec3<f32>(0.5);",
        &["pure"],
    ),
    (
        "the spread and the word",
        "return vec3<f32>(ctx.sample.ensemble_spread, f32(ctx.sample.word.x), 0.0);",
        &["spread", "word", "pure"],
    ),
    ("the quad", "return vec3<f32>(ctx.quad.uv, 0.0);", &["quad"]),
    (
        "a step time",
        "return vec3<f32>(f32(ctx.sample.t_end_step));",
        &["time", "pure"],
    ),
    ("nothing", "return vec3<f32>(0.5);", &["pure"]),
];

fn check_needs() {
    for (name, body, want) in NEEDS {
        let got = words(&colour(body));
        assert_eq!(got, want, "{name}: the needs the proof reads from the code");
    }
}

#[test]
fn needs_follow_what_reaches_the_result() {
    // Pass criterion: as `NEEDS` says for each colour. A field named by the text and read into nothing is not needed;
    // one that only decides a branch is.
    check_needs();
}

negative_control!(
    needs_follow_what_reaches_the_result,
    "a proof that forgets branches does not find the FTLE that only decides a branch",
    expected = "the needs the proof reads from the code",
    matters::probe::forgetting(Forget::Branches, check_needs)
);

/// (name, a colour's body, whether it is pure in n̂).
const PURITY: [(&str, &str, bool); 6] = [
    ("the sample", "return vec3<f32>(ctx.sample.d_min);", true),
    ("the pixel", "return vec3<f32>(ctx.frag_xy.x);", false),
    ("the screen", "return vec3<f32>(ctx.screen.uv, 0.0);", false),
    (
        "the chart's classification",
        "return vec3<f32>(select(0.0, 1.0, ctx.validity.forbidden));",
        false,
    ),
    (
        "the pixel into nothing",
        "let x = ctx.frag_xy.y; return vec3<f32>(ctx.sample.d_min);",
        true,
    ),
    (
        "the pixel only in a branch",
        "if (ctx.frag_xy.x > 3.0) { return vec3<f32>(1.0); } return vec3<f32>(0.0);",
        false,
    ),
];

fn check_purity() {
    for (name, body, pure) in PURITY {
        assert_eq!(
            colour(body).pure_in_n,
            pure,
            "{name}: whether the picture is a function of n̂"
        );
    }
}

#[test]
fn purity_follows_what_reaches_the_result() {
    // Pass criterion: as `PURITY` says for each colour.
    check_purity();
}

negative_control!(
    purity_follows_what_reaches_the_result,
    "a proof that forgets branches calls a colour that branches on the pixel pure",
    expected = "whether the picture is a function of n",
    matters::probe::forgetting(Forget::Branches, check_purity)
);

// ── What the assembler wraps ──────────────────────────────────────────────────────────────────────────────────────

const PLAIN: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.sample.d_min); }";
const HANDLED: &str =
    "fn colour(ctx: Ctx) -> vec3<f32> { handles_validity(); return vec3<f32>(ctx.sample.d_min); }";
const COMMENTED: &str = "// handles_validity();\n/* handles_validity(); */\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.sample.d_min); }";
const LIGHT: &str = "fn brightness(ctx: Ctx) -> f32 { return ctx.sample.d_min; }";

fn source_of(colour: &str, brightness: Option<&str>, view: View) -> String {
    let mut g = vec![
        node(Kind::Source, Occupant::BuiltIn(ZERO.to_owned()), &[]),
        node(Kind::Colour, Occupant::Custom(colour.into()), &[Some(0)]),
    ];
    let mut combiner = vec![Some(1), None];
    if let Some(b) = brightness {
        g.push(node(
            Kind::Brightness,
            Occupant::Custom(b.into()),
            &[Some(0)],
        ));
        combiner[1] = Some(2);
    }
    let pass = Occupant::BuiltIn("pass_through".into());
    g.push(node(Kind::Combiner, pass, &combiner));
    let at = g.len() - 1;
    g.push(node(Kind::Out, Occupant::None, &[Some(at)]));
    let stain = Stain::new(g).unwrap_or_else(|e| panic!("{e}"));
    assemble::assemble_kept(&stain, Tier::FULL, view)
        .unwrap_or_else(|e| panic!("{e}"))
        .source
}

fn check_wrapping() {
    let wrapped = |s: &str| s.contains("_standard(c");
    assert!(
        wrapped(&source_of(PLAIN, None, View::Presentation)),
        "a colour with no validity code is wrapped"
    );
    assert!(
        wrapped(&source_of(PLAIN, Some(LIGHT), View::Presentation))
            && source_of(PLAIN, Some(LIGHT), View::Presentation).contains("n2_standard(c2)"),
        "a brightness with no validity code is wrapped"
    );
    assert!(
        wrapped(&source_of(COMMENTED, None, View::Presentation)),
        "a comment that names the marker is not the marker"
    );
    assert!(
        !wrapped(&source_of(HANDLED, None, View::Presentation)),
        "a colour that handles validity itself is left alone"
    );
    assert!(
        !wrapped(&source_of(PLAIN, Some(LIGHT), View::Debug)),
        "the debug view's arm is not wrapped"
    );
}

#[test]
fn the_assembler_wraps_what_does_not_handle_validity() {
    // Pass criterion: the presentation view's source wraps a plain colour and a plain brightness, and a colour whose
    // comments only name the marker; it does not wrap a colour that calls it, nor either in the debug view.
    check_wrapping();
}

negative_control!(
    the_assembler_wraps_what_does_not_handle_validity,
    "the check with the wrapper removed finds a plain colour unwrapped",
    expected = "a colour with no validity code is wrapped",
    assemble::probe::without_standard_treatment(check_wrapping)
);
