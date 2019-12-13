//! corepack is a no_std support for messagepack in serde.
//
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "alloc", feature(alloc))]
#![allow(overflowing_literals)]

// testing requires std to be available
#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
#[cfg(all(not(feature = "std"), not(test)))]
extern crate core as std;
extern crate serde;
extern crate byteorder;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

pub use ser::Serializer;
pub use de::Deserializer;

pub mod error;
pub mod read;

mod defs;
mod seq_serializer;
mod map_serializer;
mod variant_deserializer;
mod ext_deserializer;
mod seq_deserializer;

mod ser;
mod de;

/// Parse V out of a stream of bytes.
pub fn from_iter<I, V>(mut iter: I) -> Result<V, error::Error>
    where I: Iterator<Item = u8>,
          V: serde::de::DeserializeOwned
{
    let mut de = Deserializer::new(read::CopyRead::new(|buf: &mut [u8]| {
        for i in 0..buf.len() {
            if let Some(byte) = iter.next() {
                buf[i] = byte;
            } else {
                return Err(error::Error::EndOfStream);
            }
        }

        Ok(())
    }));

    V::deserialize(&mut de)
}

/// Parse V out of a slice of bytes.
pub fn from_bytes<'a, V>(bytes: &'a [u8]) -> Result<V, error::Error>
    where V: serde::Deserialize<'a>
{
    let mut position: usize = 0;

    let mut de = Deserializer::new(read::BorrowRead::new(|len: usize| if position + len >
                                                                         bytes.len() {
        Err(error::Error::EndOfStream)
    } else {
        let result = &bytes[position..position + len];

        position += len;

        Ok(result)
    }));

    V::deserialize(&mut de)
}

/// Serialize V into a byte buffer.
pub fn to_bytes<V>(value: V) -> Result<Vec<u8>, error::Error>
    where V: serde::Serialize
{
    let mut bytes = vec![];

    {
        let mut ser = Serializer::new(|buf| {
            bytes.extend_from_slice(buf);
            Ok(())
        });

        try!(value.serialize(&mut ser));
    }

    Ok(bytes)
}

#[cfg(test)]
mod test {
    use serde::Serialize;
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;
    use std::ffi::CString;

    #[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
    enum T {
        A(usize),
        B,
        C(i8, i8),
        D { a: isize, b: String },
    }

    fn test_through<T>(item: T, expected: &[u8])
        where T: Serialize + DeserializeOwned + PartialEq + Debug
    {
        let actual = ::to_bytes(&item).expect("Failed to serialize");

        assert_eq!(expected, &*actual);

        let deserialized_item = ::from_bytes(&actual).expect("Failed to deserialize");

        assert_eq!(item, deserialized_item);
    }

    #[test]
    fn test_str() {
        test_through(format!("Hello World!"),
                     &[0xac, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x57, 0x6f, 0x72, 0x6c, 0x64,
                       0x21]);
    }

    #[test]
    fn test_enum() {
        test_through(T::B, &[0x92, 0x01, 0xc0])
    }

    #[test]
    fn test_enum_newtype() {
        test_through(T::A(42), &[0x92, 0x00, 0x2a])
    }

    #[test]
    fn test_enum_tuple() {
        test_through(T::C(-3, 22), &[0x92, 0x02, 0x92, 0xfd, 0x16])
    }

    #[test]
    fn test_enum_struct() {
        test_through(T::D {
                         a: 9001,
                         b: "Hello world!".into(),
                     },
                     &[0x92, // array with two elements
                       0x03, // 3 (variant index)
                       0x82, // map with two entries
                       0xa1, // entry one, fixstr length one: 'a'
                       0x61,
                       0xd1, // i16: 9001
                       0x23,
                       0x29,
                       0xa1, // entry two, fixstr length one: 'b'
                       0x62,
                       0xac, // fixstr, length 12: Hello world!
                       0x48,
                       0x65,
                       0x6c,
                       0x6c,
                       0x6f,
                       0x20,
                       0x77,
                       0x6f,
                       0x72,
                       0x6c,
                       0x64,
                       0x21])
    }

    #[test]
    fn test_option() {
        test_through(Some(7), &[0x92, 0xc3, 0x07])
    }

    #[test]
    fn test_option_none() {
        test_through::<Option<usize>>(None, &[0x91, 0xc2])
    }

    #[test]
    fn test_unit_option() {
        test_through(Some(()), &[0x92, 0xc3, 0xc0])
    }

    #[test]
    fn test_char() {
        test_through('b', &[0xa1, 0x62])
    }

    #[test]
    fn test_false() {
        test_through(false, &[0xc2])
    }

    #[test]
    fn test_byte_array() {
        test_through(CString::new("hello").unwrap(),
                     &[0xc4, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f])
    }

    #[test]
    fn test_float() {
        test_through(4.5, &[0xcb, 0x40, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
    }

    #[test]
    fn test_float32() {
        test_through(3.2f32, &[0xca, 0x40, 0x4c, 0xcc, 0xcd])
    }
}
