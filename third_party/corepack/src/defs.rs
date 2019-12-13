//! Common definitions that are useful in many places, as well as constants.
//
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.

use byteorder::{LittleEndian, ByteOrder};

// fixint limits
pub const FIXINT_MAX: u8 = 0b01111111;
pub const FIXINT_MIN: i8 = 0b11100000;

// map size limits
pub const MAX_FIXMAP: usize = 0xf;
pub const MAX_MAP16: usize = 0xffff;
pub const MAX_MAP32: usize = 0xffff_ffff;

// str size limits
pub const MAX_FIXSTR: usize = 0x1f;
pub const MAX_STR8: usize = 0xff;
pub const MAX_STR16: usize = 0xffff;
pub const MAX_STR32: usize = 0xffff_ffff;

// array size limits
pub const MAX_FIXARRAY: usize = 0xf;
pub const MAX_ARRAY16: usize = 0xffff;
pub const MAX_ARRAY32: usize = 0xffff_ffff;

// byte array limits
pub const MAX_BIN8: usize = 0xff;
pub const MAX_BIN16: usize = 0xffff;
pub const MAX_BIN32: usize = 0xffff_ffff;

pub struct InclusiveRange<T> {
    pub start: T,
    pub end: T,
}

impl<T> InclusiveRange<T>
    where T: PartialOrd
{
    pub fn contains(&self, idx: T) -> bool {
        (self.start <= idx) && (idx <= self.end)
    }
}

// byte defs
pub const POS_FIXINT: InclusiveRange<u8> = InclusiveRange {
    start: 0x00,
    end: 0x7f,
};
pub const FIXMAP: InclusiveRange<u8> = InclusiveRange {
    start: 0x80,
    end: 0x8f,
};
pub const FIXARRAY: InclusiveRange<u8> = InclusiveRange {
    start: 0x90,
    end: 0x9f,
};
pub const FIXSTR: InclusiveRange<u8> = InclusiveRange {
    start: 0xa0,
    end: 0xbf,
};

pub const NIL: u8 = 0xc0;
// RESERVED: 0xc1
pub const FALSE: u8 = 0xc2;
pub const TRUE: u8 = 0xc3;
pub const BIN8: u8 = 0xc4;
pub const BIN16: u8 = 0xc5;
pub const BIN32: u8 = 0xc6;
pub const EXT8: u8 = 0xc7;
pub const EXT16: u8 = 0xc8;
pub const EXT32: u8 = 0xc9;
pub const FLOAT32: u8 = 0xca;
pub const FLOAT64: u8 = 0xcb;
pub const UINT8: u8 = 0xcc;
pub const UINT16: u8 = 0xcd;
pub const UINT32: u8 = 0xce;
pub const UINT64: u8 = 0xcf;
pub const INT8: u8 = 0xd0;
pub const INT16: u8 = 0xd1;
pub const INT32: u8 = 0xd2;
pub const INT64: u8 = 0xd3;
pub const FIXEXT1: u8 = 0xd4;
pub const FIXEXT2: u8 = 0xd5;
pub const FIXEXT4: u8 = 0xd6;
pub const FIXEXT8: u8 = 0xd7;
pub const FIXEXT16: u8 = 0xd8;
pub const STR8: u8 = 0xd9;
pub const STR16: u8 = 0xda;
pub const STR32: u8 = 0xdb;
pub const ARRAY16: u8 = 0xdc;
pub const ARRAY32: u8 = 0xdd;
pub const MAP16: u8 = 0xde;
pub const MAP32: u8 = 0xdf;

pub const NEG_FIXINT: InclusiveRange<u8> = InclusiveRange {
    start: 0xe0,
    end: 0xff,
};

// bit masks
pub const FIXMAP_MASK: u8 = 0b1000_0000;
pub const FIXARRAY_MASK: u8 = 0b1001_0000;
pub const FIXSTR_MASK: u8 = 0b1010_0000;

// type sizes
pub const U64_BYTES: usize = 8;
pub const U32_BYTES: usize = 4;
pub const U16_BYTES: usize = 2;

pub fn read_signed(unsigned: u8) -> i8 {
    LittleEndian::read_i16(&[unsigned, 0]) as i8
}
