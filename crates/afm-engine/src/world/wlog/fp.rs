//! Fingerprints of stores: a 64-bit FNV-1a hash of a value's serde form,
//! with each map's entries combined in any order, so a `HashMap`'s walk
//! order (different per copy and per run) never changes the print.

use serde::ser::{self, Serialize, SerializeMap, Serializer};

/// The fingerprint of a value.
pub(crate) fn of<T: Serialize + ?Sized>(v: &T) -> u64 {
    let mut f = Fp(OFFSET);
    let _ = v.serialize(&mut f);
    f.0
}

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

pub(crate) struct Fp(u64);

impl Fp {
    fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 ^= x as u64;
            self.0 = self.0.wrapping_mul(PRIME);
        }
    }
    fn tag(&mut self, t: u8) {
        self.bytes(&[t]);
    }
}

#[derive(Debug)]
pub(crate) struct Never;

impl std::fmt::Display for Never {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("fingerprint")
    }
}

impl std::error::Error for Never {}

impl ser::Error for Never {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        Never
    }
}

type R = Result<(), Never>;

impl<'a> Serializer for &'a mut Fp {
    type Ok = ();
    type Error = Never;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Map<'a>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> R {
        self.tag(1);
        self.bytes(&[v as u8]);
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> R {
        self.serialize_i64(v as i64)
    }
    fn serialize_i16(self, v: i16) -> R {
        self.serialize_i64(v as i64)
    }
    fn serialize_i32(self, v: i32) -> R {
        self.serialize_i64(v as i64)
    }
    fn serialize_i64(self, v: i64) -> R {
        self.tag(2);
        self.bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> R {
        self.serialize_u64(v as u64)
    }
    fn serialize_u16(self, v: u16) -> R {
        self.serialize_u64(v as u64)
    }
    fn serialize_u32(self, v: u32) -> R {
        self.serialize_u64(v as u64)
    }
    fn serialize_u64(self, v: u64) -> R {
        self.tag(3);
        self.bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> R {
        self.tag(4);
        self.bytes(&v.to_bits().to_le_bytes());
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> R {
        self.tag(5);
        self.bytes(&v.to_bits().to_le_bytes());
        Ok(())
    }
    fn serialize_char(self, v: char) -> R {
        self.serialize_u64(v as u64)
    }
    fn serialize_str(self, v: &str) -> R {
        self.serialize_bytes(v.as_bytes())
    }
    fn serialize_bytes(self, v: &[u8]) -> R {
        self.tag(6);
        self.bytes(&(v.len() as u64).to_le_bytes());
        self.bytes(v);
        Ok(())
    }
    fn serialize_none(self) -> R {
        self.tag(7);
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> R {
        self.tag(8);
        v.serialize(self)
    }
    fn serialize_unit(self) -> R {
        self.tag(9);
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> R {
        self.serialize_unit()
    }
    fn serialize_unit_variant(self, _: &'static str, i: u32, _: &'static str) -> R {
        self.tag(10);
        self.bytes(&i.to_le_bytes());
        Ok(())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _: &'static str, v: &T) -> R {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(self, _: &'static str, i: u32, _: &'static str, v: &T) -> R {
        self.tag(11);
        self.bytes(&i.to_le_bytes());
        v.serialize(self)
    }
    fn serialize_seq(self, n: Option<usize>) -> Result<Self, Never> {
        self.tag(12);
        self.bytes(&(n.unwrap_or(0) as u64).to_le_bytes());
        Ok(self)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self, Never> {
        self.tag(13);
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, Never> {
        self.tag(13);
        Ok(self)
    }
    fn serialize_tuple_variant(self, _: &'static str, i: u32, _: &'static str, _: usize) -> Result<Self, Never> {
        self.tag(14);
        self.bytes(&i.to_le_bytes());
        Ok(self)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Map<'a>, Never> {
        Ok(Map { parent: self, sum: 0, n: 0, key: OFFSET })
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Never> {
        self.tag(15);
        Ok(self)
    }
    fn serialize_struct_variant(self, _: &'static str, i: u32, _: &'static str, _: usize) -> Result<Self, Never> {
        self.tag(16);
        self.bytes(&i.to_le_bytes());
        Ok(self)
    }
}

impl ser::SerializeSeq for &mut Fp {
    type Ok = ();
    type Error = Never;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> R {
        v.serialize(&mut **self)
    }
    fn end(self) -> R {
        Ok(())
    }
}

impl ser::SerializeTuple for &mut Fp {
    type Ok = ();
    type Error = Never;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> R {
        v.serialize(&mut **self)
    }
    fn end(self) -> R {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut Fp {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> R {
        v.serialize(&mut **self)
    }
    fn end(self) -> R {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut Fp {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> R {
        v.serialize(&mut **self)
    }
    fn end(self) -> R {
        Ok(())
    }
}

impl ser::SerializeStruct for &mut Fp {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, k: &'static str, v: &T) -> R {
        self.bytes(k.as_bytes());
        v.serialize(&mut **self)
    }
    fn end(self) -> R {
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut Fp {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, k: &'static str, v: &T) -> R {
        self.bytes(k.as_bytes());
        v.serialize(&mut **self)
    }
    fn end(self) -> R {
        Ok(())
    }
}

/// A map: each entry is hashed on its own and the entries are summed, so
/// any order gives the same print.
pub(crate) struct Map<'a> {
    parent: &'a mut Fp,
    sum: u64,
    n: u64,
    key: u64,
}

impl SerializeMap for Map<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, k: &T) -> R {
        self.key = of(k);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> R {
        let mut f = Fp(self.key);
        v.serialize(&mut f)?;
        self.sum = self.sum.wrapping_add(f.0);
        self.n += 1;
        Ok(())
    }
    fn end(self) -> R {
        self.parent.tag(17);
        self.parent.bytes(&self.n.to_le_bytes());
        self.parent.bytes(&self.sum.to_le_bytes());
        Ok(())
    }
}
