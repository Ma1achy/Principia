//! The outcome palette is read from its source: the CPU image and the legend are the recorded values, pasted as
//! written so nothing below is computed from the palette it checks, and a declared default moves the shader's uniform,
//! the CPU image and the legend together.

use engine::contract::legend::{outcome_entry, Palette, Swatch, SwatchStyle, K, SWATCHES};
use engine::field::{Field, Outcome};
use render::assemble::Declaration;
use render::colour::outcome;
use render::pipeline_cache::{block_layout, encode};
use validation::negative_control;

/// A field holding every `state` code, 0 to 8, by every `detail`, 0 to 4: row `state`, column `detail`.
fn every_code() -> Field {
    let outcomes = (0..9u32)
        .flat_map(|state| (0..5u32).map(move |detail| Outcome { state, detail }))
        .collect();
    Field {
        size: [5, 9],
        outcomes,
    }
}

const RECORDED_IMAGE: [u8; 180] = [
    20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20,
    24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255,
    20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24,
    255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20,
    20, 24, 255, 52, 98, 224, 255, 46, 188, 78, 255, 222, 45, 45, 255, 242, 150, 32, 255, 242, 150,
    32, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255, 20, 20, 24, 255,
    240, 222, 50, 255, 224, 52, 198, 255, 48, 200, 220, 255, 97, 21, 151, 255, 97, 21, 151, 255,
];

/// One swatch as recorded: `(param, label, state, detail, srgb8)`.
type Row = (&'static str, &'static str, u32, u32, [u8; 3]);

/// Each set's rows, in order, as recorded.
#[rustfmt::skip]
const RECORDED_SETS: [[Row; K]; 2] = [
    [
        ("bounded", "bounded / nothing yet", 1, 0, [0x14, 0x14, 0x18]),
        ("collision_pair_2", "collision 1–2", 2, 2, [0xDE, 0x2D, 0x2D]),
        ("collision_pair_1", "collision 1–3", 2, 1, [0x2E, 0xBC, 0x4E]),
        ("collision_pair_0", "collision 2–3", 2, 0, [0x34, 0x62, 0xE0]),
        ("triple_collision", "triple collision", 2, 3, [0xF2, 0x96, 0x20]),
        ("escape_body_0", "escape body 1", 0, 0, [0xF0, 0xDE, 0x32]),
        ("escape_body_1", "escape body 2", 0, 1, [0xE0, 0x34, 0xC6]),
        ("escape_body_2", "escape body 3", 0, 2, [0x30, 0xC8, 0xDC]),
        ("triple_ejection", "triple ejection", 0, 3, [0x61, 0x15, 0x97]),
    ],
    [
        ("oklab_bounded", "bounded / nothing yet", 1, 0, [0x14, 0x14, 0x18]),
        ("oklab_collision_pair_2", "collision 1–2", 2, 2, [0xE7, 0x85, 0x7A]),
        ("oklab_collision_pair_1", "collision 1–3", 2, 1, [0x6E, 0xB9, 0x76]),
        ("oklab_collision_pair_0", "collision 2–3", 2, 0, [0x7F, 0xA2, 0xF2]),
        ("oklab_triple_collision", "triple collision", 2, 3, [0xD9, 0x93, 0x49]),
        ("oklab_escape_body_0", "escape body 1", 0, 0, [0xB3, 0xA8, 0x41]),
        ("oklab_escape_body_1", "escape body 2", 0, 1, [0xD4, 0x86, 0xC3]),
        ("oklab_escape_body_2", "escape body 3", 0, 2, [0x09, 0xB9, 0xCD]),
        ("oklab_triple_ejection", "triple ejection", 0, 3, [0xB7, 0x90, 0xE2]),
    ],
];

/// The recorded default set, written by hand as a swatch table.
#[cfg(feature = "controls")]
fn hand_written() -> [Swatch; K] {
    RECORDED_SETS[0].map(|(param, label, state, detail, srgb8)| Swatch {
        param,
        label,
        state,
        detail,
        srgb8,
        proposed: false,
    })
}

/// The swatch the fixture edits, its class index, its recorded default as the source writes it, and the new default:
/// `#7FA2F2` decoded, as the source writes the OKLab-uniform collision 2–3.
const EDITED: &str = "escape_body_1";
const EDITED_CLASS: usize = 6;
const DECLARED: &str = "@uniform escape_body_1: vec3<f32> = (0.7454042, 0.034339808, 0.5647115)";
const FIXTURE_DECLARED: &str =
    "@uniform escape_body_1: vec3<f32> = (0.21223076, 0.3613068, 0.8879231)";
const FIXTURE_SRGB8: [u8; 3] = [0x7F, 0xA2, 0xF2];

/// The shipped source with [`EDITED`]'s default replaced.
fn fixture() -> String {
    let shipped = outcome::source();
    assert_eq!(
        shipped.matches(DECLARED).count(),
        1,
        "the shipped source declares `{EDITED}` once"
    );
    shipped.replace(DECLARED, FIXTURE_DECLARED)
}

/// Checks `palette`'s sets, the CPU image `swatches` draws and the legend's entries against the recorded values.
fn check_recorded(palette: &Palette, swatches: &[Swatch; K]) {
    let sets: Vec<Vec<Row>> = palette
        .sets
        .iter()
        .map(|(_, set)| {
            set.iter()
                .map(|s| (s.param, s.label, s.state, s.detail, s.srgb8))
                .collect()
        })
        .collect();
    let recorded: Vec<Vec<_>> = RECORDED_SETS.iter().map(|s| s.to_vec()).collect();
    assert_eq!(sets, recorded, "the sets are not the recorded ones");
    assert_eq!(
        every_code().rgba8_with(swatches),
        RECORDED_IMAGE,
        "the CPU image is not the recorded one"
    );
    for &(param, label, _, _, srgb8) in &RECORDED_SETS[0] {
        assert_eq!(
            outcome_entry(swatches, param),
            Some((label.to_owned(), SwatchStyle::Flat(srgb8))),
            "the legend's `{param}` is not the recorded one"
        );
    }
}

#[test]
fn cpu_image_and_legend_are_as_recorded() {
    let shipped = outcome::shipped();
    assert_eq!(
        *SWATCHES,
        *shipped.swatches(),
        "SWATCHES are the shipped source's"
    );
    assert_eq!(
        every_code().rgba8(),
        every_code().rgba8_with(&SWATCHES),
        "the field's own image draws SWATCHES"
    );
    check_recorded(shipped, &SWATCHES);
}

negative_control!(
    cpu_image_and_legend_are_as_recorded,
    "a source one swatch off the recorded palette must fail",
    expected = "not the recorded",
    {
        let edited = Palette::read(outcome::name(), &fixture()).expect("the fixture is a palette");
        check_recorded(&edited, edited.swatches());
    }
);

/// The colour the shader's uniform block holds for [`EDITED`] in `source`, each `f32` encoded to 8 bits, and whether
/// every other byte of the block is the shipped source's.
fn shader_colour(source: &str) -> ([u8; 3], bool) {
    let block = |source: &str| {
        let d = Declaration::parse(source).unwrap_or_else(|e| panic!("{e}"));
        let (offsets, size) = block_layout(&d.uniforms);
        let mut bytes = vec![0u8; size as usize];
        let mut at = 0;
        for (u, &o) in d.uniforms.iter().zip(&offsets) {
            let b = encode(u.ty, &u.default);
            bytes[o as usize..o as usize + b.len()].copy_from_slice(&b);
            if u.name == EDITED {
                at = o as usize;
            }
        }
        (bytes, at)
    };
    let (edited, at) = block(source);
    let (shipped, _) = block(outcome::source());
    let colour = [0, 1, 2].map(|i| {
        let c = f32::from_le_bytes(edited[at + 4 * i..at + 4 * i + 4].try_into().unwrap());
        outcome::encode8(f64::from(c))
    });
    let rest = |b: &[u8]| [&b[..at], &b[at + 12..]].concat();
    (colour, rest(&edited) == rest(&shipped))
}

/// Checks that the fixture's edited default reaches the shader's uniform block, the CPU image `swatches` draws and
/// the legend alike, and moves nothing else.
fn check_together(fixture: &str, swatches: &[Swatch; K]) {
    let (shader, rest) = shader_colour(fixture);
    assert_eq!(
        shader, FIXTURE_SRGB8,
        "the shader's uniform holds the fixture's default"
    );
    assert!(
        rest,
        "the fixture moves only `{EDITED}`'s bytes in the shader's block"
    );
    let image = every_code().rgba8_with(swatches);
    let field = every_code();
    for (i, (got, was)) in image.chunks(4).zip(RECORDED_IMAGE.chunks(4)).enumerate() {
        let (row, x) = (i / 5, i % 5);
        let o = field.at(x, 8 - row);
        let class = matches!(outcome::shown(o.state, o.detail, false), outcome::Shown::Swatch(k) if k == EDITED_CLASS);
        let want: Vec<u8> = if class {
            [shader[0], shader[1], shader[2], 255].to_vec()
        } else {
            was.to_vec()
        };
        assert_eq!(
            got, want,
            "the CPU image did not follow the shader at pixel {i}"
        );
    }
    for &(param, label, _, _, srgb8) in &RECORDED_SETS[0] {
        let want = if param == EDITED { shader } else { srgb8 };
        assert_eq!(
            outcome_entry(swatches, param),
            Some((label.to_owned(), SwatchStyle::Flat(want))),
            "the legend did not follow the shader at `{param}`"
        );
    }
}

#[test]
fn a_declared_default_moves_shader_image_and_legend_together() {
    let source = fixture();
    let read = Palette::read(outcome::name(), &source).expect("the fixture is a palette");
    check_together(&source, read.swatches());
}

negative_control!(
    a_declared_default_moves_shader_image_and_legend_together,
    "a swatch table written by hand does not follow the source and must fail",
    expected = "the CPU image did not follow the shader",
    check_together(&fixture(), &hand_written())
);

/// The largest distance, over every declared colour default of `palette`'s source, of `255 · encode(c)` from the
/// nearest integer: the margin by which each 8-bit code is clear of a rounding tie.
fn tie_distance(source: &str) -> f64 {
    let d = Declaration::parse(source).unwrap_or_else(|e| panic!("{e}"));
    d.uniforms
        .iter()
        .filter(|u| u.ty == render::assemble::UniformType::Vec3)
        .flat_map(|u| u.default.clone())
        .map(|c| {
            let v = 255.0 * render::present::linear_to_srgb(f64::from(c as f32));
            (v - v.round()).abs()
        })
        .fold(0.0, f64::max)
}

/// The encode's rounding is decided far from a tie: every declared default within 0.01 of an 8-bit code.
const TIE_MARGIN: f64 = 0.01;

fn check_clear_of_ties(source: &str) {
    let worst = tie_distance(source);
    println!("worst distance of a declared default's 8-bit encode from an integer: {worst:e}");
    assert!(
        worst < TIE_MARGIN,
        "a declared default encodes {worst} from an 8-bit code, within a tie's reach"
    );
}

#[test]
fn declared_defaults_encode_clear_of_ties() {
    check_clear_of_ties(outcome::source());
}

negative_control!(
    declared_defaults_encode_clear_of_ties,
    "a default at an 8-bit rounding tie must fail",
    expected = "within a tie's reach",
    {
        // 127.5 / 255 decoded: a tie between codes 127 and 128.
        let tie = render::present::srgb_to_linear(127.5 / 255.0);
        check_clear_of_ties(&outcome::source().replace(
            DECLARED,
            &format!("@uniform escape_body_1: vec3<f32> = ({tie}, {tie}, {tie})"),
        ));
    }
);
