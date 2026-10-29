//! Every record goes to disk through `Narrow`, which writes each `f64` as an
//! `f32`: seven significant digits (a $25 million contract to within $2, a
//! rating to a millionth), half the bytes, and it compresses better. Every
//! other value passes through unchanged.

use serde::ser::{self, Serialize, Serializer};

/// A value written with its `f64`s as `f32`s.
pub(crate) struct Narrow<'a, T: ?Sized>(pub &'a T);

impl<T: Serialize + ?Sized> Serialize for Narrow<'_, T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(Ser(s))
    }
}

struct Ser<S>(S);

/// A compound value whose parts are narrowed too.
pub(crate) struct Wrap<C>(C);

impl<S: Serializer> Serializer for Ser<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = Wrap<S::SerializeSeq>;
    type SerializeTuple = Wrap<S::SerializeTuple>;
    type SerializeTupleStruct = Wrap<S::SerializeTupleStruct>;
    type SerializeTupleVariant = Wrap<S::SerializeTupleVariant>;
    type SerializeMap = Wrap<S::SerializeMap>;
    type SerializeStruct = Wrap<S::SerializeStruct>;
    type SerializeStructVariant = Wrap<S::SerializeStructVariant>;

    fn serialize_bool(self, v: bool) -> Result<S::Ok, S::Error> {
        self.0.serialize_bool(v)
    }
    fn serialize_i8(self, v: i8) -> Result<S::Ok, S::Error> {
        self.0.serialize_i8(v)
    }
    fn serialize_i16(self, v: i16) -> Result<S::Ok, S::Error> {
        self.0.serialize_i16(v)
    }
    fn serialize_i32(self, v: i32) -> Result<S::Ok, S::Error> {
        self.0.serialize_i32(v)
    }
    fn serialize_i64(self, v: i64) -> Result<S::Ok, S::Error> {
        self.0.serialize_i64(v)
    }
    fn serialize_i128(self, v: i128) -> Result<S::Ok, S::Error> {
        self.0.serialize_i128(v)
    }
    fn serialize_u8(self, v: u8) -> Result<S::Ok, S::Error> {
        self.0.serialize_u8(v)
    }
    fn serialize_u16(self, v: u16) -> Result<S::Ok, S::Error> {
        self.0.serialize_u16(v)
    }
    fn serialize_u32(self, v: u32) -> Result<S::Ok, S::Error> {
        self.0.serialize_u32(v)
    }
    fn serialize_u64(self, v: u64) -> Result<S::Ok, S::Error> {
        self.0.serialize_u64(v)
    }
    fn serialize_u128(self, v: u128) -> Result<S::Ok, S::Error> {
        self.0.serialize_u128(v)
    }
    fn serialize_f32(self, v: f32) -> Result<S::Ok, S::Error> {
        self.0.serialize_f32(v)
    }
    fn serialize_f64(self, v: f64) -> Result<S::Ok, S::Error> {
        self.0.serialize_f32(v as f32)
    }
    fn serialize_char(self, v: char) -> Result<S::Ok, S::Error> {
        self.0.serialize_char(v)
    }
    fn serialize_str(self, v: &str) -> Result<S::Ok, S::Error> {
        self.0.serialize_str(v)
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<S::Ok, S::Error> {
        self.0.serialize_bytes(v)
    }
    fn serialize_none(self) -> Result<S::Ok, S::Error> {
        self.0.serialize_none()
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<S::Ok, S::Error> {
        self.0.serialize_some(&Narrow(v))
    }
    fn serialize_unit(self) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit()
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit_struct(name)
    }
    fn serialize_unit_variant(self, name: &'static str, i: u32, variant: &'static str) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit_variant(name, i, variant)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, name: &'static str, v: &T) -> Result<S::Ok, S::Error> {
        self.0.serialize_newtype_struct(name, &Narrow(v))
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        i: u32,
        variant: &'static str,
        v: &T,
    ) -> Result<S::Ok, S::Error> {
        self.0.serialize_newtype_variant(name, i, variant, &Narrow(v))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, S::Error> {
        self.0.serialize_seq(len).map(Wrap)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, S::Error> {
        self.0.serialize_tuple(len).map(Wrap)
    }
    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, S::Error> {
        self.0.serialize_tuple_struct(name, len).map(Wrap)
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        i: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, S::Error> {
        self.0.serialize_tuple_variant(name, i, variant, len).map(Wrap)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, S::Error> {
        self.0.serialize_map(len).map(Wrap)
    }
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, S::Error> {
        self.0.serialize_struct(name, len).map(Wrap)
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        i: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, S::Error> {
        self.0.serialize_struct_variant(name, i, variant, len).map(Wrap)
    }
    fn is_human_readable(&self) -> bool {
        self.0.is_human_readable()
    }
}

impl<C: ser::SerializeSeq> ser::SerializeSeq for Wrap<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), C::Error> {
        self.0.serialize_element(&Narrow(v))
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: ser::SerializeTuple> ser::SerializeTuple for Wrap<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), C::Error> {
        self.0.serialize_element(&Narrow(v))
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: ser::SerializeTupleStruct> ser::SerializeTupleStruct for Wrap<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), C::Error> {
        self.0.serialize_field(&Narrow(v))
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: ser::SerializeTupleVariant> ser::SerializeTupleVariant for Wrap<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), C::Error> {
        self.0.serialize_field(&Narrow(v))
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: ser::SerializeMap> ser::SerializeMap for Wrap<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, k: &T) -> Result<(), C::Error> {
        self.0.serialize_key(&Narrow(k))
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), C::Error> {
        self.0.serialize_value(&Narrow(v))
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: ser::SerializeStruct> ser::SerializeStruct for Wrap<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, v: &T) -> Result<(), C::Error> {
        self.0.serialize_field(key, &Narrow(v))
    }
    fn skip_field(&mut self, key: &'static str) -> Result<(), C::Error> {
        self.0.skip_field(key)
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: ser::SerializeStructVariant> ser::SerializeStructVariant for Wrap<C> {
    type Ok = C::Ok;
    type Error = C::Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, v: &T) -> Result<(), C::Error> {
        self.0.serialize_field(key, &Narrow(v))
    }
    fn skip_field(&mut self, key: &'static str) -> Result<(), C::Error> {
        self.0.skip_field(key)
    }
    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}
