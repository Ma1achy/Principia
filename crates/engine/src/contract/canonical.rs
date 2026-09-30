//! The one canonical serialisation of [`SimConfig`](super::sim_config::SimConfig) and
//! [`RenderState`](super::render_state::RenderState) (gui_state_contract §2, "One canonical serialisation", R-309;
//! the definition is REQ-TOOL-145's). Equal state is equal text: compact JSON, every object's keys in ascending order
//! of their UTF-8 bytes, integers in plain decimal, and each float from its shortest round-trip digits in one fixed
//! layout, so each value reads back exactly. The profiler header's `config`, snapshot JSON, share links and pxpack
//! all carry this text.

use std::fmt;

use serde::ser::{self, Serialize};

/// Why a value has no canonical form: a NaN or an infinity, a map key that is not a string, or two equal keys in one
/// object.
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

/// `value` in the canonical serialisation (gui_state_contract §2).
pub fn to_string<T: Serialize + ?Sized>(value: &T) -> Result<String, Error> {
    value.serialize(Canonical)
}

/// An f64 as the canonical serialisation writes it (gui_state_contract §2): `0.0` and `-0.0` for the zeros; otherwise
/// the shortest digits that read back to the same f64 (the nearest of them, ties to an even last digit), positional
/// for a decimal exponent from −5 to 15 (`1234.0`, `12.34`, `0.001234`) and scientific outside it (`1e16`,
/// `1.5e-7`). NaN and the infinities have no JSON form, and are an error.
pub fn format_f64(value: f64) -> Result<String, Error> {
    if !value.is_finite() {
        return Err(Error(format!(
            "{value} has no JSON form, so no canonical serialisation"
        )));
    }
    // ryu's shortest round-trip digits, nearest with ties to even, in the layout above.
    Ok(ryu::Buffer::new().format_finite(value).to_owned())
}

/// A string as the canonical serialisation writes it: `"` and `\` escaped, and the controls U+0000 to U+001F, as
/// `\b`, `\t`, `\n`, `\f`, `\r` or `\u00xx` in lower-case hex; every other character as itself.
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
        self.members
            .sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
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
        Ok(v.to_string())
    }
    fn serialize_i16(self, v: i16) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i32(self, v: i32) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i64(self, v: i64) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u8(self, v: u8) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u16(self, v: u16) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u32(self, v: u32) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u64(self, v: u64) -> Result<String, Error> {
        Ok(v.to_string())
    }
    /// An f32 is written as the f64 it widens to, exactly, so it reads back to the same f32.
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
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        self.key = Some(key.serialize(Key)?);
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

/// A map key: JSON keys are strings, so only a string, a char or a unit variant's name is one.
struct Key;

fn not_a_key<T>(what: &str) -> Result<T, Error> {
    Err(Error(format!(
        "a map key must be a string, not {what}: JSON has no other key"
    )))
}

impl ser::Serializer for Key {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = ser::Impossible<String, Error>;
    type SerializeTuple = ser::Impossible<String, Error>;
    type SerializeTupleStruct = ser::Impossible<String, Error>;
    type SerializeTupleVariant = ser::Impossible<String, Error>;
    type SerializeMap = ser::Impossible<String, Error>;
    type SerializeStruct = ser::Impossible<String, Error>;
    type SerializeStructVariant = ser::Impossible<String, Error>;

    fn serialize_str(self, v: &str) -> Result<String, Error> {
        Ok(v.to_owned())
    }
    fn serialize_char(self, v: char) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<String, Error> {
        Ok(variant.to_owned())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<String, Error> {
        value.serialize(self)
    }
    fn serialize_bool(self, _v: bool) -> Result<String, Error> {
        not_a_key("a bool")
    }
    fn serialize_i8(self, _v: i8) -> Result<String, Error> {
        not_a_key("a number")
    }
    fn serialize_i16(self, _v: i16) -> Result<String, Error> {
        not_a_key("a number")
    }
    fn serialize_i32(self, _v: i32) -> Result<String, Error> {
        not_a_key("a number")
    }
    fn serialize_i64(self, _v: i64) -> Result<String, Error> {
        not_a_key("a number")
    }
    fn serialize_u8(self, _v: u8) -> Result<String, Error> {
        not_a_key("a number")
    }
    fn serialize_u16(self, _v: u16) -> Result<String, Error> {
        not_a_key("a number")
    }
    fn serialize_u32(self, _v: u32) -> Result<String, Error> {
        not_a_key("a number")
    }
    fn serialize_u64(self, _v: u64) -> Result<String, Error> {
        not_a_key("a number")
    }
    fn serialize_f32(self, _v: f32) -> Result<String, Error> {
        not_a_key("a number")
    }
    fn serialize_f64(self, _v: f64) -> Result<String, Error> {
        not_a_key("a number")
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<String, Error> {
        not_a_key("bytes")
    }
    fn serialize_none(self) -> Result<String, Error> {
        not_a_key("null")
    }
    fn serialize_some<T: Serialize + ?Sized>(self, _value: &T) -> Result<String, Error> {
        not_a_key("an option")
    }
    fn serialize_unit(self) -> Result<String, Error> {
        not_a_key("null")
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<String, Error> {
        not_a_key("null")
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<String, Error> {
        not_a_key("an object")
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        not_a_key("an array")
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        not_a_key("an array")
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        not_a_key("an array")
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        not_a_key("an object")
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        not_a_key("an object")
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        not_a_key("an object")
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        not_a_key("an object")
    }
}
