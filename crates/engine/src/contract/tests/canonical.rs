//! The canonical serialisation, JCS (RFC 8785; gui_state_contract §2, R-309, R-318, R-322; REQ-TOOL-145): RFC 8785's
//! published test vectors byte for byte, its number samples, −0.0 written `0`, a u64 field always a string and every
//! other number a number, each form of serde's data model, the refusals, and the skeleton's `SimConfig` and
//! `RenderState`. Each test registers its negative control (R-176).
//!
//! The vectors in `jcs/` are RFC 8785's published test data (its Appendix I, the JCS development portal's
//! `testdata/input` and `testdata/output`), each output the canonical form of its input. The number samples are RFC 8785
//! Appendix B, Table 1.

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;
use serde_json::Value;

use crate::contract::canonical::{self, format_f64, json_to_string};
use crate::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use crate::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, Links, Lock, Plane, Quality, SimConfig, Slice,
};

// ----- RFC 8785's published test vectors -----

/// Each vector's name, its input JSON and its canonical output, byte for byte.
fn vectors() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "arrays",
            include_str!("jcs/arrays.input.json"),
            include_str!("jcs/arrays.output.json"),
        ),
        (
            "french",
            include_str!("jcs/french.input.json"),
            include_str!("jcs/french.output.json"),
        ),
        (
            "structures",
            include_str!("jcs/structures.input.json"),
            include_str!("jcs/structures.output.json"),
        ),
        (
            "unicode",
            include_str!("jcs/unicode.input.json"),
            include_str!("jcs/unicode.output.json"),
        ),
        (
            "values",
            include_str!("jcs/values.input.json"),
            include_str!("jcs/values.output.json"),
        ),
        (
            "weird",
            include_str!("jcs/weird.input.json"),
            include_str!("jcs/weird.output.json"),
        ),
    ]
}

fn check_vectors(canonicalise: impl Fn(&Value) -> String) {
    for (name, input, output) in vectors() {
        let value: Value = serde_json::from_str(input).expect("a vector's input is not JSON");
        assert_eq!(
            canonicalise(&value).as_bytes(),
            output.as_bytes(),
            "the {name} vector is not serialised byte for byte"
        );
    }
}

#[test]
fn canonical_jcs_rfc8785_vectors() {
    check_vectors(|v| json_to_string(v).unwrap());
}

validation::negative_control!(
    canonical_jcs_rfc8785_vectors,
    "serde_json's own text (UTF-8 key order, its number format) must fail the vectors",
    expected = "is not serialised byte for byte",
    check_vectors(|v| serde_json::to_string(v).unwrap())
);

/// RFC 8785 §3.2.3's sorting sample: the values in the order the members must take, by their names' UTF-16 code
/// units; and §3.2.4's sample, the bytes of the `values` vector's canonical form.
fn check_sorting_and_bytes(canonicalise: impl Fn(&Value) -> String) {
    let sample: Value = serde_json::from_str(
        r#"{
            "€": "Euro Sign",
            "\r": "Carriage Return",
            "דּ": "Hebrew Letter Dalet With Dagesh",
            "1": "One",
            "😀": "Emoji: Grinning Face",
            "\u0080": "Control",
            "ö": "Latin Small Letter O With Diaeresis"
        }"#,
    )
    .unwrap();
    let text = canonicalise(&sample);
    let order = [
        "Carriage Return",
        "One",
        "Control",
        "Latin Small Letter O With Diaeresis",
        "Euro Sign",
        "Emoji: Grinning Face",
        "Hebrew Letter Dalet With Dagesh",
    ];
    let at: Vec<usize> = order
        .iter()
        .map(|v| text.find(&format!("\"{v}\"")).expect("a value is missing"))
        .collect();
    assert!(
        at.windows(2).all(|w| w[0] < w[1]),
        "the members are not in UTF-16 code-unit order: {text}"
    );
    let values: Value =
        serde_json::from_str(include_str!("jcs/values.input.json")).expect("not JSON");
    let hex = "7b 22 6c 69 74 65 72 61 6c 73 22 3a 5b 6e 75 6c 6c 2c 74 72 \
               75 65 2c 66 61 6c 73 65 5d 2c 22 6e 75 6d 62 65 72 73 22 3a \
               5b 33 33 33 33 33 33 33 33 33 2e 33 33 33 33 33 33 33 2c 31 \
               65 2b 33 30 2c 34 2e 35 2c 30 2e 30 30 32 2c 31 65 2d 32 37 \
               5d 2c 22 73 74 72 69 6e 67 22 3a 22 e2 82 ac 24 5c 75 30 30 \
               30 66 5c 6e 41 27 42 5c 22 5c 5c 5c 5c 5c 22 2f 22 7d";
    let bytes: Vec<u8> = hex
        .split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect();
    assert_eq!(
        canonicalise(&values).into_bytes(),
        bytes,
        "§3.2.4's bytes differ"
    );
}

#[test]
fn canonical_jcs_rfc8785_sorting_and_bytes() {
    check_sorting_and_bytes(|v| json_to_string(v).unwrap());
}

validation::negative_control!(
    canonical_jcs_rfc8785_sorting_and_bytes,
    "UTF-8 byte order puts U+FB33 before the emoji, and must fail",
    expected = "the members are not in UTF-16 code-unit order",
    check_sorting_and_bytes(|v| serde_json::to_string(v).unwrap())
);

// ----- RFC 8785 Appendix B: number serialisation samples -----

/// Each sample's IEEE 754 bits and its JSON text (RFC 8785 Appendix B, Table 1); NaN and Infinity, which have none,
/// are [`refusals`]'.
fn number_samples() -> Vec<(u64, &'static str)> {
    vec![
        (0x0000_0000_0000_0000, "0"),
        (0x8000_0000_0000_0000, "0"),
        (0x0000_0000_0000_0001, "5e-324"),
        (0x8000_0000_0000_0001, "-5e-324"),
        (0x7fef_ffff_ffff_ffff, "1.7976931348623157e+308"),
        (0xffef_ffff_ffff_ffff, "-1.7976931348623157e+308"),
        (0x4340_0000_0000_0000, "9007199254740992"),
        (0xc340_0000_0000_0000, "-9007199254740992"),
        (0x4430_0000_0000_0000, "295147905179352830000"),
        (0x44b5_2d02_c7e1_4af5, "9.999999999999997e+22"),
        (0x44b5_2d02_c7e1_4af6, "1e+23"),
        (0x44b5_2d02_c7e1_4af7, "1.0000000000000001e+23"),
        (0x444b_1ae4_d6e2_ef4e, "999999999999999700000"),
        (0x444b_1ae4_d6e2_ef4f, "999999999999999900000"),
        (0x444b_1ae4_d6e2_ef50, "1e+21"),
        (0x3eb0_c6f7_a0b5_ed8c, "9.999999999999997e-7"),
        (0x3eb0_c6f7_a0b5_ed8d, "0.000001"),
        (0x41b3_de43_5555_5553, "333333333.3333332"),
        (0x41b3_de43_5555_5554, "333333333.33333325"),
        (0x41b3_de43_5555_5555, "333333333.3333333"),
        (0x41b3_de43_5555_5556, "333333333.3333334"),
        (0x41b3_de43_5555_5557, "333333333.33333343"),
        (0xbecb_f647_612f_3696, "-0.0000033333333333333333"),
        (0x4314_3ff3_c1cb_0959, "1424953923781206.2"),
    ]
}

fn check_number_samples(format: impl Fn(f64) -> String) {
    for (bits, text) in number_samples() {
        let value = f64::from_bits(bits);
        assert_eq!(
            format(value),
            text,
            "{bits:016x} is not in JCS's number format"
        );
    }
}

#[test]
fn canonical_jcs_number_samples() {
    check_number_samples(|v| format_f64(v).unwrap());
}

validation::negative_control!(
    canonical_jcs_number_samples,
    "ryu's own layout (0.0, 1e23) must fail JCS's number format",
    expected = "is not in JCS's number format",
    check_number_samples(|v| ryu::Buffer::new().format_finite(v).to_owned())
);

/// Each layout boundary of ECMAScript's Number-to-String (RFC 8785 §3.2.2.3), beside Appendix B's samples.
fn boundaries() -> Vec<(f64, &'static str)> {
    vec![
        (1.0, "1"),
        (-2.5, "-2.5"),
        (1e20, "100000000000000000000"),
        (1.5e20, "150000000000000000000"),
        (1e21, "1e+21"),
        (123.456, "123.456"),
        (1e-6, "0.000001"),
        (1.5e-6, "0.0000015"),
        (1e-7, "1e-7"),
        (1.5e-7, "1.5e-7"),
        (0.1, "0.1"),
        (1e16, "10000000000000000"),
    ]
}

fn check_boundaries(format: impl Fn(f64) -> String) {
    for (value, text) in boundaries() {
        assert_eq!(
            format(value),
            text,
            "{value:e} is not in JCS's number format"
        );
    }
}

#[test]
fn canonical_jcs_number_boundaries() {
    check_boundaries(|v| format_f64(v).unwrap());
}

validation::negative_control!(
    canonical_jcs_number_boundaries,
    "Rust's Display layout (1e21 written out, no exponent sign) must fail JCS's number format",
    expected = "is not in JCS's number format",
    check_boundaries(|v| format!("{v}"))
);

/// Every value's text reads back to the same double: 20 000 finite bit patterns from a fixed generator. (−0.0 reads
/// back as 0, which R-318 accepts.)
fn check_read_back(format: impl Fn(f64) -> String) {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut checked = 0;
    while checked < 20_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let value = f64::from_bits(state);
        if !value.is_finite() {
            continue;
        }
        checked += 1;
        let text = format(value);
        let back: f64 = serde_json::from_str(&text).expect("not a JSON number");
        assert_eq!(
            back.to_bits(),
            value.to_bits(),
            "{value:e} is written {text}, which does not read back to it"
        );
    }
}

#[test]
fn canonical_jcs_numbers_read_back() {
    check_read_back(|v| format_f64(v).unwrap());
}

validation::negative_control!(
    canonical_jcs_numbers_read_back,
    "six significant digits must fail to read back",
    expected = "which does not read back to it",
    check_read_back(|v| format!("{v:.5e}"))
);

// ----- R-322: a u64 field is always a string; every other number a number -----

/// A seed and other numeric fields, declared out of key order.
#[derive(Serialize)]
struct Seeded {
    seed: u64,
    count: i64,
    small: u32,
    neg: i8,
    scale: f64,
    narrow: f32,
}

fn seeded(seed: u64) -> Seeded {
    Seeded {
        seed,
        count: 7,
        small: 3,
        neg: -4,
        scale: -0.0,
        narrow: 0.5,
    }
}

fn check_per_field(serialise: impl Fn(&Seeded) -> String) {
    for (seed, text) in [
        (
            0,
            r#"{"count":7,"narrow":0.5,"neg":-4,"scale":0,"seed":"0","small":3}"#,
        ),
        (
            (1u64 << 53) + 1,
            r#"{"count":7,"narrow":0.5,"neg":-4,"scale":0,"seed":"9007199254740993","small":3}"#,
        ),
        (
            u64::MAX,
            r#"{"count":7,"narrow":0.5,"neg":-4,"scale":0,"seed":"18446744073709551615","small":3}"#,
        ),
    ] {
        assert_eq!(
            serialise(&seeded(seed)),
            text,
            "a u64 field is not a string, or another number not a number"
        );
    }
}

#[test]
fn canonical_jcs_u64_field_is_a_string() {
    check_per_field(|v| canonical::to_string(v).unwrap());
}

validation::negative_control!(
    canonical_jcs_u64_field_is_a_string,
    "the per-value reading (a number up to 2^53) must fail the per-field rule",
    expected = "a u64 field is not a string",
    check_per_field(|v| {
        canonical::to_string(v)
            .unwrap()
            .replace(r#""seed":"0""#, r#""seed":0"#)
    })
);

// ----- each form of serde's data model -----

#[derive(Serialize)]
struct Unit;

#[derive(Serialize)]
struct Newtype(u8);

#[derive(Serialize)]
struct Pair(i8, bool);

/// Fields declared out of key order.
#[derive(Serialize)]
struct Fields {
    b: u16,
    a: i16,
}

#[derive(Serialize)]
enum Variant {
    Plain,
    Wrap(u32),
    Tuple(u8, u8),
    Named { z: i32, y: i64 },
    WrapF(f64),
    TupleF(u8, f64),
    NamedF { x: f64 },
}

#[derive(Serialize)]
enum KeyName {
    Beta,
    Alpha,
}

/// Bytes, as serde's `serialize_bytes` gives them.
struct Raw(&'static [u8]);

impl Serialize for Raw {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(self.0)
    }
}

/// A map whose entries arrive in the order given, as a hash map's may.
struct Entries<K, V>(Vec<(K, V)>);

impl<K: Serialize, V: Serialize> Serialize for Entries<K, V> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

/// A map that sends a value before any key.
struct ValueFirst;

impl Serialize for ValueFirst {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        map.serialize_value(&1u8)?;
        map.end()
    }
}

fn to<T: Serialize + ?Sized>(value: &T) -> String {
    canonical::to_string(value).expect("no canonical form")
}

fn err<T: Serialize + ?Sized>(value: &T) -> String {
    match canonical::to_string(value) {
        Ok(text) => text,
        Err(e) => format!("error: {e}"),
    }
}

fn skeleton() -> (SimConfig, RenderState) {
    (
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
        },
        RenderState {
            stain_graph: StainGraph {},
            overlays: Overlays {},
            palette: Palette {},
            playhead: Playhead {},
        },
    )
}

/// Each value's text, and the canonical text it must be.
fn cases() -> Vec<(String, &'static str)> {
    let (sim, render) = skeleton();
    vec![
        (to(&true), "true"),
        (to(&false), "false"),
        (to(&-8i8), "-8"),
        (to(&-16i16), "-16"),
        (to(&-32i32), "-32"),
        (to(&-(1i64 << 53)), "-9007199254740992"),
        (to(&i64::MIN), "-9223372036854776000"),
        (to(&8u8), "8"),
        (to(&16u16), "16"),
        (to(&32u32), "32"),
        (to(&u64::MAX), r#""18446744073709551615""#),
        (to(&0.1f32), "0.10000000149011612"),
        (to(&-0.0f32), "0"),
        (to(&-2.5f64), "-2.5"),
        (to(&1e30f64), "1e+30"),
        (to(&'é'), "\"é\""),
        (to(&'"'), r#""\"""#),
        (
            to("a\"b\\c\u{8}\t\n\u{c}\r\u{1}\u{1f}\u{7f} é/"),
            "\"a\\\"b\\\\c\\b\\t\\n\\f\\r\\u0001\\u001f\u{7f} é/\"",
        ),
        (to(&Raw(&[0, 255])), "[0,255]"),
        (to(&None::<u8>), "null"),
        (to(&Some(3u8)), "3"),
        (to(&Some(3u64)), r#""3""#),
        (to(&()), "null"),
        (to(&Unit), "null"),
        (to(&Variant::Plain), r#""Plain""#),
        (to(&Newtype(7)), "7"),
        (to(&Variant::Wrap(9)), r#"{"Wrap":9}"#),
        (to(&vec![3u8, 1, 2]), "[3,1,2]"),
        (to(&Vec::<u8>::new()), "[]"),
        (to(&(1u8, "x")), r#"[1,"x"]"#),
        (to(&Pair(-1, true)), "[-1,true]"),
        (to(&Variant::Tuple(1, 2)), r#"{"Tuple":[1,2]}"#),
        (to(&Fields { b: 1, a: 2 }), r#"{"a":2,"b":1}"#),
        (
            to(&Variant::Named { z: 1, y: 2 }),
            r#"{"Named":{"y":2,"z":1}}"#,
        ),
        (
            to(&Entries(vec![
                ("é", 1),
                ("b", 2),
                ("ab", 4),
                ("a", 3),
                ("", 0),
            ])),
            r#"{"":0,"a":3,"ab":4,"b":2,"é":1}"#,
        ),
        // UTF-16 code units, not UTF-8 bytes: U+FB33 (0xFB33) sorts after U+1F600 (0xD83D 0xDE00).
        (
            to(&Entries(vec![("\u{fb33}", 1), ("\u{1f600}", 2)])),
            "{\"\u{1f600}\":2,\"\u{fb33}\":1}",
        ),
        (to(&Entries::<&str, u8>(vec![])), "{}"),
        (to(&Entries(vec![('b', 1), ('a', 2)])), r#"{"a":2,"b":1}"#),
        (
            to(&Entries(vec![(9u64, 1), (10u64, 2)])),
            r#"{"10":2,"9":1}"#,
        ),
        (
            to(&Entries(vec![(KeyName::Beta, 1), (KeyName::Alpha, 2)])),
            r#"{"Alpha":2,"Beta":1}"#,
        ),
        // A key sorts by its unescaped form: '"' (0x22) before 'a'.
        (
            to(&Entries(vec![("a", 1), ("\"q", 2)])),
            r#"{"\"q":2,"a":1}"#,
        ),
        (
            to(&Entries(vec![("outer", Entries(vec![("z", 1), ("m", 2)]))])),
            r#"{"outer":{"m":2,"z":1}}"#,
        ),
        (
            to(&sim),
            r#"{"chart":{},"collision":{},"horizon":{},"integrator":{},"links":{},"lock":{},"plane":{},"quality":{},"slice":{}}"#,
        ),
        (
            to(&render),
            r#"{"overlays":{},"palette":{},"playhead":{},"stain_graph":{}}"#,
        ),
    ]
}

fn check_texts(cases: &[(String, &str)]) {
    for (got, want) in cases {
        assert_eq!(got, want, "not the canonical text");
    }
}

#[test]
fn canonical_jcs_each_form_has_its_text() {
    check_texts(&cases());
}

validation::negative_control!(
    canonical_jcs_each_form_has_its_text,
    "serde_json's declaration-order text must fail the canonical-text check",
    expected = "not the canonical text",
    check_texts(&[(
        serde_json::to_string(&Fields { b: 1, a: 2 }).unwrap(),
        r#"{"a":2,"b":1}"#
    )])
);

// ----- what has no canonical form -----

/// Values with no canonical form, and the start of each refusal.
fn refusals() -> Vec<(String, &'static str)> {
    let json = |text: &str| {
        let value: Value = serde_json::from_str(text).unwrap();
        json_to_string(&value).unwrap_or_else(|e| format!("error: {e}"))
    };
    vec![
        (err(&f64::NAN), "error: NaN has no JSON form"),
        (err(&f32::INFINITY), "error: inf has no JSON form"),
        (
            err(&[1.0, f64::NEG_INFINITY]),
            "error: -inf has no JSON form",
        ),
        (err(&Some(f64::NAN)), "error: NaN"),
        (err(&Variant::WrapF(f64::NAN)), "error: NaN"),
        (err(&Variant::TupleF(1, f64::NAN)), "error: NaN"),
        (err(&Variant::NamedF { x: f64::NAN }), "error: NaN"),
        (err(&(1u8, f64::NAN)), "error: NaN"),
        (err(&Entries(vec![("a", f64::NAN)])), "error: NaN"),
        (
            err(&((1i64 << 53) + 1)),
            "error: 9007199254740993 is not exactly an IEEE 754 double",
        ),
        (
            err(&i64::MAX),
            "error: 9223372036854775807 is not exactly an IEEE 754 double",
        ),
        (
            json("9007199254740993"),
            "error: 9007199254740993 is not exactly an IEEE 754 double",
        ),
        (
            json("[-9007199254740993]"),
            "error: -9007199254740993 is not exactly an IEEE 754 double",
        ),
        (
            err(&Entries(vec![(1u8, 1u8)])),
            "error: a map key must be a string, not 1",
        ),
        (
            err(&Entries(vec![(true, 1u8)])),
            "error: a map key must be a string",
        ),
        (
            err(&Entries(vec![("a", 1), ("b", 2), ("a", 3)])),
            "error: the key \"a\" appears twice",
        ),
        (
            err(&ValueFirst),
            "error: a map value arrived before its key",
        ),
    ]
}

fn check_refused(cases: &[(String, &str)]) {
    for (got, want) in cases {
        assert!(got.starts_with(want), "{got:?} is not refused as {want:?}");
    }
}

#[test]
fn canonical_jcs_refuses_what_has_no_form() {
    check_refused(&refusals());
}

validation::negative_control!(
    canonical_jcs_refuses_what_has_no_form,
    "a finite value must fail the refusal check",
    expected = "is not refused",
    check_refused(&[(err(&1.0f64), "error: NaN")])
);

// ----- the skeleton: the same bytes twice, and read back -----

/// Two serialisations of the skeleton are the same bytes, and the text reads back to the same values and writes the
/// same text again.
fn check_round_trip(sim_text: &str, render_text: &str) {
    let (sim0, render0) = skeleton();
    let (sim1, render1) = skeleton();
    assert_eq!(
        (to(&sim0).into_bytes(), to(&render0).into_bytes()),
        (to(&sim1).into_bytes(), to(&render1).into_bytes()),
        "the same state is not the same bytes"
    );
    let sim: SimConfig = serde_json::from_str(sim_text).expect("not a SimConfig");
    let render: RenderState = serde_json::from_str(render_text).expect("not a RenderState");
    assert_eq!(
        (&sim, &render),
        (&sim0, &render0),
        "the text does not read back to the same values"
    );
    assert_eq!(
        (to(&sim), to(&render)),
        (sim_text.to_owned(), render_text.to_owned()),
        "the text does not round-trip"
    );
}

#[test]
fn canonical_jcs_skeleton_round_trips() {
    let (sim, render) = skeleton();
    check_round_trip(&to(&sim), &to(&render));
}

validation::negative_control!(
    canonical_jcs_skeleton_round_trips,
    "a text in another key order must fail the round trip",
    expected = "the text does not round-trip",
    check_round_trip(
        r#"{"slice":{},"chart":{},"collision":{},"horizon":{},"integrator":{},"links":{},"lock":{},"plane":{},"quality":{}}"#,
        r#"{"overlays":{},"palette":{},"playhead":{},"stain_graph":{}}"#
    )
);
