//! Error types for corepack.
//
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at https://mozilla.org/MPL/2.0/.
use std::fmt::Display;

#[cfg(feature = "alloc")]
use alloc::string::String;

#[cfg(feature = "alloc")]
use alloc::string::ToString;

use std::str::Utf8Error;

use std::fmt;

/// Reasons that parsing or encoding might fail in corepack.
#[derive(Debug)]
pub enum Error {
    /// Container or sequence was too big to serialize.
    TooBig,

    /// Reached end of a stream.
    EndOfStream,

    /// Invalid type encountered.
    BadType,

    /// Invalid length encountered.
    BadLength,

    /// Error decoding UTF8 string.
    Utf8Error(Utf8Error),

    /// Some other error that does not fit into the above.
    Other(String),
}

impl Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.description())
    }
}

impl Error {
    fn description(&self) -> &str {
        match self {
            &Error::TooBig => "Overflowing value",
            &Error::EndOfStream => "End of stream",
            &Error::BadType => "Invalid type",
            &Error::BadLength => "Invalid length",
            &Error::Utf8Error(_) => "UTF8 Error",
            &Error::Other(ref message) => &message,
        }
    }
}

impl From<Utf8Error> for Error {
    fn from(cause: Utf8Error) -> Error {
        Error::Utf8Error(cause)
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        Error::description(self)
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match self {
            &Error::Utf8Error(ref cause) => Some(cause),
            _ => None,
        }
    }
}

impl ::serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Error {
        Error::Other(msg.to_string())
    }
}

impl ::serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Error {
        ::serde::ser::Error::custom(msg)
    }
}
