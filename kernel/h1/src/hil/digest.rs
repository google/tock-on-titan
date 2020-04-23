// Copyright 2018 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::common::SyscallError;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DigestMode {
    /// Generates a SHA-1 digest. Output size is 160 bits (20 bytes).
    Sha1,
    /// Generates a SHA-2 256-bit digest. Output size is 256 bits (32 bytes).
    Sha256,
    /// Generates a SHA-2 256-bit HMAC. Output size is 256 bits (32 bytes).
    Sha256Hmac,
}

impl DigestMode {
    pub fn output_size(&self) -> usize {
        match *self {
            DigestMode::Sha1 => 160 / 8,
            DigestMode::Sha256 => 256 / 8,
            DigestMode::Sha256Hmac => 256 / 8,
        }
    }
}
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DigestError {
    /// The requested digest type is not supported by this hardware.
    EngineNotSupported,
    /// `update` or `finalize` where called before `initialize`.
    NotConfigured,
    /// The supplied output buffer is too small. Parameter is the required buffer size.
    BufferTooSmall(usize),
    Timeout,
}

impl From<DigestError> for SyscallError {
    fn from(e: DigestError) -> Self {
        match e {
            DigestError::EngineNotSupported => SyscallError::NotImplemented,
            DigestError::NotConfigured => SyscallError::InvalidState,
            DigestError::BufferTooSmall(_) => SyscallError::OutOfRange,
            DigestError::Timeout => SyscallError::ResourceBusy,
        }
    }
}

pub trait DigestEngine {
    /// Initializes the digest engine for the given mode.
    fn initialize(&self, mode: DigestMode) -> Result<(), DigestError>;

    /// Initialize for HMAC operation with a key.
    fn initialize_hmac(&self, key: &[u8]) -> Result<(), DigestError>;

    /// Initialize for generating a particular certificate (hidden secret)
    fn initialize_certificate(&self, certificate_id: u32) -> Result<(), DigestError>;

    /// Feeds data into the digest. Returns the number of bytes that were actually consumed from
    /// the input.
    fn update(&self, data: &[u8]) -> Result<usize, DigestError>;

    /// Finalizes the digest, and stores it in the `output` buffer. Returns the number of bytes
    /// stored.
    fn finalize(&self, output: &mut [u8]) -> Result<usize, DigestError>;

    /// Finalize withtout seeing the result; this is used for certificates
    /// (hidden secret generation). Ok is always Ok(0); passes a usize
    /// to match the finalize() signature.
    fn finalize_hidden(&self) -> Result<usize, DigestError>;

}
