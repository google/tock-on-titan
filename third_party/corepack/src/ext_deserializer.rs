//! A visitor for EXT items in a messagepack stream.
//
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.
#[cfg(feature = "alloc")]
use alloc::borrow::ToOwned;

use serde::de::{MapAccess, DeserializeSeed, IntoDeserializer};
use serde::de::value::{StrDeserializer, I8Deserializer, SeqDeserializer};

use error::Error;

pub struct ExtDeserializer<'a> {
    state: u8,
    ty: i8,
    data: &'a [u8],
}

impl<'a> ExtDeserializer<'a> {
    pub fn new(ty: i8, data: &'a [u8]) -> ExtDeserializer<'a> {
        ExtDeserializer {
            state: 0,
            ty: ty,
            data: data,
        }
    }
}

impl<'de, 'a> MapAccess<'de> for ExtDeserializer<'a> {
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
        where T: DeserializeSeed<'de>
    {
        if self.state == 0 {
            let de: StrDeserializer<Self::Error> = "type".into_deserializer();
            Ok(Some(try!(seed.deserialize(de))))
        } else if self.state == 1 {
            let de: StrDeserializer<Self::Error> = "data".into_deserializer();
            Ok(Some(try!(seed.deserialize(de))))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Self::Error>
        where T: DeserializeSeed<'de>
    {
        if self.state == 0 {
            self.state += 1;
            let de: I8Deserializer<Self::Error> = self.ty.into_deserializer();
            Ok(try!(seed.deserialize(de)))
        } else if self.state == 1 {
            self.state += 1;
            let de: SeqDeserializer<_, Self::Error> = self.data.to_owned().into_deserializer();
            Ok(try!(seed.deserialize(de)))
        } else {
            Err(Error::EndOfStream)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(2 - self.state as usize)
    }
}
