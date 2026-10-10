//! QA tests for TASK-M0-18's canonical serialisation of `SimConfig` and `RenderState` (REQ-TOOL-145, R-309), written
//! from its definition in `principia_gui_state_contract.md` §2 as R-318 and R-322 rule it: JCS, RFC 8785 — members
//! sorted by their names' UTF-16 code units, no whitespace, its string escaping and its number format, checked on the
//! RFC's own published vectors; −0.0 written `0`; a u64 field always a JSON string of its decimal digits, every other
//! number a JSON number; equal state is equal text and every value reads back exactly. Each test has its negative
//! control (R-176).
// The file name `qa_TASK-M0-18` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::collections::{BTreeMap, HashMap};

use engine::contract::canonical;
use engine::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice,
};
use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

fn sim() -> SimConfig {
    SimConfig {
        chart: Chart {},
        plane: Plane {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
        slice: Slice {},
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links {},
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
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
        playhead: Playhead { t: 0.0 },
    }
}

/// The M0 skeleton's canonical text, by §2's rules: its groups' names in byte order, each group `{}` but for the
/// plane and the lock (R-390), whose members are in byte order too, all zero and unlocked.
const SIM_TEXT: &str =
    "{\"chart\":{},\"collision\":{},\"horizon\":{},\"integrator\":{},\"kernel_variant\":\"physics\",\"links\":{},\
\"lock\":{\"locked\":false,\"z_locked\":[0,0,0,0,0,0,0,0]},\
\"plane\":{\"q1\":[0,0,0,0,0,0,0,0],\"q2\":[0,0,0,0,0,0,0,0],\"z0\":[0,0,0,0,0,0,0,0]},\"quality\":{},\"slice\":{}}";
const RENDER_TEXT: &str =
    "{\"overlays\":{},\"palette\":{},\"playhead\":{\"t\":0},\"stain_graph\":{}}";

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
        "{\"chart\":{},\"plane\":{\"z0\":[0,0,0,0,0,0,0,0],\"q1\":[0,0,0,0,0,0,0,0],\"q2\":[0,0,0,0,0,0,0,0]},\"slice\":{},\
\"lock\":{\"locked\":false,\"z_locked\":[0,0,0,0,0,0,0,0]},\"links\":{},\"integrator\":{},\
\"kernel_variant\":\"physics\",\"horizon\":{},\"collision\":{},\"quality\":{}}",
        RENDER_TEXT
    )
);

/// A struct declaring its fields out of byte order, with keys that test the order's edges: a prefix before a longer
/// key, upper case (0x41–0x5A) before `_` (0x5F) before lower case, and a multi-byte UTF-8 key after every ASCII one.
#[derive(Serialize)]
// The field `B` is upper case on purpose: it tests that upper case (0x42) sorts before `_` and lower case.
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

/// JCS's number format (RFC 8785 §3.2.2.3, ECMAScript's Number-to-String), by RFC 8785 Appendix B's published
/// table: each IEEE 754 bit pattern and the text JCS writes for it. Both zeros write `0` (R-318: −0.0 serialises as 0).
const RFC8785_NUMBERS: &[(u64, &str)] = &[
    (0x0000000000000000, "0"),
    (0x8000000000000000, "0"),
    (0x0000000000000001, "5e-324"),
    (0x8000000000000001, "-5e-324"),
    (0x7fefffffffffffff, "1.7976931348623157e+308"),
    (0xffefffffffffffff, "-1.7976931348623157e+308"),
    (0x4340000000000000, "9007199254740992"),
    (0xc340000000000000, "-9007199254740992"),
    (0x4430000000000000, "295147905179352830000"),
    (0x44b52d02c7e14af5, "9.999999999999997e+22"),
    (0x44b52d02c7e14af6, "1e+23"),
    (0x44b52d02c7e14af7, "1.0000000000000001e+23"),
    (0x444b1ae4d6e2ef4e, "999999999999999700000"),
    (0x444b1ae4d6e2ef4f, "999999999999999900000"),
    (0x444b1ae4d6e2ef50, "1e+21"),
    (0x3eb0c6f7a0b5ed8c, "9.999999999999997e-7"),
    (0x3eb0c6f7a0b5ed8d, "0.000001"),
    (0x41b3de4355555553, "333333333.3333332"),
    (0x41b3de4355555554, "333333333.33333325"),
    (0x41b3de4355555555, "333333333.3333333"),
    (0x41b3de4355555556, "333333333.3333334"),
    (0x41b3de4355555557, "333333333.33333343"),
    (0xbecbf647612f3696, "-0.0000033333333333333333"),
    (0x43143ff3c1cb0959, "1424953923781206.2"),
];

fn check_rfc8785_numbers(format: impl Fn(f64) -> String) {
    for &(bits, want) in RFC8785_NUMBERS {
        let v = f64::from_bits(bits);
        let got = format(v);
        assert_eq!(
            got, want,
            "0x{bits:016x} ({v:e}) is written {got:?}, not {want:?}"
        );
    }
}

/// An f64 struct field's canonical text.
fn field_text(v: f64) -> String {
    #[derive(Serialize)]
    struct F {
        v: f64,
    }
    let t = canonical::to_string(&F { v }).unwrap();
    t.strip_prefix("{\"v\":")
        .and_then(|r| r.strip_suffix('}'))
        .unwrap()
        .to_owned()
}

#[test]
fn qa_canonical_float_layout() {
    // RFC 8785 Appendix B, through a struct field as the state's floats are written, and as JSON data.
    check_rfc8785_numbers(field_text);
    check_rfc8785_numbers(|v| canonical::json_to_string(&serde_json::json!(v)).unwrap());
    // −0.0 writes 0, as f64 and as f32 (R-318).
    assert_eq!(canonical::to_string(&-0.0f64).unwrap(), "0");
    assert_eq!(canonical::to_string(&-0.0f32).unwrap(), "0");
    // An f32 is the f64 it widens to, exactly, written in JCS's format.
    assert_eq!(
        canonical::to_string(&0.1f32).unwrap(),
        "0.10000000149011612"
    );
    // The format's switch points, each side: 1e21 is the first exponent form, 1e-7 the first small one.
    for (v, want) in [
        (1e20, "100000000000000000000"),
        (1e21, "1e+21"),
        (1e-6, "0.000001"),
        (1e-7, "1e-7"),
        (1.5e-7, "1.5e-7"),
        (123.456, "123.456"),
        (1.0, "1"),
        (-2.5, "-2.5"),
        (0.1 + 0.2, "0.30000000000000004"),
    ] {
        assert_eq!(field_text(v), want, "{v:e}");
    }
}

validation::negative_control!(
    qa_canonical_float_layout,
    "Rust's own float Display must fail the RFC 8785 number check",
    expected = "is written",
    check_rfc8785_numbers(|v| format!("{v}"))
);

/// RFC 8785's own examples, as published: §3.2.2's input and its canonical output (literals, numbers, a string with
/// every kind of escape), and §3.2.3's sorting example, whose keys order differently by UTF-16 code units than by
/// UTF-8 bytes (U+1F600 before U+FB33). Each output is given as its UTF-8 bytes in hex.
const RFC8785_VECTORS: &[(&str, &str)] = &[
    (
        "{\n  \"numbers\": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001],\n  \
         \"string\": \"\\u20ac$\\u000F\\u000aA'\\u0042\\u0022\\u005c\\\\\\\"\\/\",\n  \
         \"literals\": [null, true, false]\n}",
        "7b226c69746572616c73223a5b6e756c6c2c747275652c66616c73655d2c226e756d62657273223a5b3333333333333333332e33\
         3333333333332c31652b33302c342e352c302e3030322c31652d32375d2c22737472696e67223a22e282ac245c75303030665c6e\
         4127425c225c5c5c5c5c222f227d",
    ),
    (
        "{\n  \"\\u20ac\": \"Euro Sign\",\n  \"\\r\": \"Carriage Return\",\n  \
         \"\\ufb33\": \"Hebrew Letter Dalet With Dagesh\",\n  \"1\": \"One\",\n  \
         \"\\ud83d\\ude00\": \"Emoji: Grinning Face\",\n  \"\\u0080\": \"Control\",\n  \
         \"\\u00f6\": \"Latin Small Letter O With Diaeresis\"\n}",
        "7b225c72223a2243617272696167652052657475726e222c2231223a224f6e65222c22c280223a22436f6e74726f6c222c22c3b6\
         223a224c6174696e20536d616c6c204c6574746572204f205769746820446961657265736973222c22e282ac223a224575726f20\
         5369676e222c22f09f9880223a22456d6f6a693a204772696e6e696e672046616365222c22efacb3223a22486562726577204c\
         65747465722044616c6574205769746820446167657368227d",
    ),
];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn check_rfc8785_vectors(canonicalise: impl Fn(&serde_json::Value) -> String) {
    for (input, want_hex) in RFC8785_VECTORS {
        let value: serde_json::Value =
            serde_json::from_str(input).expect("an RFC 8785 input parses");
        let got = canonicalise(&value);
        assert_eq!(
            hex(got.as_bytes()),
            *want_hex,
            "RFC 8785's vector is written {got}, not its published output"
        );
    }
}

#[test]
fn qa_canonical_jcs_rfc8785_vectors() {
    check_rfc8785_vectors(|v| canonical::json_to_string(v).unwrap());
}

validation::negative_control!(
    qa_canonical_jcs_rfc8785_vectors,
    "serde_json's compact writer (UTF-8 byte key order, its own number format) must fail the vector check",
    expected = "not its published output",
    check_rfc8785_vectors(|v| serde_json::to_string(v).unwrap())
);

/// A typed struct sorts its fields by their names' UTF-16 code units (RFC 8785 §3.2.3), not by UTF-8 bytes: U+1F600
/// (UTF-16 0xD83D…, UTF-8 0xF0…) before U+FB33 (0xFB33, UTF-8 0xEF…) before U+FF5E.
#[derive(Serialize)]
struct Utf16Keys {
    #[serde(rename = "\u{ff5e}")]
    fullwidth_tilde: u8,
    #[serde(rename = "\u{fb33}")]
    dalet: u8,
    #[serde(rename = "\u{1f600}")]
    grinning: u8,
    #[serde(rename = "z")]
    z: u8,
}

fn check_utf16_order(text: &str) {
    assert_eq!(
        text, "{\"z\":4,\"\u{1f600}\":3,\"\u{fb33}\":2,\"\u{ff5e}\":1}",
        "the keys are not sorted by UTF-16 code units"
    );
}

#[test]
fn qa_canonical_key_order_is_utf16() {
    let v = Utf16Keys {
        fullwidth_tilde: 1,
        dalet: 2,
        grinning: 3,
        z: 4,
    };
    check_utf16_order(&canonical::to_string(&v).unwrap());
    let map: BTreeMap<&str, i32> = [("\u{ff5e}", 1), ("\u{fb33}", 2), ("\u{1f600}", 3), ("z", 4)]
        .into_iter()
        .collect();
    check_utf16_order(&canonical::to_string(&map).unwrap());
}

validation::negative_control!(
    qa_canonical_key_order_is_utf16,
    "a UTF-8 byte order (BTreeMap's, through serde_json) must fail the UTF-16 order check",
    expected = "not sorted by UTF-16 code units",
    check_utf16_order(
        &serde_json::to_string(
            &[("\u{ff5e}", 1), ("\u{fb33}", 2), ("\u{1f600}", 3), ("z", 4)]
                .into_iter()
                .collect::<BTreeMap<&str, i32>>()
        )
        .unwrap()
    )
);

/// R-322: a u64 field is always a JSON string of its decimal digits, whatever its value (0, 1, 2^53, 2^53 + 1,
/// u64::MAX), at any depth and inside an Option; every other integer field is a JSON number, whatever its value. A
/// field's type never depends on its value.
#[derive(Serialize)]
struct Seeded {
    seed: u64,
    maybe_seed: Option<u64>,
    count: u32,
    offset: i64,
    small: i32,
    byte: u8,
    signed_byte: i8,
    half: u16,
    nested: SeededInner,
}

#[derive(Serialize)]
struct SeededInner {
    seed: u64,
    steps: i32,
}

fn seeded(seed: u64, int: i32) -> Seeded {
    Seeded {
        seed,
        maybe_seed: Some(seed),
        count: int.unsigned_abs(),
        offset: i64::from(int),
        small: int,
        byte: (int.unsigned_abs() % 256) as u8,
        signed_byte: (int % 128) as i8,
        half: (int.unsigned_abs() % 65536) as u16,
        nested: SeededInner { seed, steps: int },
    }
}

fn check_u64_strings(text: &str, seed: u64, int: i32) {
    let v: serde_json::Value = serde_json::from_str(text).expect("canonical text parses");
    for pointer in ["/seed", "/maybe_seed", "/nested/seed"] {
        assert_eq!(
            v.pointer(pointer),
            Some(&serde_json::Value::String(seed.to_string())),
            "the u64 field {pointer} = {seed} is not written as a string of its decimal digits: {text}"
        );
    }
    for pointer in [
        "/count",
        "/offset",
        "/small",
        "/byte",
        "/signed_byte",
        "/half",
        "/nested/steps",
    ] {
        assert!(
            v.pointer(pointer).is_some_and(serde_json::Value::is_number),
            "the non-u64 integer field {pointer} (from {int}) is not written as a number: {text}"
        );
    }
    // The string reads back to the same u64.
    let back: u64 = v["seed"].as_str().unwrap().parse().unwrap();
    assert_eq!(back, seed, "the seed's string does not read back: {text}");
}

#[test]
fn qa_canonical_integers() {
    let big = (1u64 << 53) + 1;
    for (seed, int) in [
        (0u64, 0i32),
        (1, 1),
        (1 << 53, -1),
        (big, i32::MAX),
        (u64::MAX, i32::MIN),
    ] {
        check_u64_strings(
            &canonical::to_string(&seeded(seed, int)).unwrap(),
            seed,
            int,
        );
    }
    // The exact text at 0 and at 2^53 + 1 (R-322's cases), members in JCS order.
    assert_eq!(
        canonical::to_string(&seeded(0, 0)).unwrap(),
        "{\"byte\":0,\"count\":0,\"half\":0,\"maybe_seed\":\"0\",\"nested\":{\"seed\":\"0\",\"steps\":0},\
\"offset\":0,\"seed\":\"0\",\"signed_byte\":0,\"small\":0}"
    );
    assert_eq!(
        canonical::to_string(&seeded(big, -7)).unwrap(),
        "{\"byte\":7,\"count\":7,\"half\":7,\"maybe_seed\":\"9007199254740993\",\"nested\":{\"seed\":\
\"9007199254740993\",\"steps\":-7},\"offset\":-7,\"seed\":\"9007199254740993\",\"signed_byte\":-7,\"small\":-7}"
    );
}

validation::negative_control!(
    qa_canonical_integers,
    "serde_json's writer, which writes a u64 as a number, must fail the u64-string check",
    expected = "is not written as a string of its decimal digits",
    check_u64_strings(&serde_json::to_string(&seeded(0, 0)).unwrap(), 0, 0)
);

/// The digits d₁…d_k of a JCS number and n, the value being 0.d₁…d_k × 10ⁿ, read from its text; checking on the way
/// that the text has the layout ECMAScript's Number-to-String gives that n (RFC 8785 §3.2.2.3): integer digits and
/// n − k zeros when k ≤ n ≤ 21; a point after the n-th digit when 0 < n ≤ 21; `0.`, −n zeros and the digits when
/// −6 < n ≤ 0; otherwise d₁, `.d₂…d_k` when k > 1, `e`, a sign and n − 1.
fn jcs_digits_and_n(text: &str) -> (String, i32) {
    let body = text.strip_prefix('-').unwrap_or(text);
    if let Some((mantissa, exp)) = body.split_once('e') {
        let (sign, mag) = exp.split_at(1);
        assert!(
            (sign == "+" || sign == "-")
                && !mag.is_empty()
                && !(mag.starts_with('0'))
                && mag.chars().all(|c| c.is_ascii_digit()),
            "{text}: the exponent is not a sign and digits with no leading zero"
        );
        let e: i32 = exp.parse().unwrap();
        let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        assert!(
            int.len() == 1 && int != "0",
            "{text}: not one non-zero digit before the point"
        );
        if mantissa.contains('.') {
            assert!(
                !frac.is_empty() && !frac.ends_with('0'),
                "{text}: a bare or zero-ended fraction"
            );
        }
        let n = e + 1;
        assert!(
            !(-6 < n && n <= 21),
            "{text}: n = {n} is written with an exponent, not positionally"
        );
        (format!("{int}{frac}"), n)
    } else if let Some((int, frac)) = body.split_once('.') {
        assert!(
            !frac.is_empty() && !frac.ends_with('0'),
            "{text}: a bare or zero-ended fraction"
        );
        if int == "0" {
            let digits = frac.trim_start_matches('0');
            let n = -((frac.len() - digits.len()) as i32);
            assert!(
                -6 < n,
                "{text}: n = {n} is written positionally, not with an exponent"
            );
            (digits.to_owned(), n)
        } else {
            assert!(!int.starts_with('0'), "{text}: a leading zero");
            let n = int.len() as i32;
            assert!(n <= 21, "{text}: n = {n} is written positionally");
            (format!("{int}{frac}"), n)
        }
    } else {
        assert!(
            !body.is_empty() && !body.starts_with('0') && body.chars().all(|c| c.is_ascii_digit()),
            "{text}: not an integer's digits"
        );
        let n = body.len() as i32;
        assert!(n <= 21, "{text}: n = {n} is written positionally");
        (body.trim_end_matches('0').to_owned(), n)
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

/// Every value reads back to the same bits through a text in JCS's layout, and its digits are the shortest that do
/// (one digit fewer, rounded either way, reads back differently) (RFC 8785 §3.2.2.3).
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
        let (digits, n) = jcs_digits_and_n(&text);
        let k = digits.len();
        if k > 1 {
            let shorter: u64 = digits[..k - 1].parse().unwrap();
            let sign = if v < 0.0 { "-" } else { "" };
            for cand in [shorter, shorter + 1] {
                let c = format!("{sign}{cand}e{}", n - (k as i32 - 1));
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
        1e-6,
        f64::from_bits(1e-6f64.to_bits() - 1),
        1e-7,
        1e20,
        1e21,
        f64::from_bits(1e21f64.to_bits() - 1),
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

/// JCS's string escapes (RFC 8785 §3.2.2.2): `"`, `\`, and U+0000–U+001F (the five short forms, else `\u00xx` in
/// lower-case hex);
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
