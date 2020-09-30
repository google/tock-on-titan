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

//! SPI flash protocol.

use crate::io::BeInt;
use crate::io::Read;
use crate::io::Write;
use crate::protocol::wire::FromWireError;
use crate::protocol::wire::FromWire;
use crate::protocol::wire::ToWireError;
use crate::protocol::wire::ToWire;
use crate::protocol::wire::WireEnum;

use core::convert::Into;
use core::convert::TryFrom;
use core::result::Result;

/// SPI flash address modes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AddressMode {
    /// Address is represented by 3 bytes.
    ThreeByte = 0,

    /// Address is represented by 4 bytes.
    FourByte = 1,
}

/// Error for invalid address mode conversion.
pub struct InvalidAddressMode;

impl TryFrom<usize> for AddressMode {
    type Error = InvalidAddressMode;

    fn try_from(item: usize) -> Result<AddressMode, Self::Error> {
        match item {
            0 => Ok(AddressMode::ThreeByte),
            1 => Ok(AddressMode::FourByte),
            _ => Err(InvalidAddressMode),
        }
    }
}

impl From<AddressMode> for usize {
    fn from(item: AddressMode) -> usize {
        item as usize
    }
}

wire_enum! {
    /// SPI flash op codes
    pub enum OpCode: u8 {
        /// No operation
        Nop = 0x00,

        ////////////////////////////////////////////////////////////
        // Status commands

        /// Write to eeprom_status register.
        WriteStatusRegister = 0x01,

        /// Returns contents of eeprom_status register.
        /// Implemented in hardware.
        ReadStatusRegister = 0x05,

        /// Disables writes to device, sets WEL = 0 in hardware.
        WriteDisable = 0x04,

        /// Enables writes to device, sets WEL = 1 in hardware.
        WriteEnable = 0x06,

        /// Suspend write. Software should set WSP or WSE = 1.
        WriteSuspend = 0xb0,

        /// Resumes write. Software should set WSP or WSE = 0.
        WriteResume = 0x30,

        ////////////////////////////////////////////////////////////
        // Erase and program commands

        /// Clears bits of a particular 4KB sector to '1'.
        /// Must be implemented in software. HW sets BUSY bit.
        SectorErase = 0x20,

        /// Clears bits of a particular 32KB block to '1'.
        /// Must be implemented in software. HW sets BUSY bit.
        BlockErase32KB = 0x52,

        /// Clears bits of a particular 64KB block to '1'.
        /// Must be implemented in software. HW sets BUSY bit.
        BlockErase64KB = 0xd8,

        /// Clears all bits to '1'.
        /// Must be implemented in software. HW sets BUSY bit.
        ChipErase = 0xc7,

        /// Alternative op code for ChipErase. HW sets BUSY bit.
        ChipErase2 = 0x60,

        /// Programs up to 256 bytes of memory.
        /// Must be implemented in software. HW sets BUSY bit.
        PageProgram = 0x02,

        ////////////////////////////////////////////////////////////
        // ID commands

        /// Retrieves JEDEC-ID as configured in jedec_id registers.
        /// Implemented in hardware.
        ReadJedec = 0x9f,

        /// Retrieves SFDP as configured in sfdp registers.
        /// Implemented in hardware.
        ReadSfdp = 0x5a,

        ////////////////////////////////////////////////////////////
        // Read commands

        /// Retrieves data. The behavior of this command depends on the selected
        /// mode.
        NormalRead = 0x03,

        /// Retrieves data. The behavior of this command depends on the selected
        /// mode. Fast read includes a 1 byte delay after retrieving the last
        /// bit of the addrees before the first bit of data is delivered.
        FastRead = 0x0b,

        /// Similar to FastReads but uses explicit 4 byte addressing.
        FastRead4B = 0x0c,

        /// Similar to FastRead with output on both MISO and MOSI.
        FastReadDualOutput = 0x3b,

        ////////////////////////////////////////////////////////////
        // Address mode commands

        /// Enable 4 byte address mode.
        /// Must be implemented in software.
        Enter4ByteAddressMode = 0xb7,

        /// Disable 4 byte address mode and revert to 3 byte address mode.
        /// Must be implemented in software.
        Exit4ByteAddressMode = 0xe9,
    }
}

impl<'a> OpCode {
    /// Returns true iff the OpCode requires an address.
    pub fn has_address(&self) -> bool {
        match self {
            Self::SectorErase => true,
            Self::BlockErase32KB => true,
            Self::BlockErase64KB => true,
            Self::PageProgram => true,
            Self::ReadSfdp => true,
            Self::NormalRead => true,
            Self::FastRead => true,
            Self::FastRead4B => true,
            Self::FastReadDualOutput => true,
            _ => false,
        }
    }

    /// Returns true iff the OpCode requires a dummy byte.
    pub fn has_dummy_byte(&self) -> bool {
        match self {
            Self::FastRead => true,
            Self::FastRead4B => true,
            Self::FastReadDualOutput => true,
            _ => false,
        }
    }

    /// Returns true iff the OpCode requires data.
    pub fn has_data(&self) -> bool {
        match self {
            Self::PageProgram => true,
            _ => false,
        }
    }

    /// Returns true iff the OpCode requires for the BUSY bit to clear.
    pub fn wait_busy_clear(&self) -> bool {
        match self {
            Self::WriteSuspend => true,
            Self::WriteResume => true,
            Self::SectorErase => true,
            Self::BlockErase32KB => true,
            Self::BlockErase64KB => true,
            Self::ChipErase => true,
            Self::ChipErase2 => true,
            Self::PageProgram => true,
            _ => false,
        }
    }
}

const DUMMY_BYTE_VALUE: u8 = 0xff;

/// Error used when address cannot be converted.
pub struct AddressConversionError;

/// Address in SPI flash protocol header
pub trait Address: BeInt + Into::<u32> {
    /// Convert from a u32.
    /// Note: This will become redundant once types from `ux` crate implement
    /// `core::convert::TryFrom`.
    fn try_from(val: u32) -> Result<Self, AddressConversionError>;
}

impl Address for ux::u24 {
    fn try_from(val: u32) -> Result<Self, AddressConversionError> {
        // ux::[type]::new just assert!s that the val fits, which results
        // in a panic if it does not. Instead explicitly check and return
        // an error if the val does not fit.
        if val >= u32::from(ux::u24::min_value()) && val <= u32::from(ux::u24::max_value()) {
            Ok(ux::u24::new(val))
        } else {
            Err(AddressConversionError {})
        }
    }
}

impl Address for u32 {
    fn try_from(val: u32) -> Result<Self, AddressConversionError> {
        Ok(val)
    }
}

/// A parsed SPI flash protocol header.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Header<AddrType> {
    /// The SPI op code
    pub opcode: OpCode,

    /// The address.
    ///
    /// Note that not all SPI op codes require an address.
    pub address: Option<AddrType>,
}

impl<'a, AddrType> Header<AddrType>
where AddrType: Address {
    /// Get the address as an Option<u32>
    pub fn get_address(&self) -> Option<u32> {
        self.address.map(|val| val.into())
    }

    /// Deserializes a `Header` from `r`.
    pub fn from_wire<R: Read<'a>>(mut r: R) -> Result<Self, FromWireError> {
        let opcode_u8 = r.read_be::<u8>()?;
        let opcode = OpCode::from_wire_value(opcode_u8).ok_or(FromWireError::OutOfRange)?;

        let address = match opcode.has_address() {
            true => Some(r.read_be::<AddrType>()?),
            false => None,
        };

        if opcode.has_dummy_byte() {
            // We don't actually care about the value, we just need to consume it.
            let _ = r.read_be::<u8>()?;
        }

        Ok(Self {
            opcode,
            address,
        })
    }

    /// Serializes `self` into `w`.
    pub fn to_wire<W: Write>(&self, mut w: W) -> Result<(), ToWireError> {
        w.write_be(self.opcode.to_wire_value())?;
        if self.opcode.has_address() {
            if ! self.address.is_some() {
                return Err(ToWireError::InvalidData)
            }
            w.write_be(self.address.unwrap())?;
        }
        if self.opcode.has_dummy_byte() {
            w.write_be(DUMMY_BYTE_VALUE)?;
        }

        Ok(())
    }
}

impl<'a> FromWire<'a> for Header<ux::u24> {
    fn from_wire<R: Read<'a>>(r: R) -> Result<Self, FromWireError> {
        Self::from_wire(r)
    }
}

impl ToWire for Header<ux::u24> {
    fn to_wire<W: Write>(&self, w: W) -> Result<(), ToWireError> {
        self.to_wire(w)
    }
}

impl<'a> FromWire<'a> for Header<u32> {
    fn from_wire<R: Read<'a>>(r: R) -> Result<Self, FromWireError> {
        Self::from_wire(r)
    }
}

impl ToWire for Header<u32> {
    fn to_wire<W: Write>(&self, w: W) -> Result<(), ToWireError> {
        self.to_wire(w)
    }
}

