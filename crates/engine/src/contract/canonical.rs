//! The one canonical serialisation of [`SimConfig`](super::sim_config::SimConfig) and
//! [`RenderState`](super::render_state::RenderState): JCS, RFC 8785 (gui_state_contract §2, R-309, R-318, R-322;
//! REQ-TOOL-145). Equal state is equal text: no whitespace between tokens, every object's members sorted by their
//! names' UTF-16 code units at every depth, strings escaped as RFC 8785 §3.2.2.2 gives, and numbers in ECMAScript's
//! Number-to-String format (§3.2.2.3), so −0.0 is written `0`. A u64 field (a seed, say) is always written as a JSON
//! string of its decimal digits, whatever its value; every other number is a JSON number, so a field's type never
//! depends on its value (R-322). The profiler header's `config`, snapshot JSON, share links and pxpack all carry this
//! text.

use std::fmt;

use serde::ser::{self, Serialize};
use serde_json::{Number, Value};

/// Why a value has no canonical form: a NaN or an infinity, an integer no IEEE 754 double holds exactly, a map key
/// that is not a string, or two equal keys in one object (RFC 8785 §3.1, §3.2.2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

/// `value` in the canonical serialisation (gui_state_contract §2): JCS, a u64 field written as a string (R-322).
pub fn to_string<T: Serialize + ?Sized>(value: &T) -> Result<String, Error> {
    value.serialize(Canonical)
}

/// JSON data in JCS (RFC 8785 §3.2): what the profiler header's `config` is written as (telemetry §5). Here a value
/// carries no Rust field type, so every JSON number is a number, read as the IEEE 754 double JCS holds it as; a JSON
/// string stays a string, so a u64 field written as one by [`to_string`] is written the same again.
pub fn json_to_string(value: &Value) -> Result<String, Error> {
    Json(value).serialize(Canonical)
}

/// A JSON value, serialised with every number as a double (RFC 8785 §3.2.2.3).
struct Json<'a>(&'a Value);

impl Serialize for Json<'_> {
    fn serialize<S: ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Null => s.serialize_unit(),
            Value::Bool(b) => s.serialize_bool(*b),
            Value::Number(n) => s.serialize_f64(double(n).map_err(ser::Error::custom)?),
            Value::String(t) => s.serialize_str(t),
            Value::Array(items) => s.collect_seq(items.iter().map(Json)),
            Value::Object(members) => s.collect_map(members.iter().map(|(k, v)| (k, Json(v)))),
        }
    }
}

/// A JSON number as the double JCS holds it as; an integer no double holds exactly is not I-JSON (RFC 8785 §3.1).
fn double(n: &Number) -> Result<f64, Error> {
    match (n.as_u64(), n.as_i64(), n.as_f64()) {
        (Some(u), _, _) => exact(i128::from(u)),
        (None, Some(i), _) => exact(i128::from(i)),
        (None, None, Some(f)) => Ok(f),
        (None, None, None) => Err(Error(format!("{n} is not a JSON number JCS can hold"))),
    }
}

/// An integer as the double that holds it exactly; one no double holds is refused, since JCS numbers are IEEE 754
/// doubles (RFC 8785 §3.1).
fn exact(i: i128) -> Result<f64, Error> {
    // The conversion rounds to nearest; the integer is exact when the double converts back to it. A double from an i64
    // or a u64 is within i128's range, so the conversion back saturates nowhere.
    let f = i as f64;
    let back = f as i128;
    if back == i {
        Ok(f)
    } else {
        Err(Error(format!(
            "{i} is not exactly an IEEE 754 double, so JCS has no number for it (RFC 8785 §3.1)"
        )))
    }
}

/// An f64 in JCS's number format (RFC 8785 §3.2.2.3, ECMAScript's Number-to-String): `0` for both zeros; otherwise
/// the shortest digits d₁…d_k that read back to the same f64 (the nearest of them, ties to even), with n the position
/// of the decimal point (the value is 0.d₁…d_k × 10ⁿ): plain digits and n − k zeros when k ≤ n ≤ 21; the point after
/// the n-th digit when 0 < n ≤ 21; `0.`, −n zeros and the digits when −6 < n ≤ 0; otherwise d₁, `.` and d₂…d_k when
/// k > 1, `e`, the sign of n − 1 (`+` or `-`) and its magnitude (`1e+21`, `1.5e-7`). A negative value has `-` before
/// it. NaN and the infinities have no JSON form, and are an error.
pub fn format_f64(value: f64) -> Result<String, Error> {
    if !value.is_finite() {
        return Err(Error(format!(
            "{value} has no JSON form, so no canonical serialisation"
        )));
    }
    if value == 0.0 {
        return Ok("0".to_owned());
    }
    // ryu's shortest round-trip digits, nearest with ties to even (RFC 8785 §3.2.2.3 names it), read out of its text.
    let mut buffer = ryu::Buffer::new();
    let text = buffer.format_finite(value.abs());
    let (mantissa, exponent) = match text.split_once('e') {
        Some((m, e)) => (m, e.parse::<isize>().map_err(|e| Error(e.to_string()))?),
        None => (text, 0),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all = format!("{int}{frac}");
    let significant = all.trim_start_matches('0');
    let digits = significant.trim_end_matches('0');
    // The value is 0.d₁…d_k × 10ⁿ: the point sits after `int`, moved by the exponent, less the leading zeros dropped.
    // Each length is a few tens of characters at most, so the casts are exact.
    let k = digits.len() as isize;
    let n = int.len() as isize + exponent - (all.len() - significant.len()) as isize;
    let mut out = String::new();
    if value < 0.0 {
        out.push('-');
    }
    if k <= n && n <= 21 {
        out.push_str(digits);
        out.push_str(&"0".repeat((n - k).unsigned_abs()));
    } else if 0 < n && n <= 21 {
        let (before, after) = digits.split_at(n.unsigned_abs());
        out.push_str(before);
        out.push('.');
        out.push_str(after);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.push_str(&"0".repeat(n.unsigned_abs()));
        out.push_str(digits);
    } else {
        let (first, rest) = digits.split_at(1);
        out.push_str(first);
        if !rest.is_empty() {
            out.push('.');
            out.push_str(rest);
        }
        let e = n - 1;
        out.push('e');
        out.push(if e < 0 { '-' } else { '+' });
        out.push_str(&e.unsigned_abs().to_string());
    }
    Ok(out)
}

/// A string as JCS writes it (RFC 8785 §3.2.2.2): `"` and `\` escaped; the controls U+0000 to U+001F as `\b`, `\t`,
/// `\n`, `\f`, `\r` or `\u00xx` in lower-case hex; every other character as itself.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The serializer: each value becomes its canonical text.
struct Canonical;

/// An array's elements, in order.
struct Array(Vec<String>);

/// An object's members, sorted by key when it ends; `key` holds a map key until its value arrives.
struct Object {
    members: Vec<(String, String)>,
    key: Option<String>,
}

/// A variant's content, written as `{"variant": content}`.
struct Variant<T> {
    name: &'static str,
    inner: T,
}

impl Object {
    fn new(len: usize) -> Self {
        Object {
            members: Vec::with_capacity(len),
            key: None,
        }
    }

    fn end(mut self) -> Result<String, Error> {
        // RFC 8785 §3.2.3: by the names' UTF-16 code units, unescaped, compared as unsigned integers.
        self.members
            .sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
        if let Some(pair) = self.members.windows(2).find(|w| w[0].0 == w[1].0) {
            return Err(Error(format!(
                "the key {:?} appears twice in one object",
                pair[0].0
            )));
        }
        let body: Vec<String> = self
            .members
            .into_iter()
            .map(|(k, v)| format!("{}:{v}", quote(&k)))
            .collect();
        Ok(format!("{{{}}}", body.join(",")))
    }
}

fn array(items: Vec<String>) -> String {
    format!("[{}]", items.join(","))
}

fn tagged(name: &str, inner: String) -> String {
    format!("{{{}:{inner}}}", quote(name))
}

impl ser::Serializer for Canonical {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = Array;
    type SerializeTuple = Array;
    type SerializeTupleStruct = Array;
    type SerializeTupleVariant = Variant<Array>;
    type SerializeMap = Object;
    type SerializeStruct = Object;
    type SerializeStructVariant = Variant<Object>;

    fn serialize_bool(self, v: bool) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i8(self, v: i8) -> Result<String, Error> {
        format_f64(f64::from(v))
    }
    fn serialize_i16(self, v: i16) -> Result<String, Error> {
        format_f64(f64::from(v))
    }
    fn serialize_i32(self, v: i32) -> Result<String, Error> {
        format_f64(f64::from(v))
    }
    /// An i64 is a JSON number, so it must be a double exactly (RFC 8785 §3.1); one beyond that is refused.
    fn serialize_i64(self, v: i64) -> Result<String, Error> {
        format_f64(exact(i128::from(v))?)
    }
    fn serialize_u8(self, v: u8) -> Result<String, Error> {
        format_f64(f64::from(v))
    }
    fn serialize_u16(self, v: u16) -> Result<String, Error> {
        format_f64(f64::from(v))
    }
    fn serialize_u32(self, v: u32) -> Result<String, Error> {
        format_f64(f64::from(v))
    }
    /// A u64 field (a seed, say) is always a JSON string of its decimal digits, whatever its value (R-322).
    fn serialize_u64(self, v: u64) -> Result<String, Error> {
        Ok(quote(&v.to_string()))
    }
    /// An f32 is written as the f64 it widens to, exactly.
    fn serialize_f32(self, v: f32) -> Result<String, Error> {
        format_f64(f64::from(v))
    }
    fn serialize_f64(self, v: f64) -> Result<String, Error> {
        format_f64(v)
    }
    fn serialize_char(self, v: char) -> Result<String, Error> {
        Ok(quote(v.encode_utf8(&mut [0; 4])))
    }
    fn serialize_str(self, v: &str) -> Result<String, Error> {
        Ok(quote(v))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<String, Error> {
        Ok(array(v.iter().map(u8::to_string).collect()))
    }
    fn serialize_none(self) -> Result<String, Error> {
        Ok("null".to_owned())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<String, Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<String, Error> {
        Ok("null".to_owned())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<String, Error> {
        Ok("null".to_owned())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<String, Error> {
        Ok(quote(variant))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<String, Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<String, Error> {
        Ok(tagged(variant, value.serialize(Canonical)?))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Array, Error> {
        Ok(Array(Vec::with_capacity(len.unwrap_or(0))))
    }
    fn serialize_tuple(self, len: usize) -> Result<Array, Error> {
        Ok(Array(Vec::with_capacity(len)))
    }
    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Array, Error> {
        Ok(Array(Vec::with_capacity(len)))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Variant<Array>, Error> {
        Ok(Variant {
            name: variant,
            inner: Array(Vec::with_capacity(len)),
        })
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Object, Error> {
        Ok(Object::new(len.unwrap_or(0)))
    }
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Object, Error> {
        Ok(Object::new(len))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Variant<Object>, Error> {
        Ok(Variant {
            name: variant,
            inner: Object::new(len),
        })
    }
}

impl ser::SerializeSeq for Array {
    type Ok = String;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.0.push(value.serialize(Canonical)?);
        Ok(())
    }
    fn end(self) -> Result<String, Error> {
        Ok(array(self.0))
    }
}

impl ser::SerializeTuple for Array {
    type Ok = String;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<String, Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for Array {
    type Ok = String;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<String, Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleVariant for Variant<Array> {
    type Ok = String;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(&mut self.inner, value)
    }
    fn end(self) -> Result<String, Error> {
        Ok(tagged(self.name, array(self.inner.0)))
    }
}

impl ser::SerializeMap for Object {
    type Ok = String;
    type Error = Error;
    /// JSON keys are strings: a key must serialise to a string (a string, a char, a unit variant's name, a u64), and is
    /// held unescaped, so the object sorts by the key's own code units (RFC 8785 §3.2.3).
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        let text = key.serialize(Canonical)?;
        let key = if text.starts_with('"') {
            serde_json::from_str(&text).map_err(|e| Error(e.to_string()))?
        } else {
            return Err(Error(format!(
                "a map key must be a string, not {text}: JSON has no other key"
            )));
        };
        self.key = Some(key);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error("a map value arrived before its key".to_owned()))?;
        self.members.push((key, value.serialize(Canonical)?));
        Ok(())
    }
    fn end(self) -> Result<String, Error> {
        Object::end(self)
    }
}

impl ser::SerializeStruct for Object {
    type Ok = String;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.members
            .push((key.to_owned(), value.serialize(Canonical)?));
        Ok(())
    }
    fn end(self) -> Result<String, Error> {
        Object::end(self)
    }
}

impl ser::SerializeStructVariant for Variant<Object> {
    type Ok = String;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        ser::SerializeStruct::serialize_field(&mut self.inner, key, value)
    }
    fn end(self) -> Result<String, Error> {
        Ok(tagged(self.name, self.inner.end()?))
    }
}
