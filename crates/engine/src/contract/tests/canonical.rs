//! The canonical serialisation (gui_state_contract §2, R-309, REQ-TOOL-145): each form of serde's data model written
//! as its canonical text, the key order at every depth, the refusals, and the skeleton's `SimConfig` and
//! `RenderState`. Each test registers its negative control (R-176).

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

use crate::contract::canonical::{self, format_f64};
use crate::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use crate::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, Links, Lock, Plane, Quality, SimConfig, Slice,
};

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
        (to(&i64::MIN), "-9223372036854775808"),
        (to(&8u8), "8"),
        (to(&16u16), "16"),
        (to(&32u32), "32"),
        (to(&u64::MAX), "18446744073709551615"),
        (to(&0.1f32), "0.10000000149011612"),
        (to(&-2.5f64), "-2.5"),
        (to(&1e16f64), "1e16"),
        (to(&'é'), "\"é\""),
        (to(&'"'), r#""\"""#),
        (
            to("a\"b\\c\u{8}\t\n\u{c}\r\u{1}\u{1f} é/"),
            r#""a\"b\\c\b\t\n\f\r\u0001\u001f é/""#,
        ),
        (to(&Raw(&[0, 255])), "[0,255]"),
        (to(&None::<u8>), "null"),
        (to(&Some(3u8)), "3"),
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
            to(&Entries(vec![("é", 1), ("b", 2), ("ab", 4), ("a", 3)])),
            r#"{"a":3,"ab":4,"b":2,"é":1}"#,
        ),
        (to(&Entries::<&str, u8>(vec![])), "{}"),
        (to(&Entries(vec![('b', 1), ('a', 2)])), r#"{"a":2,"b":1}"#),
        (
            to(&Entries(vec![(KeyName::Beta, 1), (KeyName::Alpha, 2)])),
            r#"{"Alpha":2,"Beta":1}"#,
        ),
        // A key with an escape sorts by its own bytes, '"' (0x22) before 'a'.
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
fn canonical_each_form_has_its_text() {
    check_texts(&cases());
}

validation::negative_control!(
    canonical_each_form_has_its_text,
    "serde_json's declaration-order text must fail the canonical-text check",
    expected = "not the canonical text",
    check_texts(&[(
        serde_json::to_string(&Fields { b: 1, a: 2 }).unwrap(),
        r#"{"a":2,"b":1}"#
    )])
);

/// Values with no canonical form, and the start of each refusal.
fn refusals() -> Vec<(String, &'static str)> {
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
fn canonical_refuses_what_has_no_form() {
    check_refused(&refusals());
}

validation::negative_control!(
    canonical_refuses_what_has_no_form,
    "a finite value must fail the refusal check",
    expected = "is not refused",
    check_refused(&[(err(&1.0f64), "error: NaN")])
);

/// The float layout of gui_state_contract §2, and each read back to the same bits.
const LAYOUT: &[(f64, &str)] = &[
    (0.0, "0.0"),
    (-0.0, "-0.0"),
    (1.0, "1.0"),
    (1234.0, "1234.0"),
    (12.34, "12.34"),
    (0.001234, "0.001234"),
    (1e-5, "0.00001"),
    (1e-6, "1e-6"),
    (1.5e-7, "1.5e-7"),
    (1e15, "1000000000000000.0"),
    (1e16, "1e16"),
    (5e-324, "5e-324"),
];

fn check_layout(format: impl Fn(f64) -> String) {
    for (value, text) in LAYOUT {
        let got = format(*value);
        assert_eq!(got, *text, "{value:e} is not in the canonical layout");
        let back: f64 = serde_json::from_str(&got).expect("not a JSON number");
        assert_eq!(back.to_bits(), value.to_bits(), "{got} does not read back");
    }
}

#[test]
fn canonical_float_layout() {
    check_layout(|v| format_f64(v).unwrap());
}

validation::negative_control!(
    canonical_float_layout,
    "Rust's Display layout must fail the canonical layout",
    expected = "is not in the canonical layout",
    check_layout(|v| format!("{v}"))
);

/// The skeleton reads back from its canonical text, and writes the same text again.
fn check_round_trip(sim_text: &str, render_text: &str) {
    let sim: SimConfig = serde_json::from_str(sim_text).expect("not a SimConfig");
    let render: RenderState = serde_json::from_str(render_text).expect("not a RenderState");
    assert_eq!(
        (to(&sim), to(&render)),
        (sim_text.to_owned(), render_text.to_owned()),
        "the text does not round-trip"
    );
}

#[test]
fn canonical_skeleton_round_trips() {
    let (sim, render) = skeleton();
    check_round_trip(&to(&sim), &to(&render));
}

validation::negative_control!(
    canonical_skeleton_round_trips,
    "a text in another key order must fail the round trip",
    expected = "the text does not round-trip",
    check_round_trip(
        r#"{"slice":{},"chart":{},"collision":{},"horizon":{},"integrator":{},"links":{},"lock":{},"plane":{},"quality":{}}"#,
        r#"{"overlays":{},"palette":{},"playhead":{},"stain_graph":{}}"#
    )
);
