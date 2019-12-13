//! The visitor that decodes sequences.
//
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.
use serde::de::{SeqAccess, MapAccess, DeserializeSeed};

use de::Deserializer;

use error::Error;
use read::Read;

pub struct SeqDeserializer<'de: 'a, 'a, R: 'a + Read<'de>> {
    de: &'a mut Deserializer<'de, R>,
    count: usize,
}

impl<'de, 'a, R: Read<'de>> SeqDeserializer<'de, 'a, R> {
    pub fn new(de: &'a mut Deserializer<'de, R>, count: usize) -> SeqDeserializer<'de, 'a, R> {
        SeqDeserializer {
            de: de,
            count: count,
        }
    }

    fn visit_item<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
        where T: DeserializeSeed<'de>
    {
        if self.count == 0 {
            return Ok(None);
        }

        self.count -= 1;

        Ok(Some(try!(seed.deserialize(&mut *self.de))))
    }
}

impl<'de, 'a, R: Read<'de>> SeqAccess<'de> for SeqDeserializer<'de, 'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
        where T: DeserializeSeed<'de>
    {
        self.visit_item(seed)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.count)
    }
}

impl<'de, 'a, R: Read<'de>> MapAccess<'de> for SeqDeserializer<'de, 'a, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
        where K: DeserializeSeed<'de>
    {
        self.visit_item(seed)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
        where V: DeserializeSeed<'de>
    {
        self.visit_item(seed)
            .and_then(|maybe_value| maybe_value.ok_or(Error::EndOfStream))
    }

    fn size_hint(&self) -> Option<usize> {
        Some((self.count + 1) / 2)
    }
}
