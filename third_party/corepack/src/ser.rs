//! The main serializer mux.
//
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.
use std::result;

use byteorder::{ByteOrder, BigEndian, LittleEndian};

use serde::Serialize;

use serde;

use error::Error;

use defs::*;
use seq_serializer::*;
use map_serializer::*;

/// The corepack Serializer. Contains a closure that receives byte buffers as the output is created.
pub struct Serializer<F: FnMut(&[u8]) -> Result<(), Error>> {
    output: F,
}

impl<F: FnMut(&[u8]) -> Result<(), Error>> Serializer<F> {
    /// Create a new Deserializer given an input function.
    pub fn new(output: F) -> Serializer<F> {
        Serializer { output: output }
    }

    fn serialize_signed(&mut self, value: i64) -> Result<(), Error> {
        if value >= FIXINT_MIN as i64 && value <= FIXINT_MAX as i64 {
            let mut buf = [0; U16_BYTES];
            LittleEndian::write_i16(&mut buf, value as i16);
            (self.output)(&buf[..1])
        } else if value >= i8::min_value() as i64 && value <= i8::max_value() as i64 {
            let mut buf = [0; U16_BYTES];
            LittleEndian::write_i16(&mut buf, value as i16);
            (self.output)(&[INT8, buf[0]])
        } else if value >= 0 && value <= u8::max_value() as i64 {
            let mut buf = [0; U16_BYTES];
            LittleEndian::write_i16(&mut buf, value as i16);
            (self.output)(&[UINT8, buf[0]])
        } else if value >= i16::min_value() as i64 && value <= i16::max_value() as i64 {
            let mut buf = [INT16; U16_BYTES + 1];
            BigEndian::write_i16(&mut buf[1..], value as i16);
            (self.output)(&buf)
        } else if value >= 0 && value <= u16::max_value() as i64 {
            let mut buf = [UINT16; U16_BYTES + 1];
            BigEndian::write_u16(&mut buf[1..], value as u16);
            (self.output)(&buf)
        } else if value >= i32::min_value() as i64 && value <= i32::max_value() as i64 {
            let mut buf = [INT32; U32_BYTES + 1];
            BigEndian::write_i32(&mut buf[1..], value as i32);
            (self.output)(&buf)
        } else if value >= 0 && value <= u32::max_value() as i64 {
            let mut buf = [UINT32; U16_BYTES + 1];
            BigEndian::write_u32(&mut buf[1..], value as u32);
            (self.output)(&buf)
        } else {
            let mut buf = [INT64; U64_BYTES + 1];
            BigEndian::write_i64(&mut buf[1..], value);
            (self.output)(&buf)
        }
    }

    fn serialize_unsigned(&mut self, value: u64) -> Result<(), Error> {
        if value <= FIXINT_MAX as u64 {
            (self.output)(&[value as u8])
        } else if value <= u8::max_value() as u64 {
            (self.output)(&[UINT8, value as u8])
        } else if value <= u16::max_value() as u64 {
            let mut buf = [UINT16; U16_BYTES + 1];
            BigEndian::write_u16(&mut buf[1..], value as u16);
            (self.output)(&buf)
        } else if value <= u32::max_value() as u64 {
            let mut buf = [UINT32; U32_BYTES + 1];
            BigEndian::write_u32(&mut buf[1..], value as u32);
            (self.output)(&buf)
        } else {
            let mut buf = [UINT64; U64_BYTES + 1];
            BigEndian::write_u64(&mut buf[1..], value);
            (self.output)(&buf)
        }
    }

    fn serialize_bool(&mut self, value: bool) -> Result<(), Error> {
        if value {
            (self.output)(&[TRUE])
        } else {
            (self.output)(&[FALSE])
        }
    }

    fn serialize_f32(&mut self, value: f32) -> Result<(), Error> {
        let mut buf = [FLOAT32; U32_BYTES + 1];
        BigEndian::write_f32(&mut buf[1..], value);
        (self.output)(&buf)
    }

    fn serialize_f64(&mut self, value: f64) -> Result<(), Error> {
        let mut buf = [FLOAT64; U64_BYTES + 1];
        BigEndian::write_f64(&mut buf[1..], value);
        (self.output)(&buf)
    }

    fn serialize_bytes(&mut self, value: &[u8]) -> Result<(), Error> {
        if value.len() <= MAX_BIN8 {
            try!((self.output)(&[BIN8, value.len() as u8]));
        } else if value.len() <= MAX_BIN16 {
            let mut buf = [BIN16; U16_BYTES + 1];
            BigEndian::write_u16(&mut buf[1..], value.len() as u16);
            try!((self.output)(&buf));
        } else if value.len() <= MAX_BIN32 {
            let mut buf = [BIN32; U32_BYTES + 1];
            BigEndian::write_u32(&mut buf[1..], value.len() as u32);
            try!((self.output)(&buf));
        } else {
            return Err(Error::TooBig);
        }

        (self.output)(value)
    }

    fn serialize_str(&mut self, value: &str) -> Result<(), Error> {
        if value.len() <= MAX_FIXSTR {
            try!((self.output)(&[value.len() as u8 | FIXSTR_MASK]));
        } else if value.len() <= MAX_STR8 {
            try!((self.output)(&[STR8, value.len() as u8]));
        } else if value.len() <= MAX_STR16 {
            let mut buf = [STR16; U16_BYTES + 1];
            BigEndian::write_u16(&mut buf[1..], value.len() as u16);
            try!((self.output)(&buf));
        } else if value.len() <= MAX_STR32 {
            let mut buf = [STR32; U32_BYTES + 1];
            BigEndian::write_u32(&mut buf[1..], value.len() as u32);
            try!((self.output)(&buf));
        } else {
            return Err(Error::TooBig);
        }

        (self.output)(value.as_bytes())
    }

    fn serialize_unit(&mut self) -> Result<(), Error> {
        (self.output)(&[NIL])
    }

    fn serialize_variant(&mut self, variant_index: u32) -> Result<(), Error> {
        // Serialize variants as two-tuples with the variant index and its contents.
        // Because messagepack is purely right-associative, we don't have to track
        // the variant once we get it going.

        // start a two element array
        (self.output)(&[2u8 | FIXARRAY_MASK])?;

        // encode the variant and done
        self.serialize_unsigned(variant_index as u64)
    }
}

impl<'a, F: 'a + FnMut(&[u8]) -> Result<(), Error>> serde::Serializer for &'a mut Serializer<F> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a, F>;
    type SerializeTuple = Self::SerializeSeq;
    type SerializeTupleStruct = Self::SerializeTuple;
    type SerializeTupleVariant = Self::SerializeTuple;

    type SerializeMap = MapSerializer<'a, F>;
    type SerializeStruct = Self::SerializeMap;
    type SerializeStructVariant = Self::SerializeMap;

    fn serialize_seq(self, size: Option<usize>) -> result::Result<Self::SerializeSeq, Self::Error> {
        let mut seq = SeqSerializer::new(&mut self.output);

        seq.hint_size(size)?;

        Ok(seq)
    }

    fn serialize_map(self, size: Option<usize>) -> result::Result<Self::SerializeMap, Self::Error> {
        let mut map = MapSerializer::new(&mut self.output);

        map.hint_size(size)?;

        Ok(map)
    }

    fn serialize_bool(self, v: bool) -> Result<(), Error> {
        Serializer::serialize_bool(self, v)
    }

    fn serialize_i64(self, value: i64) -> Result<(), Error> {
        Serializer::serialize_signed(self, value)
    }

    fn serialize_u64(self, value: u64) -> Result<(), Error> {
        Serializer::serialize_unsigned(self, value)
    }

    fn serialize_f32(self, value: f32) -> Result<(), Error> {
        Serializer::serialize_f32(self, value)
    }

    fn serialize_f64(self, value: f64) -> Result<(), Error> {
        Serializer::serialize_f64(self, value)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<(), Error> {
        Serializer::serialize_bytes(self, value)
    }

    fn serialize_str(self, value: &str) -> Result<(), Error> {
        Serializer::serialize_str(self, value)
    }

    fn serialize_unit(self) -> Result<(), Error> {
        Serializer::serialize_unit(self)
    }

    fn serialize_i8(self, value: i8) -> Result<(), Error> {
        Serializer::serialize_signed(self, value as i64)
    }

    fn serialize_i16(self, value: i16) -> Result<(), Error> {
        Serializer::serialize_signed(self, value as i64)
    }

    fn serialize_i32(self, value: i32) -> Result<(), Error> {
        Serializer::serialize_signed(self, value as i64)
    }

    fn serialize_u8(self, value: u8) -> Result<(), Error> {
        Serializer::serialize_unsigned(self, value as u64)
    }

    fn serialize_u16(self, value: u16) -> Result<(), Error> {
        Serializer::serialize_unsigned(self, value as u64)
    }

    fn serialize_u32(self, value: u32) -> Result<(), Error> {
        Serializer::serialize_unsigned(self, value as u64)
    }

    fn serialize_char(self, v: char) -> Result<(), Error> {
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(self,
                              _: &'static str,
                              index: u32,
                              _: &'static str)
                              -> Result<(), Error> {
        self.serialize_variant(index)?;
        self.serialize_unit()
    }

    fn serialize_newtype_struct<T>(self, _: &'static str, value: &T) -> Result<(), Error>
        where T: ?Sized + serde::Serialize
    {
        // serialize newtypes directly
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(self,
                                    name: &'static str,
                                    variant_index: u32,
                                    _: &'static str,
                                    value: &T)
                                    -> Result<(), Error>
        where T: ?Sized + serde::Serialize
    {
        self.serialize_variant(variant_index)?;
        self.serialize_newtype_struct(name, value)
    }

    fn serialize_none(self) -> Result<(), Error> {
        (false,).serialize(self)
    }

    fn serialize_some<V>(self, value: &V) -> Result<(), Self::Error>
        where V: ?Sized + serde::Serialize
    {
        (true, value).serialize(self)
    }

    fn serialize_tuple(self, len: usize) -> result::Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self,
                              _: &'static str,
                              len: usize)
                              -> result::Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(self,
                               name: &'static str,
                               index: u32,
                               _: &'static str,
                               len: usize)
                               -> result::Result<Self::SerializeTupleVariant, Self::Error> {
        self.serialize_variant(index)?;
        self.serialize_tuple_struct(name, len)
    }

    fn serialize_struct(self,
                        _: &'static str,
                        len: usize)
                        -> result::Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(self,
                                name: &'static str,
                                index: u32,
                                _: &'static str,
                                len: usize)
                                -> result::Result<Self::SerializeStructVariant, Self::Error> {
        self.serialize_variant(index)?;
        self.serialize_struct(name, len)
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    #[test]
    fn positive_fixint_test() {
        let v: u8 = 23;
        assert_eq!(::to_bytes(v).unwrap(), &[0x17]);
    }
    #[test]
    fn negative_fixint_test() {
        let v: i8 = -5;
        assert_eq!(::to_bytes(v).unwrap(), &[0xfb]);
    }

    #[test]
    fn uint8_test() {
        let v: u8 = 154;
        assert_eq!(::to_bytes(v).unwrap(), &[0xcc, 0x9a]);
    }

    #[test]
    fn fixstr_test() {
        let s: &str = "Hello World!";
        assert_eq!(::to_bytes(s).unwrap(),
                   &[0xac, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x57, 0x6f, 0x72, 0x6c, 0x64, 0x21]);
    }

    #[test]
    fn str8_test() {
        let s: &str = "The quick brown fox jumps over the lazy dog";
        let mut fixture: Vec<u8> = vec![];
        fixture.push(0xd9);
        fixture.push(s.len() as u8);
        fixture.extend_from_slice(s.as_bytes());
        assert_eq!(::to_bytes(s).unwrap(), fixture);
    }

    #[test]
    fn fixarr_test() {
        let v: Vec<u8> = vec![5, 8, 20, 231];
        assert_eq!(::to_bytes(v).unwrap(),
                   &[0x94, 0x05, 0x08, 0x14, 0xcc, 0xe7]);
    }

    #[test]
    fn array16_test() {
        let v: Vec<isize> = vec![-5, 16, 101, -45, 184, 89, 62, -233, -33, 304, 76, 90, 23, 108,
                                 45, -3, 2];
        assert_eq!(::to_bytes(v).unwrap(),
                   &[0xdc, 0x00, 0x11, 0xfb, 0x10, 0x65, 0xd0, 0xd3, 0xcc, 0xb8, 0x59, 0x3e,
                     0xd1, 0xff, 0x17, 0xd0, 0xdf, 0xd1, 0x01, 0x30, 0x4c, 0x5a, 0x17, 0x6c,
                     0x2d, 0xfd, 0x02]);
    }

    #[test]
    fn fixmap_test() {
        let mut map: BTreeMap<String, usize> = BTreeMap::new();
        map.insert("one".into(), 1);
        map.insert("two".into(), 2);
        map.insert("three".into(), 3);
        assert_eq!(::to_bytes(map).unwrap(),
                   &[0x83, 0xa3, 0x6f, 0x6e, 0x65, 0x01, 0xa5, 0x74, 0x68, 0x72, 0x65, 0x65,
                     0x03, 0xa3, 0x74, 0x77, 0x6f, 0x02]);
    }
}
