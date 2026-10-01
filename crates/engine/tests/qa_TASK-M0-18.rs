//! QA tests for TASK-M0-18's canonical serialisation of `SimConfig` and `RenderState` (REQ-TOOL-145, R-309), written
//! from its definition in `principia_gui_state_contract.md` §2, "One canonical serialisation": compact JSON, every
//! object's keys in ascending byte order, integers in decimal, each float from its shortest round-trip digits in one
//! layout, strings with exactly the listed escapes; equal state is equal text and every value reads back exactly.
//! Each test has its negative control (R-176).
#![allow(non_snake_case)]

use std::collections::{BTreeMap, HashMap};

use engine::contract::canonical;
use engine::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, Links, Lock, Plane, Quality, SimConfig, Slice,
};
use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

fn sim() -> SimConfig {
    SimConfig {
        chart: Chart {},
        plane: Plane {},
        slice: Slice {},
        lock: Lock {},
        links: Links {},
        integrator: Integrator {},
        horizon: Horizon {},
        collision: Collision {},
        quality: Quality {},
    }
}

fn render() -> RenderState {
    RenderState {
        stain_graph: StainGraph {},
        overlays: Overlays {},
        palette: Palette {},
        playhead: Playhead {},
    }
}

/// The M0 skeleton's canonical text, by §2's rules: its groups' names in byte order, each group `{}`.
const SIM_TEXT: &str =
    "{\"chart\":{},\"collision\":{},\"horizon\":{},\"integrator\":{},\"links\":{},\"lock\":{},\
\"plane\":{},\"quality\":{},\"slice\":{}}";
const RENDER_TEXT: &str = "{\"overlays\":{},\"palette\":{},\"playhead\":{},\"stain_graph\":{}}";

fn check_skeleton(sim_text: &str, render_text: &str) {
    assert_eq!(
        sim_text, SIM_TEXT,
        "SimConfig's canonical text is not its groups in byte order"
    );
    assert_eq!(
        render_text, RENDER_TEXT,
        "RenderState's canonical text is not its groups in byte order"
    );
}

#[test]
fn qa_canonical_skeleton_text_same_bytes_twice_and_reads_back() {
    let (s1, s2) = (
        canonical::to_string(&sim()).unwrap(),
        canonical::to_string(&sim()).unwrap(),
    );
    let (r1, r2) = (
        canonical::to_string(&render()).unwrap(),
        canonical::to_string(&render()).unwrap(),
    );
    assert_eq!(
        s1.as_bytes(),
        s2.as_bytes(),
        "SimConfig twice: different bytes"
    );
    assert_eq!(
        r1.as_bytes(),
        r2.as_bytes(),
        "RenderState twice: different bytes"
    );
    check_skeleton(&s1, &r1);
    let sim_back: SimConfig = serde_json::from_str(&s1).expect("SimConfig does not read back");
    let render_back: RenderState =
        serde_json::from_str(&r1).expect("RenderState does not read back");
    assert_eq!(sim_back, sim());
    assert_eq!(render_back, render());
}

validation::negative_control!(
    qa_canonical_skeleton_text_same_bytes_twice_and_reads_back,
    "the declaration-order text must fail the canonical-text check",
    expected = "is not its groups in byte order",
    check_skeleton(
        "{\"chart\":{},\"plane\":{},\"slice\":{},\"lock\":{},\"links\":{},\"integrator\":{},\"horizon\":{},\
\"collision\":{},\"quality\":{}}",
        RENDER_TEXT
    )
);

/// A struct declaring its fields out of byte order, with keys that test the order's edges: a prefix before a longer
/// key, upper case (0x41–0x5A) before `_` (0x5F) before lower case, and a multi-byte UTF-8 key after every ASCII one.
#[derive(Serialize)]
#[allow(non_snake_case)]
struct Shuffled {
    zeta: u8,
    ab: u8,
    a: u8,
    #[serde(rename = "é")]
    e_acute: u8,
    B: u8,
    _c: u8,
    a_b: u8,
    nested: Nested,
}

#[derive(Serialize)]
struct Nested {
    y: Vec<Inner>,
    x: Option<u8>,
}

#[derive(Serialize)]
struct Inner {
    q: bool,
    p: (),
}

fn shuffled() -> Shuffled {
    Shuffled {
        zeta: 1,
        ab: 2,
        a: 3,
        e_acute: 4,
        B: 5,
        _c: 6,
        a_b: 7,
        nested: Nested {
            y: vec![Inner { q: true, p: () }],
            x: None,
        },
    }
}

const SHUFFLED_TEXT: &str = "{\"B\":5,\"_c\":6,\"a\":3,\"a_b\":7,\"ab\":2,\"nested\":{\"x\":null,\
\"y\":[{\"p\":null,\"q\":true}]},\"zeta\":1,\"é\":4}";

fn check_key_order(text: &str) {
    assert_eq!(
        text, SHUFFLED_TEXT,
        "the keys are not in ascending byte order at every depth"
    );
}

#[test]
fn qa_canonical_key_order_is_bytewise_at_every_depth() {
    check_key_order(&canonical::to_string(&shuffled()).unwrap());
    // A map's insertion order does not show: three orders of the same entries give one text.
    let entries = [
        ("delta", 4),
        ("alpha", 1),
        ("Charlie", 3),
        ("bravo", 2),
        ("alph", 0),
    ];
    let mut texts = Vec::new();
    for rotate in 0..entries.len() {
        let mut e = entries.to_vec();
        e.rotate_left(rotate);
        let map: HashMap<&str, i32> = e.iter().copied().collect();
        texts.push(canonical::to_string(&map).unwrap());
        let tree: BTreeMap<&str, i32> = e.iter().copied().collect();
        texts.push(canonical::to_string(&tree).unwrap());
    }
    for t in &texts {
        assert_eq!(
            t, "{\"Charlie\":3,\"alph\":0,\"alpha\":1,\"bravo\":2,\"delta\":4}",
            "a map's text depends on its insertion order"
        );
    }
}

validation::negative_control!(
    qa_canonical_key_order_is_bytewise_at_every_depth,
    "serde_json's declaration-order text must fail the key-order check",
    expected = "not in ascending byte order",
    check_key_order(&serde_json::to_string(&shuffled()).unwrap())
);

/// A map that writes the same key twice: no canonical form.
struct TwoEqualKeys;

impl Serialize for TwoEqualKeys {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        m.serialize_entry("k", &1)?;
        m.serialize_entry("k", &2)?;
        m.end()
    }
}

fn check_fails<T: Serialize>(value: &T, what: &str) {
    let got = canonical::to_string(value);
    assert!(got.is_err(), "{what} serialised canonically as {got:?}");
}

#[test]
fn qa_canonical_refuses_what_has_no_form() {
    check_fails(&TwoEqualKeys, "an object with two equal keys");
    let int_keys: BTreeMap<i32, i32> = [(1, 1)].into_iter().collect();
    check_fails(&int_keys, "a map with integer keys");
    for (v, what) in [
        (f64::NAN, "NaN"),
        (f64::INFINITY, "+inf"),
        (f64::NEG_INFINITY, "-inf"),
    ] {
        check_fails(&v, what);
        check_fails(&vec![1.0, v], &format!("{what} in an array"));
        let mut m = BTreeMap::new();
        m.insert("deep", BTreeMap::from([("x", v)]));
        check_fails(&m, &format!("{what} in a nested object"));
        check_fails(&(v as f32), &format!("{what} as f32"));
    }
}

validation::negative_control!(
    qa_canonical_refuses_what_has_no_form,
    "a finite float must not be refused",
    expected = "serialised canonically",
    check_fails(&1.5f64, "1.5")
);

/// §2's float layout, by the definition's own examples and its edges: zero and negative zero, the positional range
/// −5 ≤ e ≤ 15 at both ends, scientific outside it, the extremes of f64, and an f32 widened exactly.
fn float_cases() -> Vec<(f64, &'static str)> {
    vec![
        (0.0, "0.0"),
        (-0.0, "-0.0"),
        (1234.0, "1234.0"),
        (12.34, "12.34"),
        (0.001234, "0.001234"),
        (0.00001, "0.00001"),
        (1e15, "1000000000000000.0"),
        (1e16, "1e16"),
        (1.5e-7, "1.5e-7"),
        (5e-324, "5e-324"),
        (0.000001, "1e-6"),
        (0.0000123, "0.0000123"),
        (9999999999999998.0, "9999999999999998.0"),
        (1.2345e16, "1.2345e16"),
        (-2.5, "-2.5"),
        (-1.5e-7, "-1.5e-7"),
        (0.1, "0.1"),
        (1.0, "1.0"),
        (100.0, "100.0"),
        (f64::MAX, "1.7976931348623157e308"),
        (f64::MIN_POSITIVE, "2.2250738585072014e-308"),
        (f64::from(0.1f32), "0.10000000149011612"),
        (f64::from(16777217.0f32), "16777216.0"),
    ]
}

fn check_floats(format: impl Fn(f64) -> String) {
    for (v, want) in float_cases() {
        let got = format(v);
        assert_eq!(got, want, "{v:e} is written {got:?}, not {want:?}");
    }
}

#[test]
fn qa_canonical_float_layout() {
    // Through a struct field, as the state's floats are written.
    #[derive(Serialize)]
    struct F {
        v: f64,
    }
    check_floats(|v| {
        let t = canonical::to_string(&F { v }).unwrap();
        t.strip_prefix("{\"v\":")
            .and_then(|r| r.strip_suffix('}'))
            .unwrap()
            .to_owned()
    });
    // An f32 field is widened to f64 exactly, so 0.1f32 is the f64 nearest it, not 0.1.
    assert_eq!(
        canonical::to_string(&0.1f32).unwrap(),
        "0.10000000149011612"
    );
    assert_eq!(canonical::to_string(&-0.0f32).unwrap(), "-0.0");
}

validation::negative_control!(
    qa_canonical_float_layout,
    "Rust's own float Display must fail the layout check",
    expected = "is written",
    check_floats(|v| format!("{v}"))
);

/// Integers in decimal: no leading zero, `-` for a negative, no `+`, fraction or exponent.
fn check_integers(text: &str) {
    assert_eq!(
        text, "[0,-1,7,-9223372036854775808,18446744073709551615,255,-128,4294967295]",
        "the integers are not in plain decimal"
    );
}

#[test]
fn qa_canonical_integers() {
    #[derive(Serialize)]
    struct I(i32, i32, u8, i64, u64, u8, i8, u32);
    check_integers(
        &canonical::to_string(&I(0, -1, 7, i64::MIN, u64::MAX, 255, -128, u32::MAX)).unwrap(),
    );
}

validation::negative_control!(
    qa_canonical_integers,
    "integers written as floats must fail the integer check",
    expected = "not in plain decimal",
    check_integers(
        &canonical::to_string(&[
            0.0,
            -1.0,
            7.0,
            -9223372036854775808.0,
            1.8446744073709552e19
        ])
        .unwrap()
    )
);

/// The digits of a canonical float, and its decimal exponent e (the value is d₁.d₂…d_k × 10^e), read from the text.
fn digits_and_exponent(text: &str) -> (String, i32) {
    let body = text.strip_prefix('-').unwrap_or(text);
    if let Some((mantissa, exp)) = body.split_once('e') {
        assert!(
            !exp.starts_with('+')
                && !exp.starts_with("-0")
                && !(exp.starts_with('0') && exp.len() > 1),
            "{text}: the exponent has a + or a leading zero"
        );
        let e: i32 = exp.parse().unwrap();
        let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
        if let Some((int, frac)) = mantissa.split_once('.') {
            assert_eq!(int.len(), 1, "{text}: not one digit before the point");
            assert!(
                !frac.is_empty() && !frac.ends_with('0'),
                "{text}: a bare or zero-ended fraction"
            );
        }
        assert!(
            !digits.starts_with('0'),
            "{text}: a leading zero in the mantissa"
        );
        (digits, e)
    } else {
        let (int, frac) = body
            .split_once('.')
            .unwrap_or_else(|| panic!("{text}: no '.' and no 'e', so it reads as an integer"));
        assert!(
            !int.is_empty() && !frac.is_empty(),
            "{text}: no digit on a side of the point"
        );
        assert!(
            int == "0" || !int.starts_with('0'),
            "{text}: a leading zero"
        );
        let all = format!("{int}{frac}");
        let lead = all.find(|c| c != '0').expect("zero is not in this check");
        let e = int.len() as i32 - 1 - lead as i32;
        let digits = all.trim_start_matches('0').trim_end_matches('0').to_owned();
        (digits, e)
    }
}

/// Deterministic f64s spread over every exponent: random bit patterns, finite and non-zero.
fn random_floats(n: usize) -> Vec<f64> {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let v = f64::from_bits(x);
        if v.is_finite() && v != 0.0 {
            out.push(v);
        }
        // And short decimals, which are common in state: k / 10^j.
        let short = (x % 100_000) as f64 / 10f64.powi((x >> 40) as i32 % 12);
        if short != 0.0 {
            out.push(short);
        }
    }
    out
}

/// Every value reads back to the same bits, through a text in §2's layout: positional for −5 ≤ e ≤ 15, scientific
/// otherwise; and its digits are the shortest that do (one digit fewer, rounded either way, reads back differently).
fn check_reads_back_shortest(format: impl Fn(f64) -> String, values: &[f64]) {
    for &v in values {
        let text = format(v);
        let back: f64 = text
            .parse()
            .unwrap_or_else(|e| panic!("{text} does not parse: {e}"));
        assert_eq!(
            back.to_bits(),
            v.to_bits(),
            "{v:e} is written {text}, which reads back as {back:e}"
        );
        let (digits, e) = digits_and_exponent(&text);
        let scientific = text.contains('e');
        assert_eq!(
            scientific,
            !(-5..=15).contains(&e),
            "{text}: exponent {e} in the wrong layout"
        );
        let k = digits.len();
        if k > 1 {
            let shorter: u64 = digits[..k - 1].parse().unwrap();
            let sign = if v < 0.0 { "-" } else { "" };
            for cand in [shorter, shorter + 1] {
                let c = format!("{sign}{cand}e{}", e - (k as i32 - 2));
                let cv: f64 = c.parse().unwrap();
                assert_ne!(
                    cv.to_bits(),
                    v.to_bits(),
                    "{text} is not the shortest: {c} reads back to the same value"
                );
            }
        }
        assert!(k <= 17, "{text}: {k} significant digits");
    }
}

fn canonical_f64(v: f64) -> String {
    canonical::to_string(&v).unwrap()
}

#[test]
fn qa_canonical_numbers_read_back_exactly() {
    check_reads_back_shortest(canonical_f64, &random_floats(20_000));
    let edges = [
        f64::MAX,
        -f64::MAX,
        f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        f64::EPSILON,
        1e-5,
        9.99999e-6,
        1e15,
        9.999999999999998e15,
        1e16,
        0.1 + 0.2,
        2f64.powi(53) + 2.0,
    ];
    check_reads_back_shortest(canonical_f64, &edges);
    // An f32 reads back as the same f32.
    for v in [0.1f32, f32::MAX, f32::MIN_POSITIVE, 1e-45, -3.3333333] {
        let text = canonical::to_string(&v).unwrap();
        let back: f64 = text.parse().unwrap();
        assert_eq!(
            (back as f32).to_bits(),
            v.to_bits(),
            "{v:e} as f32 reads back as {back:e}"
        );
        assert_eq!(back, f64::from(v), "{v:e} is not widened exactly: {text}");
    }
}

validation::negative_control!(
    qa_canonical_numbers_read_back_exactly,
    "nine significant digits must fail the read-back check",
    expected = "which reads back as",
    check_reads_back_shortest(|v| format!("{v:.8e}"), &[0.1 + 0.2])
);

validation::negative_control!(
    qa_canonical_numbers_read_back_exactly_shortest,
    "digits past the shortest, reading back to the same value, must fail the shortest check",
    expected = "is not the shortest",
    check_reads_back_shortest(|_| "1.5000000000000000001e-7".to_owned(), &[1.5e-7])
);

#[test]
fn qa_canonical_numbers_read_back_exactly_shortest() {
    check_reads_back_shortest(canonical_f64, &[1.0e20 + 1.0e5 * 3.0, 0.3, 123.456]);
}

/// §2's string escapes: `"`, `\`, and U+0000–U+001F (the five short forms, else `\u00xx` in lower-case hex);
/// everything else as itself, `/`, DEL and U+2028 included.
fn check_strings(quote: impl Fn(&str) -> String) {
    let cases = [
        ("plain", "\"plain\""),
        ("q\"b\\s", "\"q\\\"b\\\\s\""),
        ("\u{8}\t\n\u{c}\r", "\"\\b\\t\\n\\f\\r\""),
        ("\u{0}\u{1}\u{1b}\u{1f}", "\"\\u0000\\u0001\\u001b\\u001f\""),
        ("/ \u{7f} \u{2028} é 😀", "\"/ \u{7f} \u{2028} é 😀\""),
    ];
    for (s, want) in cases {
        let got = quote(s);
        assert_eq!(got, want, "{s:?} is written {got}, not {want}");
    }
}

#[test]
fn qa_canonical_string_escapes() {
    check_strings(|s| canonical::to_string(s).unwrap());
    // And as a key.
    let m: BTreeMap<&str, u8> = [("a\"\u{1}", 1)].into_iter().collect();
    assert_eq!(canonical::to_string(&m).unwrap(), "{\"a\\\"\\u0001\":1}");
}

validation::negative_control!(
    qa_canonical_string_escapes,
    "upper-case hex escapes must fail the string check",
    expected = "is written",
    check_strings(|s| canonical::to_string(s)
        .unwrap()
        .replace("\\u001b", "\\u001B"))
);

fn check_equal_text(a: &str, b: &str) {
    assert_eq!(
        a.as_bytes(),
        b.as_bytes(),
        "equal state gives different text"
    );
}

#[derive(Serialize)]
struct State {
    z0: [f64; 2],
    dt: f64,
    name: String,
    flags: BTreeMap<String, bool>,
}

fn state(dt: f64) -> State {
    State {
        z0: [0.25, -1.0 / 3.0],
        dt,
        name: "slice".to_owned(),
        flags: [("lock".to_owned(), true)].into_iter().collect(),
    }
}

/// Equal state is equal text: two values built apart, with floats reached by different arithmetic, give the same
/// bytes; a value one ulp away gives different bytes.
#[test]
fn qa_canonical_equal_state_equal_text() {
    let a = canonical::to_string(&state(0.1 * 3.0)).unwrap();
    let b = canonical::to_string(&state(0.30000000000000004)).unwrap();
    check_equal_text(&a, &b);
    let c = canonical::to_string(&state(f64::from_bits((0.1f64 * 3.0).to_bits() + 1))).unwrap();
    assert_ne!(a, c, "states one ulp apart give the same text");
    assert!(
        !a.contains(' '),
        "the text has whitespace between tokens: {a}"
    );
}

validation::negative_control!(
    qa_canonical_equal_state_equal_text,
    "the same state through serde_json's pretty writer must fail the equal-text check",
    expected = "equal state gives different text",
    check_equal_text(
        &canonical::to_string(&state(0.3)).unwrap(),
        &serde_json::to_string_pretty(&state(0.3)).unwrap()
    )
);
