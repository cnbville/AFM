//! `lean`: a value as the game log writes it, made in one pass. It is
//! `serde_json::to_value` and `games::compact` together: a struct's or
//! map's field that comes out null, false, zero, "" or empty is left out
//! (an array keeps its places), and decimals are rounded the log's way
//! (`games::round`; a whole decimal becomes a whole number). Only the
//! fields that stay get a key, so a player's line or a snap, mostly
//! zeros, costs a fraction of building every field and dropping most.

use serde::ser::{self, Serialize};
use serde_json::{Error, Map, Number, Value};

use super::games::{empty, round};

/// `x` as the log writes it.
pub(crate) fn lean<T: Serialize + ?Sized>(x: &T) -> Value {
    x.serialize(Lean).unwrap_or(Value::Null)
}

/// A decimal as the log writes it: rounded, and whole when it is whole.
pub(crate) fn num(x: f64) -> Value {
    let r = round(x);
    if r.fract() == 0.0 && r.abs() < 9.0e15 {
        Value::from(r as i64)
    } else {
        Number::from_f64(r).map_or(Value::Null, Value::Number)
    }
}

struct Lean;

/// A struct's or map's kept fields; a variant's name when it has one.
pub(crate) struct Fields {
    map: Map<String, Value>,
    key: Option<String>,
    variant: Option<&'static str>,
}

impl Fields {
    fn put(&mut self, k: String, v: Value) {
        if !empty(&v) {
            self.map.insert(k, v);
        }
    }
}

/// A sequence's parts, every place kept; a variant's name when it has one.
pub(crate) struct Parts {
    items: Vec<Value>,
    variant: Option<&'static str>,
}

fn named(variant: Option<&'static str>, v: Value) -> Value {
    match variant {
        Some(name) => {
            let mut m = Map::new();
            m.insert(name.to_string(), v);
            Value::Object(m)
        }
        None => v,
    }
}

impl ser::Serializer for Lean {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = Parts;
    type SerializeTuple = Parts;
    type SerializeTupleStruct = Parts;
    type SerializeTupleVariant = Parts;
    type SerializeMap = Fields;
    type SerializeStruct = Fields;
    type SerializeStructVariant = Fields;

    fn serialize_bool(self, v: bool) -> Result<Value, Error> {
        Ok(Value::Bool(v))
    }
    fn serialize_i8(self, v: i8) -> Result<Value, Error> {
        Ok(v.into())
    }
    fn serialize_i16(self, v: i16) -> Result<Value, Error> {
        Ok(v.into())
    }
    fn serialize_i32(self, v: i32) -> Result<Value, Error> {
        Ok(v.into())
    }
    fn serialize_i64(self, v: i64) -> Result<Value, Error> {
        Ok(v.into())
    }
    fn serialize_u8(self, v: u8) -> Result<Value, Error> {
        Ok(v.into())
    }
    fn serialize_u16(self, v: u16) -> Result<Value, Error> {
        Ok(v.into())
    }
    fn serialize_u32(self, v: u32) -> Result<Value, Error> {
        Ok(v.into())
    }
    fn serialize_u64(self, v: u64) -> Result<Value, Error> {
        Ok(v.into())
    }
    fn serialize_f32(self, v: f32) -> Result<Value, Error> {
        Ok(num(v as f64))
    }
    fn serialize_f64(self, v: f64) -> Result<Value, Error> {
        Ok(num(v))
    }
    fn serialize_char(self, v: char) -> Result<Value, Error> {
        Ok(Value::String(v.to_string()))
    }
    fn serialize_str(self, v: &str) -> Result<Value, Error> {
        Ok(Value::String(v.to_string()))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Value, Error> {
        Ok(Value::Array(v.iter().map(|&b| b.into()).collect()))
    }
    fn serialize_none(self) -> Result<Value, Error> {
        Ok(Value::Null)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<Value, Error> {
        v.serialize(Lean)
    }
    fn serialize_unit(self) -> Result<Value, Error> {
        Ok(Value::Null)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<Value, Error> {
        Ok(Value::Null)
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, variant: &'static str) -> Result<Value, Error> {
        Ok(Value::String(variant.to_string()))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _: &'static str, v: &T) -> Result<Value, Error> {
        v.serialize(Lean)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        v: &T,
    ) -> Result<Value, Error> {
        Ok(named(Some(variant), v.serialize(Lean)?))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Parts, Error> {
        Ok(Parts { items: Vec::with_capacity(len.unwrap_or(0)), variant: None })
    }
    fn serialize_tuple(self, len: usize) -> Result<Parts, Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(self, _: &'static str, len: usize) -> Result<Parts, Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Parts, Error> {
        Ok(Parts { items: Vec::with_capacity(len), variant: Some(variant) })
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Fields, Error> {
        Ok(Fields { map: Map::new(), key: None, variant: None })
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Fields, Error> {
        self.serialize_map(None)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Fields, Error> {
        Ok(Fields { map: Map::new(), key: None, variant: Some(variant) })
    }
}

impl ser::SerializeSeq for Parts {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        self.items.push(v.serialize(Lean)?);
        Ok(())
    }
    fn end(self) -> Result<Value, Error> {
        Ok(named(self.variant, Value::Array(self.items)))
    }
}

impl ser::SerializeTuple for Parts {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<Value, Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for Parts {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<Value, Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleVariant for Parts {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<Value, Error> {
        ser::SerializeSeq::end(self)
    }
}

/// A map key as text, the way `serde_json` writes one.
fn key(v: Value) -> Result<String, Error> {
    match v {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(ser::Error::custom("a map key must be text, a number or a bool")),
    }
}

impl ser::SerializeMap for Fields {
    type Ok = Value;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, k: &T) -> Result<(), Error> {
        self.key = Some(key(k.serialize(Lean)?)?);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        let k = self.key.take().ok_or_else(|| ser::Error::custom("a map value without its key"))?;
        let v = v.serialize(Lean)?;
        self.put(k, v);
        Ok(())
    }
    fn end(self) -> Result<Value, Error> {
        Ok(Value::Object(self.map))
    }
}

impl ser::SerializeStruct for Fields {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, k: &'static str, v: &T) -> Result<(), Error> {
        let v = v.serialize(Lean)?;
        if !empty(&v) {
            self.map.insert(k.to_string(), v);
        }
        Ok(())
    }
    fn end(self) -> Result<Value, Error> {
        Ok(Value::Object(self.map))
    }
}

impl ser::SerializeStructVariant for Fields {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, k: &'static str, v: &T) -> Result<(), Error> {
        ser::SerializeStruct::serialize_field(self, k, v)
    }
    fn end(self) -> Result<Value, Error> {
        Ok(named(self.variant, Value::Object(self.map)))
    }
}
