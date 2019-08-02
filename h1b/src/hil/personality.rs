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

//! Interface for accessing H1B device personality (individual attestation
//! data). Called "Personality" to remain consistent with ec-cr52 codebase.

use kernel::ReturnCode;

/// Structure of device attestation data.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PersonalityData {
    pub checksum: [u32; 8],
    pub salt: [u32; 8],
    pub pub_x: [u32; 8],
    pub pub_y: [u32; 8],
    pub certificate_hash: [u32; 8],
    pub certificate_len: u32,
    pub certificate: [u8; 2048 - (4 + 5 * 32)],
}


/// Trait for getting and setting device attestation data.
///
/// Implementors should assume the client implements the
/// [Client](trait.Client.html) trait.
pub trait Personality<'a> {
    /// Set the client for callbacks on set calls.
    fn set_client(&self, client: &'a Client);

    /// Fetch the device's attestation data into a typed PersonalityData
    /// structure.
    fn get(&self, personality: &mut PersonalityData);
    /// Fetch the device's attestation data into a slice; this slice
    /// must be at least 2048 bytes long.
    fn get_u8(&self, personality: &mut [u8]) -> ReturnCode;

    /// Set the device's attestation data.
    fn set(&self, personality: &PersonalityData) -> ReturnCode;
    /// Set the device's attestation data from a slice; this slice
    /// must be at least 2048 bytes long.
    fn set_u8(&self, personality: &[u8]) -> ReturnCode;
}

/// A [Personality](trait.Personality.html) client
///
/// Clients of a [Personality](trait.Personality.html) must implement this
/// trait.
pub trait Client {
    /// Called by (Personality)[trait.Personality.html] when the device's
    /// personality has been committed to nonvolatile storage.
    fn set_done(&self, rval: ReturnCode);
}
