//! The read trait used by the deserializer.
//
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.
use std::ops::Deref;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use error::Error;

/// The trait used by Deserializer to read input data
pub trait Read<'de>: private::Sealed {
    /// Reads the next len bytes of data, either by borowing or copying
    fn input<'a>(&mut self,
                 len: usize,
                 scratch: &'a mut Vec<u8>)
                 -> Result<Reference<'de, 'a>, Error>;
}

/// Data that was copied or borrowed
pub enum Reference<'de, 'a> {
    Borrowed(&'de [u8]),
    Copied(&'a [u8]),
}

/// Wrapper object around a closure that provides borrowed data
pub struct BorrowRead<'de, F: FnMut(usize) -> Result<&'de [u8], Error>> {
    thunk: F,
}

/// Wrapper object around a closure that provides copied data
pub struct CopyRead<F: FnMut(&mut [u8]) -> Result<(), Error>> {
    thunk: F,
}

impl<'de, 'a> Deref for Reference<'de, 'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match *self {
            Reference::Borrowed(data) => data,
            Reference::Copied(data) => data,
        }
    }
}

impl<'de, F: FnMut(usize) -> Result<&'de [u8], Error>> BorrowRead<'de, F> {
    pub fn new(thunk: F) -> BorrowRead<'de, F> {
        BorrowRead { thunk: thunk }
    }
}

impl<F: FnMut(&mut [u8]) -> Result<(), Error>> CopyRead<F> {
    pub fn new(thunk: F) -> CopyRead<F> {
        CopyRead { thunk: thunk }
    }
}

impl<'de, F: FnMut(usize) -> Result<&'de [u8], Error>> private::Sealed for BorrowRead<'de, F> {}

impl<F: FnMut(&mut [u8]) -> Result<(), Error>> private::Sealed for CopyRead<F> {}

impl<'de, F: FnMut(usize) -> Result<&'de [u8], Error>> Read<'de> for BorrowRead<'de, F> {
    fn input<'a>(&mut self, len: usize, _: &'a mut Vec<u8>) -> Result<Reference<'de, 'a>, Error> {
        Ok(Reference::Borrowed((self.thunk)(len)?))
    }
}

impl<'de, F: FnMut(&mut [u8]) -> Result<(), Error>> Read<'de> for CopyRead<F> {
    fn input<'a>(&mut self,
                 len: usize,
                 scratch: &'a mut Vec<u8>)
                 -> Result<Reference<'de, 'a>, Error> {
        scratch.resize(len, 0);
        (self.thunk)(scratch)?;
        Ok(Reference::Copied(scratch))
    }
}

mod private {
    /// Keeps users from directly implementing the Read trait
    pub trait Sealed {}
}
