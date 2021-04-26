// Copyright 2020 lowRISC contributors.
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
//
// SPDX-License-Identifier: Apache-2.0

//! Kernel interface

use crate::io::Read;
use crate::io::Write;
use crate::protocol::wire::FromWireError;
use crate::protocol::wire::FromWire;
use crate::protocol::wire::ToWireError;
use crate::protocol::wire::ToWire;

use core::convert::TryFrom;
use core::default::Default;

/// Handler mode.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum HandlerMode {
    /// Do not handle request.
    Disabled = 0,

    /// Handle request in user space.
    UserSpace = 1,

    /// Handle request in kernel space.
    KernelSpace = 2,
}

impl Default for HandlerMode {
    fn default() -> Self { Self::Disabled }
}

/// Error for invalid handler mode conversion.
pub struct InvalidHandlerMode;

impl TryFrom<usize> for HandlerMode {
    type Error = InvalidHandlerMode;

    fn try_from(item: usize) -> Result<HandlerMode, Self::Error> {
        match item {
            0 => Ok(HandlerMode::Disabled),
            1 => Ok(HandlerMode::UserSpace),
            2 => Ok(HandlerMode::KernelSpace),
            _ => Err(InvalidHandlerMode),
        }
    }
}

/// Address configuration for SPI device hardware.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AddressConfig {
    /// The address on the SPI device bus that the external flash is accessible at.
    pub flash_virtual_base: u32,

    /// The base address in the external flash device on the SPI host bus.
    pub flash_physical_base: u32,

    /// The size of the external flash device.
    /// This must be a 2^N.
    pub flash_physical_size: u32,

    /// The address on the SPI device bus that the RAM (mailbox) is accessible at.
    pub ram_virtual_base: u32,

    /// The total size available on the SPI device bus.
    /// This must be a 2^N.
    pub virtual_size: u32,
}

impl<'a> FromWire<'a> for AddressConfig {
    fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let flash_virtual_base = r.read_be::<u32>()?;
        let flash_physical_base = r.read_be::<u32>()?;
        let flash_physical_size = r.read_be::<u32>()?;
        let ram_virtual_base = r.read_be::<u32>()?;
        let virtual_size = r.read_be::<u32>()?;
        Ok(Self {
            flash_virtual_base,
            flash_physical_base,
            flash_physical_size,
            ram_virtual_base,
            virtual_size,
        })
    }
}

impl ToWire for AddressConfig {
    fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.flash_virtual_base)?;
        w.write_be(self.flash_physical_base)?;
        w.write_be(self.flash_physical_size)?;
        w.write_be(self.ram_virtual_base)?;
        w.write_be(self.virtual_size)?;
        Ok(())
    }
}
