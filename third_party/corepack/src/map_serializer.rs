//! The serializer that formats maps correctly for messagepack.
//
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use serde::ser::{Serialize, SerializeMap, SerializeStruct, SerializeStructVariant};

use byteorder::{ByteOrder, BigEndian};

use ser::Serializer;

use defs::*;
use error::Error;

pub struct MapSerializer<'a, F: 'a + FnMut(&[u8]) -> Result<(), Error>> {
    count: usize,
    size: Option<usize>,
    buffer: Vec<u8>,
    output: &'a mut F,
}

impl<'a, F: 'a + FnMut(&[u8]) -> Result<(), Error>> MapSerializer<'a, F> {
    pub fn new(output: &'a mut F) -> MapSerializer<'a, F> {
        MapSerializer {
            count: 0,
            size: None,
            buffer: vec![],
            output: output,
        }
    }

    pub fn hint_size(&mut self, size: Option<usize>) -> Result<(), Error> {
        self.size = size;

        if let Some(size) = self.size {
            // output this now because we know it
            self.output_map_header(size)
        } else {
            Ok(())
        }
    }

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
        where T: ?Sized + Serialize
    {
        self.count += 1;

        if self.should_serialize_directly() {
            self.serialize_directly(value)
        } else {
            self.serialize_into_buffer(value)
        }
    }

    fn finish(mut self) -> Result<(), Error> {
        if let Some(size) = self.size {
            self.check_item_count_matches_size(size * 2)?;
            Ok(())
        } else {
            let count = self.get_item_count()?;
            self.output_map_header(count)?;
            (self.output)(&*self.buffer)
        }
    }

    fn output_map_header(&mut self, size: usize) -> Result<(), Error> {
        if size <= MAX_FIXMAP {
            (self.output)(&[size as u8 | FIXMAP_MASK])
        } else if size <= MAX_MAP16 {
            let mut buf = [MAP16; U16_BYTES + 1];
            BigEndian::write_u16(&mut buf[1..], size as u16);
            (self.output)(&buf)
        } else if size <= MAX_MAP32 {
            let mut buf = [MAP32; U32_BYTES + 1];
            BigEndian::write_u32(&mut buf[1..], size as u32);
            (self.output)(&buf)
        } else {
            Err(Error::TooBig)
        }
    }

    fn get_item_count(&self) -> Result<usize, Error> {
        if self.count % 1 != 0 {
            Err(Error::BadLength)
        } else {
            Ok(self.count / 2)
        }
    }

    fn check_item_count_matches_size(&self, size: usize) -> Result<(), Error> {
        if size != self.count {
            Err(Error::BadLength)
        } else {
            Ok(())
        }
    }

    fn should_serialize_directly(&mut self) -> bool {
        self.size.is_some()
    }

    fn serialize_into_buffer<T>(&mut self, value: &T) -> Result<(), Error>
        where T: ?Sized + Serialize
    {
        let mut target = Serializer::new(|bytes| {
            self.buffer.extend_from_slice(bytes);
            Ok(())
        });

        value.serialize(&mut target)
    }

    fn serialize_directly<T>(&mut self, value: &T) -> Result<(), Error>
        where T: ?Sized + Serialize
    {
        let mut target = Serializer::new(|bytes| (self.output)(bytes));

        value.serialize(&mut target)
    }
}

impl<'a, F: 'a + FnMut(&[u8]) -> Result<(), Error>> SerializeMap for MapSerializer<'a, F> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Error>
        where T: ?Sized + Serialize
    {
        MapSerializer::serialize_element(self, key)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
        where T: ?Sized + Serialize
    {
        MapSerializer::serialize_element(self, value)
    }

    fn end(self) -> Result<(), Error> {
        MapSerializer::finish(self)
    }
}

impl<'a, F: 'a + FnMut(&[u8]) -> Result<(), Error>> SerializeStruct for MapSerializer<'a, F> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
        where T: ?Sized + Serialize
    {
        MapSerializer::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<(), Error> {
        MapSerializer::finish(self)
    }
}

impl<'a, F: 'a + FnMut(&[u8]) -> Result<(), Error>> SerializeStructVariant
    for MapSerializer<'a, F> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
        where T: ?Sized + Serialize
    {
        MapSerializer::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<(), Error> {
        MapSerializer::finish(self)
    }
}
