// Copyright lowRISC contributors.
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

// This file was forked from https://github.com/lowRISC/manticore.

//! I/O interfaces, in lieu of [`std::io`].
//!
//! These functions and traits are mostly intended for manipulating byte
//! buffers, but they could be implemented on other types that provide a
//! read/write interface.
//!
//! [`std::io`]: https://doc.rust-lang.org/std/io/index.html

use core::mem;

use static_assertions::assert_obj_safe;

use ux;

/// Represents a byte as a queue of bits, for simplifying parsing bit fields
/// out of a byte.
///
/// The semantics of this buffer are roughly:
/// - Bits are written at the least significant end.
/// - Bits are read from the most significant end (a length is used to track
///   where this is).
///
/// This queue behavior means that the dual operation to a sequence of writes
/// is a sequence of reads in the same order.
pub struct BitBuf {
    // NOTE: len represents the number of *least significant* bits that
    // are part of the buffer.
    len: u8,
    bits: u8,
}

/// Returns the "inverse popcnt", the smallest byte with `n` bits set.
///
/// In other words, this function computes `2^n - 1`, accounting for overflow.
#[inline(always)]
fn inverse_popcnt(n: usize) -> u8 {
    // NOTE: if the `1` below is accientally typed at `u8`, for `n = 8` we will
    // get overflow from the shift; instead, we perform the shift using native
    // arithmetic.
    ((1usize << n) - 1) as _
}

impl BitBuf {
    /// Creates an empty `BitBuf`.
    pub fn new() -> Self {
        Self { len: 0, bits: 0 }
    }

    /// Creates a new eight-bit `BitBuf` with the given bits.
    pub fn from_bits(bits: u8) -> Self {
        Self { len: 8, bits }
    }

    /// Returns the number of bits currently in the buffer.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns the bits currently in the buffer; all bits beyond `len` are
    /// guaranteed to be zero.
    pub fn bits(&self) -> u8 {
        self.bits
    }

    /// Reads the `n` most significant bits from `self`, returning them as the
    /// least significant bits of a byte.
    #[inline]
    pub fn read_bits(&mut self, n: usize) -> Result<u8, Error> {
        if self.len() < n {
            return Err(Error::BufferExhausted);
        }

        // Avoid the corner-case of `n = 0` entirely, since it can trigger
        // shift underflow.
        if n == 0 {
            return Ok(0);
        }

        let mask = inverse_popcnt(n);
        let offset = self.len() - n;
        let val = (self.bits >> offset) & mask;
        self.bits &= !(mask << offset);
        self.len -= n as u8;
        Ok(val)
    }

    /// Reads a single bit, and converts it to `bool`.
    #[inline]
    pub fn read_bit(&mut self) -> Result<bool, Error> {
        self.read_bits(1).map(|b| b != 0)
    }

    /// Writes exactly `n` bits to `self`, taken as the least significant bits
    /// of `bits`.
    #[inline]
    pub fn write_bits(&mut self, n: usize, bits: u8) -> Result<(), Error> {
        if self.len() + n > 8 {
            return Err(Error::BufferExhausted);
        }

        let mask = inverse_popcnt(n);
        self.bits = self.bits.wrapping_shl(n as u32);
        self.bits |= bits & mask;
        self.len += n as u8;
        Ok(())
    }

    /// Writes a single bit, represented as a `bool`.
    #[inline]
    pub fn write_bit(&mut self, bit: bool) -> Result<(), Error> {
        self.write_bits(1, bit as u8)
    }

    /// Writes `n` zero bits.
    #[inline]
    pub fn write_zero_bits(&mut self, n: usize) -> Result<(), Error> {
        self.write_bits(n, 0)
    }
}

/// A generic, low-level I/O error.
#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Indicates that some underlying buffer has been completely used up,
    /// either for reading from or writing to.
    ///
    /// This is typically a fatal error, since it is probably not possible
    /// to re-allocate that underlying buffer.
    BufferExhausted,

    /// Indicates that an unspecified, internal failure occurred.
    Internal,
}

/// A big-endian integer, which can be read and written.
///
/// This trait can be used for operating generically over big-endian integer
/// I/O.
pub trait BeInt: Sized + Copy {
    /// Reads a value of type `Self`, in big-endian order.
    fn read_from<'a, R: Read<'a>>(r: R) -> Result<Self, Error>;

    /// Writes a value of type `Self`, in big-endian order.
    fn write_to<W: Write>(self, w: W) -> Result<(), Error>;
}

impl BeInt for u8 {
    #[inline]
    fn read_from<'a, R: Read<'a>>(mut r: R) -> Result<Self, Error> {
        Ok(r.read_bytes(mem::size_of::<Self>())?[0])
    }

    #[inline]
    fn write_to<W: Write>(self, mut w: W) -> Result<(), Error> {
        w.write_bytes(&[self])
    }
}

impl BeInt for u16 {
    #[inline]
    fn read_from<'a, R: Read<'a>>(mut r: R) -> Result<Self, Error> {
        use byteorder::ByteOrder as _;

        Ok(byteorder::BE::read_u16(
            r.read_bytes(mem::size_of::<Self>())?,
        ))
    }

    #[inline]
    fn write_to<W: Write>(self, mut w: W) -> Result<(), Error> {
        use byteorder::ByteOrder as _;

        let mut bytes = [0; mem::size_of::<Self>()];
        byteorder::BE::write_u16(&mut bytes, self);
        w.write_bytes(&bytes)
    }
}

impl BeInt for ux::u24 {
    #[inline]
    fn read_from<'a, R: Read<'a>>(mut r: R) -> Result<Self, Error> {
        use byteorder::ByteOrder as _;

        Ok(ux::u24::new(byteorder::BE::read_u24(
            r.read_bytes(3)?,
        )))
    }

    #[inline]
    fn write_to<W: Write>(self, mut w: W) -> Result<(), Error> {
        use byteorder::ByteOrder as _;

        let mut bytes = [0; 3];
        byteorder::BE::write_u24(&mut bytes, u32::from(self));
        w.write_bytes(&bytes)
    }
}

impl BeInt for u32 {
    #[inline]
    fn read_from<'a, R: Read<'a>>(mut r: R) -> Result<Self, Error> {
        use byteorder::ByteOrder as _;

        Ok(byteorder::BE::read_u32(
            r.read_bytes(mem::size_of::<Self>())?,
        ))
    }

    #[inline]
    fn write_to<W: Write>(self, mut w: W) -> Result<(), Error> {
        use byteorder::ByteOrder as _;

        let mut bytes = [0; mem::size_of::<Self>()];
        byteorder::BE::write_u32(&mut bytes, self);
        w.write_bytes(&bytes)
    }
}

impl BeInt for u64 {
    #[inline]
    fn read_from<'a, R: Read<'a>>(mut r: R) -> Result<Self, Error> {
        use byteorder::ByteOrder as _;

        Ok(byteorder::BE::read_u64(
            r.read_bytes(mem::size_of::<Self>())?,
        ))
    }

    #[inline]
    fn write_to<W: Write>(self, mut w: W) -> Result<(), Error> {
        use byteorder::ByteOrder as _;

        let mut bytes = [0; mem::size_of::<Self>()];
        byteorder::BE::write_u64(&mut bytes, self);
        w.write_bytes(&bytes)
    }
}

/// Represents a place that bytes can be read from, such as a `&[u8]`.
///
/// Types which implement this trait enable *zero copy reads*, that is,
/// a read opertion does not need to allocate memory to perform a read, since
/// all of that memory has already been allocated ahead-of-time. The lifetime
/// of that memory if represented by the lifetime `'a`.
///
/// # Relation with [`std::io::Read`]
/// [`std::io::Read`] is distinct from `Read`; it copies data onto a buffer
/// provided by the caller. Such an API is unworkable in `manticore`, since
/// `manticore` cannot usually allocate.
///
/// The recommended way to use a [`std::io::Read`] with a `manticore` API is to
/// use `read_to_end(&mut buf)` and to then pass `&mut buf[..]` into
/// `manticore`.
///
/// [`std::io::Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
pub trait Read<'a> {
    /// Reads exactly `n` bytes from `self`.
    ///
    /// This function does not perform partial reads: it will either block
    /// until completion or return an error.
    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], Error>;

    /// Returns the number of bytes still available to read.
    fn remaining_data(&self) -> usize;

    /// Reads a big-endian integer.
    ///
    /// # Note
    /// Do not implement this function yourself. Callers are not required to
    /// call it in order to actually perform a read, so whether or not it is
    /// called is an implementation detail.
    #[inline]
    fn read_be<I: BeInt>(&mut self) -> Result<I, Error>
    where
        Self: Sized,
    {
        I::read_from(self)
    }
}

assert_obj_safe!(Read<'static>);

impl<'a, R: Read<'a> + ?Sized> Read<'a> for &'_ mut R {
    #[inline]
    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        R::read_bytes(*self, n)
    }

    #[inline]
    fn remaining_data(&self) -> usize {
        R::remaining_data(*self)
    }
}

impl<'a> Read<'a> for &'a [u8] {
    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if self.len() < n {
            return Err(Error::BufferExhausted);
        }

        let result = &self[..n];
        *self = &self[n..];
        Ok(result)
    }

    fn remaining_data(&self) -> usize {
        self.len()
    }
}

impl<'a> Read<'a> for &'a mut [u8] {
    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if self.len() < n {
            return Err(Error::BufferExhausted);
        }

        let (result, rest) = mem::replace(self, &mut []).split_at_mut(n);
        *self = rest;
        Ok(result)
    }

    fn remaining_data(&self) -> usize {
        self.len()
    }
}

/// Represents a place that bytes can be written to, such as a `&[u8]`.
///
/// # Relation with [`std::io::Write`]
/// [`std::io::Write`] provides approximately a superset of `Write`, with
/// more detailed errors. [`StdWrite`] provides an implementation of
/// `Write` in terms of [`std::io::Write`].
///
/// [`std::io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
/// [`write_bytes()`]: trait.Write.html#tymethod.write_bytes
pub trait Write {
    /// Attempt to write `buf` exactly to `self`.
    ///
    /// This function does not perform partial writes: it will either block
    /// until completion or return an error.
    fn write_bytes(&mut self, buf: &[u8]) -> Result<(), Error>;

    /// Writes a big-endian integer.
    ///
    /// # Note
    /// Do not implement this function yourself. Callers are not required to
    /// call it in order to actually perform a write, so whether or not it is
    /// called is an implementation detail.
    #[inline]
    fn write_be<I: BeInt>(&mut self, val: I) -> Result<(), Error>
    where
        Self: Sized,
    {
        val.write_to(self)
    }
}

assert_obj_safe!(Write);

impl<W: Write + ?Sized> Write for &'_ mut W {
    #[inline]
    fn write_bytes(&mut self, buf: &[u8]) -> Result<(), Error> {
        W::write_bytes(*self, buf)
    }
}

impl Write for &'_ mut [u8] {
    fn write_bytes(&mut self, buf: &[u8]) -> Result<(), Error> {
        let n = buf.len();
        if self.len() < n {
            return Err(Error::BufferExhausted);
        }

        let (dest, rest) = mem::replace(self, &mut []).split_at_mut(n);
        dest.copy_from_slice(buf);
        *self = rest;
        Ok(())
    }
}

/// Converts a [`std::io::Write`] into a [`manticore::io::Write`].
///
/// [`write_bytes()`] is implemented by simply calling [`write()`] repeatedly
/// until every byte is written; [`manticore::io::Write`] should be implemented
/// directly if possible.
///
/// This type is provided instead of implementing [`manticore::io::Write`]
/// directly for every [`std::io::Write`] due to trait coherence issues
/// involving the blanket impl on `&mut _`.
///
/// [`std::io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
/// [`manticore::io::Write`]: trait.Write.html
/// [`write_bytes()`]: trait.Write.html#tymethod.write_bytes
/// [`write()`]: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write
#[cfg(feature = "std")]
pub struct StdWrite<W>(pub W);

#[cfg(feature = "std")]
impl<W: std::io::Write> Write for StdWrite<W> {
    fn write_bytes(&mut self, mut buf: &[u8]) -> Result<(), Error> {
        use std::io::ErrorKind;
        loop {
            if buf.is_empty() {
                return Ok(());
            }
            match self.0.write(buf).map_err(|e| e.kind()) {
                Ok(len) => buf = &buf[len..],
                Err(ErrorKind::Interrupted) => continue,
                // No good way to propagate this. =/
                Err(_) => return Err(Error::Internal),
            }
        }
    }
}

/// A "cursor" over a mutable byte buffer.
///
/// This type provides a `consume()` function, which can be called repeatedly
/// to take portions of the buffer. An internal cursor will track the location
/// of the buffer. This method is used to implement [`Write`] for `Cursor`.
///
/// This type is useful when you want to feed a scratch buffer into a function
/// that performs I/O operations on a buffer, and then extract how much of the
/// buffer was read or written. This is especialy useful when used in
/// conjunction with [`ToWire`]:
/// ```
/// # use spiutils::io::*;
/// # use spiutils::protocol::wire::*;
/// # struct MyMessage;
/// # impl ToWire for MyMessage {
/// #     fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
/// #         w.write_bytes(&[1, 2, 3, 4]);
/// #         Ok(())
/// #     }
/// # }
/// let msg = MyMessage;
/// let mut buf = [0; 256];
///
/// let mut cursor = Cursor::new(&mut buf);
/// msg.to_wire(&mut cursor);
///
/// let msg_bytes = cursor.take_consumed_bytes();
/// assert_ne!(msg_bytes.len(), 0);
/// ```
///
/// [`Write`]: trait.Write.html
/// [`ToWire`]: ../protocol/wire/trait.ToWire.html
pub struct Cursor<'a> {
    buf: &'a mut [u8],
    // Invariant: cursor <= buf.len().
    cursor: usize,
}

impl<'a> Cursor<'a> {
    /// Creates a new `Cursor` for the given buffer.
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, cursor: 0 }
    }

    /// Consumes `n` bytes from the underlying buffer.
    ///
    /// If `n` bytes are unavailable, `None` is returned.
    pub fn consume(&mut self, n: usize) -> Option<&mut [u8]> {
        let end = self.cursor.checked_add(n)?;
        if self.buf.len() < end {
            return None;
        }
        let output = &mut self.buf[self.cursor..end];
        self.cursor = end;

        Some(output)
    }

    /// Returns the number of bytes consumed thus far.
    pub fn consumed_len(&self) -> usize {
        self.cursor
    }

    /// Returns the portion of the buffer which has been consumed thus far.
    pub fn consumed_bytes(&self) -> &[u8] {
        &self.buf[..self.cursor]
    }

    /// Takes the portion of the buffer which has been consumed so far,
    /// resetting the cursor value back to zero.
    ///
    /// This function leaves `self` as if it had been newly initialized with
    /// the unconsumed portion of the buffer. Repeatedly calling this function
    /// with no other intervening operations will return `&mut []`.
    ///
    /// Because this function returns a `'a` reference, it is not bound to the
    /// `Cursor` that originally contained it. This function is useful when
    /// a desired reference needs to have the lifetime of the buffer that went
    /// into the `Cursor`, rather than the `Cursor`'s own local lifetime.
    pub fn take_consumed_bytes(&mut self) -> &'a mut [u8] {
        let (output, rest) =
            mem::replace(&mut self.buf, &mut []).split_at_mut(self.cursor);
        self.cursor = 0;
        self.buf = rest;
        output
    }
}

impl Write for Cursor<'_> {
    fn write_bytes(&mut self, buf: &[u8]) -> Result<(), Error> {
        let dest = self.consume(buf.len()).ok_or(Error::BufferExhausted)?;
        dest.copy_from_slice(buf);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bit_buf_queue() {
        let mut buf = BitBuf::new();
        assert_eq!(buf.bits(), 0);
        assert_eq!(buf.len(), 0);
        buf.write_bits(3, 0b101).unwrap();
        assert_eq!(buf.bits(), 0b101);
        assert_eq!(buf.len(), 3);
        buf.write_bits(2, 0b10).unwrap();
        assert_eq!(buf.bits(), 0b10110);
        assert_eq!(buf.len(), 5);
        buf.write_bit(true).unwrap();
        assert_eq!(buf.bits(), 0b101101);
        assert_eq!(buf.len(), 6);
        buf.write_zero_bits(2).unwrap();
        assert_eq!(buf.bits(), 0b10110100);
        assert_eq!(buf.len(), 8);
        assert!(buf.write_bit(true).is_err());

        assert_eq!(buf.read_bits(3).unwrap(), 0b101);
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.read_bit().unwrap(), true);
        assert_eq!(buf.len(), 4);
        assert_eq!(buf.read_bits(4).unwrap(), 0b0100);
        assert_eq!(buf.len(), 0);
        assert!(buf.read_bit().is_err());
    }

    #[test]
    fn bit_bif_edge_cases() {
        let mut buf = BitBuf::from_bits(0x55);
        assert_eq!(buf.read_bits(0).unwrap(), 0);
        assert_eq!(buf.len(), 8);
        assert_eq!(buf.read_bits(8).unwrap(), 0x55);
        assert_eq!(buf.len(), 0);

        let mut buf = BitBuf::new();
        buf.write_bits(8, 0xaa).unwrap();
        assert_eq!(buf.bits(), 0xaa);
        assert_eq!(buf.len(), 8);
        buf.write_bits(0, 0x42).unwrap();
        assert_eq!(buf.bits(), 0xaa);
        assert_eq!(buf.len(), 8);
    }

    #[test]
    fn read_bytes() {
        let mut bytes: &[u8] = b"Hello!";
        assert_eq!(bytes.read_bytes(3).unwrap(), b"Hel");
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes.read_le::<u16>().unwrap(), 0x6f6c);
        assert_eq!(bytes.len(), 1);
        assert!(bytes.read_le::<u32>().is_err());
    }

    #[test]
    fn read_and_write_bytes() {
        let mut buf = [0; 6];
        let mut bytes = &mut buf[..];
        bytes.write_bytes(b"Wo").unwrap();
        bytes.write_bytes(b"r").unwrap();
        assert_eq!(bytes.len(), 3);
        bytes.write_le::<u16>(0x646c).unwrap();
        assert_eq!(bytes.len(), 1);
        assert!(bytes.write_bytes(b"!!").is_err());
        bytes.write_le::<u8>(b'!').unwrap();
        assert_eq!(bytes.len(), 0);
        assert_eq!(&buf, b"World!");

        let mut bytes = &mut buf[..];
        assert_eq!(bytes.read_le::<u32>().unwrap(), 0x6c726f57);
    }

    #[test]
    fn std_write() {
        let mut buf = [0; 4];
        let mut std_write = StdWrite(&mut buf[..]);
        std_write.write_le::<u32>(0x04030201).unwrap();
        assert_eq!(buf, [1, 2, 3, 4]);
    }

    #[test]
    fn cursor() {
        let mut buf = [0; 8];
        let mut cursor = Cursor::new(&mut buf);

        cursor.write_le::<u32>(0xffaaffaa).unwrap();
        assert_eq!(cursor.consumed_len(), 4);
        assert_eq!(cursor.consumed_bytes(), &[0xaa, 0xff, 0xaa, 0xff]);
        let bytes = cursor.take_consumed_bytes();
        assert_eq!(bytes, &[0xaa, 0xff, 0xaa, 0xff]);
        assert_eq!(cursor.consumed_len(), 0);

        assert!(cursor.write_bytes(&[0x55; 7]).is_err());
    }
}
