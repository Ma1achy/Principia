//! The shared prelude's generation (render_gui_spec §10.1; lowering Part 3a; R-145), `ledger::gen::prelude`:
//! - REQ-RENDER-014: the prelude bakes `has_ftle` and `has_word` as consts whose values are the variant's tier bits,
//!   and reads `has_ensemble` from a uniform; toggling E between 0 and 3 leaves the fragment variant unchanged, one
//!   compiled pipeline serving both, and E = 0 reads `ensemble_spread` as the canonical quiet NaN
//!   (`prelude_has_consts_*`).
//! - The checked-in prelude is the emitters' output, and the published tables it embeds parse whole
//!   (`prelude_checked_in_*`, `prelude_luts_*`).
//!
//! The GPU check draws the assembled fragment variant (the unpack layer, the read side, the prelude) once and reads it
//! at each E; its control runs the same check with one change and shows it fails (pitfalls §9).

use std::path::Path;

use ledger::gen::prelude::{self, Lut};
use ledger::gen::read::{self, Tier};
use ledger::gen::{self, wgsl};
use naga::{AddressSpace, Expression, Literal, Module, TypeInner};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

/// Lowering Part 3a's canonical quiet NaN (R-72; REQ-RENDER-077).
const QNAN: u32 = 0x7fc0_0000;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn parse(source: &str) -> Module {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    module
}

/// The value of the bool constant `name` in `module`, or `None` if it declares none.
fn bool_const(module: &Module, name: &str) -> Option<bool> {
    let (_, c) = module
        .constants
        .iter()
        .find(|(_, c)| c.name.as_deref() == Some(name))?;
    match module.global_expressions[c.init] {
        Expression::Literal(Literal::Bool(b)) => Some(b),
        ref other => panic!("`{name}` is not a bool literal: {other:?}"),
    }
}

/// The element type's name of the storage global `name`, or `None` if `module` declares none.
fn buffer_element(module: &Module, name: &str) -> Option<String> {
    let (_, g) = module
        .global_variables
        .iter()
        .find(|(_, g)| g.name.as_deref() == Some(name))?;
    match module.types[g.ty].inner {
        TypeInner::Array { base, .. } => Some(module.types[base].name.clone().unwrap_or_default()),
        ref other => panic!("`{name}` is not an array: {other:?}"),
    }
}

// ── prelude_has_consts (REQ-RENDER-014) ───────────────────────────────────────────────────────────────────────────

/// At every tier, the prelude `prelude_at(tier)` bakes `has_ftle` and `has_word` with the tier's bits, and they agree
/// with the variant the unpack layer binds at that tier: `SimStateFTLE` exactly when `has_ftle`, the word buffer
/// exactly when `has_word` (lowering Part 3a).
fn check_tier_consts(prelude_at: fn(Tier) -> String) {
    let words = &ledger::layout().words;
    let entries = gen::validate(&ledger::layout()).expect("the layout validates");
    for tier in Tier::ALL {
        let p = parse(&prelude_at(tier));
        assert_eq!(
            (bool_const(&p, "has_ftle"), bool_const(&p, "has_word")),
            (Some(tier.has_ftle), Some(tier.has_word)),
            "the prelude's has_ftle and has_word at {tier:?}"
        );
        let layer = parse(&wgsl::layer(words, &entries, tier));
        let variant = buffer_element(&layer, "simstate_buffer");
        assert_eq!(
            variant.as_deref() == Some("SimStateFTLE"),
            tier.has_ftle,
            "the variant bound at {tier:?} is {variant:?}"
        );
        assert_eq!(
            buffer_element(&layer, "word_buffer").is_some(),
            tier.has_word,
            "the word buffer's binding at {tier:?}"
        );
    }
}

#[test]
fn prelude_has_consts_match_the_tier_bits() {
    check_tier_consts(prelude::wgsl);
}

negative_control!(
    prelude_has_consts_match_the_tier_bits,
    "a prelude baking has_ftle true at every tier must fail",
    expected = "the prelude's has_ftle and has_word",
    check_tier_consts(|tier| prelude::wgsl(tier).replace(
        "const has_ftle: bool = false;",
        "const has_ftle: bool = true;"
    ))
);

/// The prelude `text` declares no `has_ensemble` constant, binds `PreludeUniforms` as a uniform at
/// [`prelude::uniforms_binding`], its first member `has_ensemble`, and declares `fn has_ensemble() -> bool` (R-145).
fn check_ensemble_uniform(text: &str) {
    let p = parse(text);
    assert_eq!(
        bool_const(&p, "has_ensemble"),
        None,
        "has_ensemble is baked as a constant"
    );
    let (_, g) = p
        .global_variables
        .iter()
        .find(|(_, g)| g.name.as_deref() == Some("prelude_uniforms"))
        .expect("the prelude declares prelude_uniforms");
    assert_eq!(
        g.space,
        AddressSpace::Uniform,
        "prelude_uniforms is not a uniform"
    );
    let b = prelude::uniforms_binding();
    let at = g.binding.as_ref().expect("prelude_uniforms has a binding");
    assert_eq!(
        (at.group, at.binding),
        (b.group, b.binding),
        "prelude_uniforms' binding"
    );
    match &p.types[g.ty].inner {
        TypeInner::Struct { members, span } => {
            assert_eq!(
                members[0].name.as_deref(),
                Some("has_ensemble"),
                "the uniform's first member"
            );
            assert_eq!(*span, 16, "the uniform block's size");
        }
        other => panic!("prelude_uniforms is not a struct: {other:?}"),
    }
    let f = p
        .functions
        .iter()
        .find(|(_, f)| f.name.as_deref() == Some("has_ensemble"))
        .map(|(_, f)| f)
        .expect("the prelude declares fn has_ensemble");
    let returns = f.result.as_ref().map(|r| &p.types[r.ty].inner);
    assert!(
        matches!(returns, Some(TypeInner::Scalar(s)) if s.kind == naga::ScalarKind::Bool),
        "has_ensemble() does not return bool"
    );
    assert_eq!(
        b.group, 0,
        "the prelude's uniforms are not in group 0, the per-frame uniforms (R-343)"
    );
}

#[test]
fn prelude_has_consts_ensemble_is_a_uniform() {
    for tier in Tier::ALL {
        check_ensemble_uniform(&prelude::wgsl(tier));
    }
}

negative_control!(
    prelude_has_consts_ensemble_is_a_uniform,
    "a prelude holding has_ensemble in private memory must fail",
    expected = "prelude_uniforms is not a uniform",
    check_ensemble_uniform(&prelude::wgsl(Tier::FULL).replace(
        "@group(0) @binding(0) var<uniform> prelude_uniforms",
        "var<private> prelude_uniforms"
    ))
);

/// The test entry: one pixel, the read side's `ensemble_spread` filled from `spread_in[0]` with the prelude's
/// `has_ensemble()`, then its bits, `has_ensemble()`, `has_ftle` and `has_word`.
const ENSEMBLE_ENTRY: &str = r"
@group(2) @binding(0) var<storage, read> spread_in: array<f32>;

@fragment
fn t_ensemble(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let params = ReadParams(0.01, 1e-6, 16u, 1000u);
    let s = sample_read(0u, spread_in[0], has_ensemble(), vec3<f32>(1.0, 1.0, 1.0), params);
    return vec4<u32>(bitcast<u32>(s.ensemble_spread), u32(has_ensemble()), u32(has_ftle), u32(has_word));
}
";

/// At every tier: the fragment variant, the unpack layer, the read side filling `ensemble_spread` and the prelude, is
/// assembled and compiled once, then drawn at E = 0, 1 and 3 with the uniform words `words(E)`: at E = 0
/// `ensemble_spread` reads the canonical quiet NaN's bits and `has_ensemble()` is false; at E ≥ 1 it reads the spread
/// given and `has_ensemble()` is true; `has_ftle` and `has_word` are the tier's throughout. Nothing is re-baked:
/// one pipeline serves every E.
fn check_e_toggle(gpu: &GpuHarness, words: fn(u32) -> [u32; 4]) {
    let layout = ledger::layout();
    let entries = gen::validate(&layout).expect("the layout validates");
    let spread = 0.25f32;
    for tier in Tier::ALL {
        let variant = format!(
            "{}{}{}",
            read::assemble(&layout.words, &entries, tier, &["ensemble_spread"]).expect("assembles"),
            prelude::wgsl(tier),
            ENSEMBLE_ENTRY
        );
        parse(&variant);
        let kernel = gpu
            .fragment(
                &variant,
                "t_ensemble",
                &[&[BindingKind::Uniform], &[], &[BindingKind::Storage]],
                1,
                1,
            )
            .unwrap_or_else(|e| panic!("{e}"));
        for e in [0u32, 1, 3] {
            let uniform = words(e);
            let input = [spread.to_bits()];
            let got = kernel
                .draw(&[&[&uniform], &[], &[&input]])
                .unwrap_or_else(|e| panic!("{e}"))[0];
            let want_spread = if e == 0 { QNAN } else { spread.to_bits() };
            assert_eq!(
                got[0], want_spread,
                "at {tier:?}, E = {e} reads ensemble_spread as {:#010x}, not {want_spread:#010x}",
                got[0]
            );
            assert_eq!(
                [got[1], got[2], got[3]],
                [
                    u32::from(e >= 1),
                    u32::from(tier.has_ftle),
                    u32::from(tier.has_word)
                ],
                "at {tier:?}, E = {e}: has_ensemble(), has_ftle and has_word"
            );
        }
    }
}

#[test]
fn prelude_has_consts_toggling_e_never_rebakes() {
    check_e_toggle(
        &GpuHarness::new().expect("a GPU device"),
        prelude::uniform_words,
    );
}

negative_control!(
    prelude_has_consts_toggling_e_never_rebakes,
    "uniform words that keep the ensemble on at E = 0 must not read NaN",
    expected = "E = 0 reads ensemble_spread as",
    check_e_toggle(&GpuHarness::new().expect("a GPU device"), |_| [1, 0, 0, 0])
);

/// The uniform words `words(E)`: `has_ensemble` 1 exactly when E ≥ 1, then three words of padding.
fn check_uniform_words(words: fn(u32) -> [u32; 4]) {
    for (e, want) in [
        (0, [0, 0, 0, 0]),
        (1, [1, 0, 0, 0]),
        (3, [1, 0, 0, 0]),
        (u32::MAX, [1, 0, 0, 0]),
    ] {
        assert_eq!(words(e), want, "the prelude's uniform words at E = {e}");
    }
}

#[test]
fn prelude_has_consts_uniform_words_follow_e() {
    check_uniform_words(prelude::uniform_words);
}

negative_control!(
    prelude_has_consts_uniform_words_follow_e,
    "an ensemble on only above E = 1 must fail",
    expected = "the prelude's uniform words at E = 1",
    check_uniform_words(|e| [u32::from(e > 1), 0, 0, 0])
);

// ── the checked-in prelude and its tables ─────────────────────────────────────────────────────────────────────────

/// `on_disk` is the emitters' prelude, at `prelude::PATH`, the full tier's.
fn check_checked_in(on_disk: &str) {
    let files = gen::generate(&ledger::layout(), gen::EMITTERS).expect("the layout generates");
    let emitted = files
        .iter()
        .find(|g| g.path.ends_with(prelude::PATH))
        .expect("the emitters write the prelude");
    assert!(
        emitted.contents == on_disk && on_disk == prelude::wgsl(Tier::FULL),
        "the checked-in prelude is not the emitters' full-tier output: run `cargo xtask codegen`"
    );
}

fn checked_in() -> String {
    let path = root().join(prelude::PATH);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn prelude_checked_in_is_the_emitters_output() {
    check_checked_in(&checked_in());
}

negative_control!(
    prelude_checked_in_is_the_emitters_output,
    "a prelude edited by hand must fail",
    expected = "is not the emitters' full-tier output",
    check_checked_in(&checked_in().replace("0.12", "0.13"))
);

/// The prelude parses with naga at every tier, and the hatch and hue wheel it writes are the ledger's.
fn check_parses_with_values(prelude_at: fn(Tier) -> String) {
    let h = prelude::hatch();
    let (l, c) = prelude::hue_wheel_lc();
    for tier in Tier::ALL {
        let text = prelude_at(tier);
        parse(&text);
        for (k, colour) in h.colours.iter().enumerate() {
            let rgb = format!(
                "vec3<f32>({}.0, {}.0, {}.0) / 255.0",
                colour[0], colour[1], colour[2]
            );
            assert!(
                text.contains(&rgb),
                "the prelude does not write hatch colour {k}, {rgb}"
            );
        }
        let shift = format!(">> {}u)", h.half_period.trailing_zeros());
        assert!(
            text.contains(&shift),
            "the prelude's stripe width is not {} px",
            h.half_period
        );
        let wheel = format!("oklch_to_linear({l}, {c}, t)");
        assert!(
            text.contains(&wheel),
            "the prelude's hue wheel is not {wheel}"
        );
    }
}

#[test]
fn prelude_checked_in_parses_and_writes_the_ledgers_values() {
    check_parses_with_values(prelude::wgsl);
}

negative_control!(
    prelude_checked_in_parses_and_writes_the_ledgers_values,
    "a prelude with another stripe width must fail",
    expected = "the prelude's stripe width",
    check_parses_with_values(|tier| prelude::wgsl(tier).replace(">> 2u)", ">> 3u)"))
);

/// The published tables (R-122): `luts` gives viridis, twilight and magma, 256, 510 and 256 stops, each parsed whole,
/// with matplotlib's first and last stops; the WGSL constant holds each stop's literal as the source writes it.
fn check_tables(luts: [Lut; 3]) {
    let [viridis, twilight, magma] = luts;
    assert_eq!(
        (viridis.name, twilight.name, magma.name),
        ("viridis", "twilight", "magma"),
        "the tables"
    );
    let want = [
        (
            viridis,
            256,
            [0.267004, 0.004874, 0.329415],
            [0.993248, 0.906157, 0.143936],
        ),
        (
            twilight,
            510,
            [0.8857501584075443, 0.8500092494306783, 0.8879736506427196],
            [0.8857115512284565, 0.8500218611585632, 0.8857253899008712],
        ),
        (
            magma,
            256,
            [0.001462, 0.000466, 0.013866],
            [0.987053, 0.991438, 0.749504],
        ),
    ];
    let text = prelude::wgsl(Tier::FULL);
    for (lut, len, first, last) in want {
        let stops = lut.stops();
        assert_eq!(stops.len(), len, "{}: the stop count", lut.name);
        assert_eq!(
            (stops[0], stops[len - 1]),
            (first, last),
            "{}: the first and last stops",
            lut.name
        );
        assert!(
            stops.iter().flatten().all(|x| (0.0..=1.0).contains(x)),
            "{}: a stop outside [0, 1]",
            lut.name
        );
        for s in lut.literals() {
            let line = format!("    vec3<f32>({}, {}, {}),", s[0], s[1], s[2]);
            assert!(
                text.contains(&line),
                "{}: the prelude lacks the stop {line}",
                lut.name
            );
        }
    }
}

#[test]
fn prelude_luts_parse_every_published_stop() {
    check_tables(prelude::luts());
}

negative_control!(
    prelude_luts_parse_every_published_stop,
    "a table missing its last stop must fail",
    expected = "viridis: the stop count",
    check_tables({
        let [mut v, t, m] = prelude::luts();
        let data = v.data;
        let cut = data.trim_end().rfind('\n').expect("a line");
        v.data = Box::leak(data[..cut].to_owned().into_boxed_str());
        [v, t, m]
    })
);

/// A data file's comment lines and blank lines are not stops, and a literal that is not a number reads as NaN, so
/// no stop is dropped silently.
fn check_parse_rules(parse_stops: fn(&Lut) -> Vec<[f64; 3]>) {
    let lut = |data: &'static str| Lut {
        name: "test",
        constant: "LUT_TEST",
        ramp: "ramp_test",
        data,
    };
    let stops = parse_stops(&lut("# source\n\n0.1 0.2 0.3\n  \n0.4 0.5 0.6\n"));
    assert_eq!(
        stops,
        vec![[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]],
        "the stops of a commented file"
    );
    let bad = parse_stops(&lut("0.1 x 0.3\n0.4 0.5\n"));
    assert!(
        bad.len() == 2 && bad[0][1].is_nan() && bad[1][2].is_nan() && bad[1][0] == 0.4,
        "a stop that does not parse is not NaN: {bad:?}"
    );
}

#[test]
fn prelude_luts_parse_rules() {
    check_parse_rules(Lut::stops);
}

negative_control!(
    prelude_luts_parse_rules,
    "a parser that drops bad stops must fail",
    expected = "a stop that does not parse is not NaN",
    check_parse_rules(|l| l
        .stops()
        .into_iter()
        .filter(|s| s.iter().all(|x| !x.is_nan()))
        .collect())
);
