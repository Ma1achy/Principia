//! The ROUNDTRIP preset's CPU side (`render::presets::roundtrip`; TASK-M2-26; REQ-RENDER-025, REQ-ENC-002; R-75,
//! R-116, R-297): its class, brightness, colour and residual twins, and the preset's text: locked, in the library, a
//! fragment view over the generated decode and encode, never a hand-written copy, its tolerance the register's
//! `ε_phys`.
//!
//! - `roundtrip_preset_class_order`: `Class::of` takes the first that holds of invalid, either decode `DEGENERATE`,
//!   `lookup_clamped` (notices bits 0 and 8 to 15), `lookup_mirrored` (bit 1), at each bit and its neighbours; and each
//!   class's code round-trips. Control: the clamp and mirror tests swapped.
//! - `roundtrip_preset_brightness_and_colour`: the brightness is 0 at or below `ε_phys`, `log₂ρ/24` above it, full
//!   from `2²⁴`; an untagged pixel is `(b, 0, 0)`, a tagged one its hue with no red at `¼ + ¾b`, an invalid one the
//!   hatch; `ε_phys` is the register's `k` times f32's epsilon. Control: a natural-log brightness.
//!
//! The residual twin is held to `validation::roundtrip::phys` in `validation`'s `roundtrip_preset_golden.rs`.
//! - `roundtrip_preset_text`: the preset is locked, named, in the library; its views place the generated free decode
//!   and encode verbatim (R-116), state the register's `ε_phys`, the span and floor, and compile with naga over every
//!   chart's decode. Control: a view without the generated encode.

use render::presets::roundtrip::{
    self, brightness, colour, eps_phys, ratio, Class, CLAMPED, FLOOR, MIRRORED, SPAN,
};
use render::presets::{self, section, DecodeChart, DECODE_WGSL, ENCODE_WGSL};
use validation::negative_control;

// ── The class ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// `(finite, tag, tag_free, notices)` and the class each must be.
const CASES: [(bool, u32, u32, u32, Class); 16] = [
    (false, 4, 4, 0xffff, Class::Invalid),
    (false, 0, 0, 0, Class::Invalid),
    (true, 4, 0, 0xff03, Class::Feasibility),
    (true, 0, 1, 0xff03, Class::Feasibility),
    (true, 1, 0, 0, Class::Feasibility),
    (true, 0, 0, 0x0001 | 0x0002, Class::Clamp),
    (true, 0, 0, 0x0100 | 0x0002, Class::Clamp),
    (true, 0, 0, 0x8000, Class::Clamp),
    (true, 0, 0, 0x0001, Class::Clamp),
    (true, 0, 0, 0x0002, Class::MirrorTie),
    (true, 0, 0, 0x0002 | 0x0004, Class::MirrorTie),
    (true, 0, 0, 0x0004, Class::Untagged),
    (true, 0, 0, 0x00fc, Class::Untagged),
    (true, 0, 0, 0x0001_0000, Class::Untagged),
    (true, 0, 0, 0x0080, Class::Untagged),
    (true, 0, 0, 0, Class::Untagged),
];

fn check_class(of: fn(bool, u32, u32, u32) -> Class) {
    for (finite, tag, tag_free, notices, want) in CASES {
        assert_eq!(
            of(finite, tag, tag_free, notices),
            want,
            "the class of (finite {finite}, tags {tag}, {tag_free}, notices {notices:#x})"
        );
    }
    assert_eq!((CLAMPED, MIRRORED), (0xff01, 2), "the notices' tag bits");
    for c in [
        Class::Untagged,
        Class::Feasibility,
        Class::Clamp,
        Class::MirrorTie,
        Class::Invalid,
    ] {
        assert_eq!(Class::from_code(c as u32), Some(c), "{c:?}'s code");
    }
    assert_eq!(Class::from_code(5), None, "code 5 is no class");
}

#[test]
fn roundtrip_preset_class_order() {
    check_class(Class::of);
}

negative_control!(
    roundtrip_preset_class_order,
    "the clamp and mirror tests swapped must be found",
    expected = "the class of",
    check_class(|finite, tag, tag_free, notices| {
        match Class::of(finite, tag, tag_free, notices & !(CLAMPED | MIRRORED)) {
            Class::Untagged if notices & MIRRORED != 0 => Class::Clamp,
            Class::Untagged if notices & CLAMPED != 0 => Class::MirrorTie,
            c => c,
        }
    })
);

// ── The brightness and colour ───────────────────────────────────────────────────────────────────────────────────────

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= 1e-12)
}

fn check_colour(bright: fn(f64) -> f64) {
    let k = ledger::constants::EPS_PHYS.number();
    assert_eq!(
        eps_phys(),
        k * f64::from(f32::EPSILON),
        "ε_phys is not k·2⁻²³"
    );
    assert_eq!(ratio(eps_phys()), 1.0, "ε_phys's ratio");
    assert_eq!(ratio(0.0), 0.0);
    assert_eq!((SPAN, FLOOR), (24.0, 0.25), "the span and floor");
    let at = [
        (0.0, 0.0),
        (0.5, 0.0),
        (1.0, 0.0),
        (2.0, 1.0 / 24.0),
        (8.0, 3.0 / 24.0),
        (2f64.powi(12), 0.5),
        (2f64.powi(24), 1.0),
        (2f64.powi(30), 1.0),
    ];
    for (r, want) in at {
        let got = bright(r);
        assert!(
            (got - want).abs() <= 1e-15,
            "brightness({r}) = {got}, not {want}"
        );
        assert_eq!(got, brightness(r), "brightness({r})");
    }
    let above = bright(1.0 + 1e-9);
    assert!(
        above > 0.0 && above < 1e-9,
        "brightness just past ε_phys: {above}"
    );
    let xy = [3.5, 7.5];
    for r in [0.5, 2.0, 2f64.powi(12), 2f64.powi(24)] {
        let b = bright(r);
        let t = FLOOR + (1.0 - FLOOR) * b;
        for (class, want) in [
            (Class::Untagged, [b, 0.0, 0.0]),
            (Class::Feasibility, [0.0, t, t]),
            (Class::Clamp, [0.0, 0.0, t]),
            (Class::MirrorTie, [0.0, t, 0.0]),
            (Class::Invalid, render::present::debug_invalid(xy)),
        ] {
            let got = colour(class, r, xy);
            assert!(
                close(got, want),
                "{class:?} at ratio {r}: {got:?}, not {want:?}"
            );
        }
    }
    assert_eq!(
        colour(Class::Untagged, 1.0, xy),
        [0.0; 3],
        "within ε_phys is black"
    );
    assert_eq!(
        colour(Class::Clamp, 1.0, xy),
        [0.0, 0.0, FLOOR],
        "a tag shows at its floor"
    );
}

#[test]
fn roundtrip_preset_brightness_and_colour() {
    check_colour(brightness);
}

negative_control!(
    roundtrip_preset_brightness_and_colour,
    "a natural-log brightness must be found",
    expected = "brightness(",
    check_colour(|r| (r.max(1.0).ln() / SPAN).min(1.0))
);

// ── The text ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// `name`'s value in the WGSL `const <name>: f32 = <value>;` of `text`.
fn wgsl_const(text: &str, name: &str) -> f32 {
    let open = format!("const {name}: f32 = ");
    let at = text
        .find(&open)
        .unwrap_or_else(|| panic!("no const {name}"))
        + open.len();
    let end = at + text[at..].find(';').unwrap();
    text[at..end]
        .parse()
        .unwrap_or_else(|e| panic!("const {name}: {e}"))
}

fn check_text(view: &str) {
    let free = section(DECODE_WGSL, "free").unwrap();
    assert!(
        view.contains(free),
        "the view does not place the generated free decode"
    );
    assert!(
        view.contains(ENCODE_WGSL),
        "the view does not place the generated encode"
    );
    assert!(
        !view.contains("fn frag_decode("),
        "the view defines a chart's decode of its own"
    );
    assert_eq!(wgsl_const(view, "ROUNDTRIP_EPS_PHYS"), eps_phys() as f32);
    assert_eq!(f64::from(wgsl_const(view, "ROUNDTRIP_SPAN")), SPAN);
    assert_eq!(f64::from(wgsl_const(view, "ROUNDTRIP_FLOOR")), FLOOR);
    assert!(view.contains(&format!("{CLAMPED:#x}u")) && view.contains(&format!("& {MIRRORED}u")));
}

#[test]
fn roundtrip_preset_text() {
    let p = roundtrip::preset();
    assert_eq!(p.name(), "ROUNDTRIP");
    assert!(p.locked());
    assert_eq!(p.output(), render::bind::ViewOutput::Colour);
    assert_eq!(roundtrip::words().output(), render::bind::ViewOutput::Words);
    assert_eq!(
        presets::library()[2],
        p,
        "ROUNDTRIP is not the library's third preset"
    );
    check_text(p.view());
    check_text(roundtrip::words().view());
    assert!(p.view().starts_with(&roundtrip::snippet()));
    for chart in DecodeChart::ALL {
        for preset in [roundtrip::preset(), roundtrip::words()] {
            let module = preset.module(chart).unwrap();
            let parsed = naga::front::wgsl::parse_str(&module)
                .unwrap_or_else(|e| panic!("{chart:?}: {}", e.emit_to_string(&module)));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&parsed)
            .unwrap_or_else(|e| panic!("{chart:?}: the ROUNDTRIP module does not validate: {e:?}"));
        }
    }
}

negative_control!(
    roundtrip_preset_text,
    "a view without the generated encode must be found",
    expected = "the generated encode",
    check_text(&roundtrip::preset().view().replace(ENCODE_WGSL, ""))
);
