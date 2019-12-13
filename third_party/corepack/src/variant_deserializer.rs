//! The visitor for variants, used to deserialize enums.
//
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.
#[cfg(feature = "alloc")]
use alloc::borrow::ToOwned;

use serde::de::{IntoDeserializer, DeserializeSeed, EnumAccess, Visitor, Deserialize, VariantAccess};
use serde::de::value::StringDeserializer;

use de::Deserializer;

use error::Error;
use read::Read;

pub struct VariantDeserializer<'de: 'a, 'a, R: 'a + Read<'de>> {
    de: &'a mut Deserializer<'de, R>,
    variants: &'static [&'static str],
}

impl<'de, 'a, R: Read<'de>> VariantDeserializer<'de, 'a, R> {
    pub fn new(de: &'a mut Deserializer<'de, R>,
               variants: &'static [&'static str])
               -> VariantDeserializer<'de, 'a, R> {
        VariantDeserializer {
            de: de,
            variants: variants,
        }
    }
}

impl<'de, 'a, R: Read<'de>> EnumAccess<'de> for VariantDeserializer<'de, 'a, R> {
    type Error = Error;
    type Variant = VariantDeserializer<'de, 'a, R>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
        where V: DeserializeSeed<'de>
    {
        // get the variant index with a one-item tuple
        let variant_index_container: (usize, /* enum-type */) =
            Deserialize::deserialize(&mut *self.de)?;

        // the other value in this tuple would be the actual value of the enum,
        // but we don't know what that is
        let (variant_index /* enum-value */,) = variant_index_container;

        // translate that to the name of the variant
        let name = self.variants[variant_index].to_owned();
        let de: StringDeserializer<Error> = name.into_deserializer();
        let value = seed.deserialize(de)?;

        Ok((value, self))
    }
}

impl<'de, 'a, R: Read<'de>> VariantAccess<'de> for VariantDeserializer<'de, 'a, R> {
    type Error = Error;

    fn tuple_variant<V>(self, _: usize, visitor: V) -> Result<V::Value, Error>
        where V: Visitor<'de>
    {
        ::serde::Deserializer::deserialize_any(self.de, visitor)
    }

    fn struct_variant<V>(self, _: &'static [&'static str], visitor: V) -> Result<V::Value, Error>
        where V: Visitor<'de>
    {
        ::serde::Deserializer::deserialize_any(self.de, visitor)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
        where T: DeserializeSeed<'de>
    {
        seed.deserialize(self.de)
    }

    fn unit_variant(self) -> Result<(), Error> {
        Deserialize::deserialize(&mut *self.de)
    }
}
