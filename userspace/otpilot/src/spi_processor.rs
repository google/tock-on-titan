// Copyright 2021 lowRISC contributors.
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

use crate::manticore_support::Identity;
use crate::manticore_support::NoRsa;
use crate::manticore_support::Reset;
use crate::spi_host;
use crate::spi_host_h1;
use crate::spi_device;

use core::cmp::min;
use core::convert::TryFrom;
use core::fmt::Write;

use libtock::console::Console;
use libtock::result::TockError;

use manticore::io::Cursor as ManticoreCursor;
use manticore::server::pa_rot::PaRot;

use spiutils::io::Cursor as SpiutilsCursor;
use spiutils::io::Write as SpiutilsWrite;
use spiutils::protocol::flash;
use spiutils::protocol::flash::Address;
use spiutils::protocol::flash::AddressMode;
use spiutils::protocol::flash::OpCode;
use spiutils::protocol::payload;
use spiutils::protocol::wire::FromWire;
use spiutils::protocol::wire::FromWireError;
use spiutils::protocol::wire::ToWire;
use spiutils::protocol::wire::ToWireError;

// Size of the SPI flash chip.
// Hard-coded to 64 MiB for now.
// TODO: Modify this to be read from the actual SPI flash chip at runtime.
pub const SPI_FLASH_SIZE: u32 = 0x4000000;

// The location of the mailbox.
// TODO: Make this configurable, possibly by reading it from the SPI flash chip.
pub const SPI_MAILBOX_ADDRESS: u32 = 0xf00000;

// The size of the mailbox.
const SPI_MAILBOX_SIZE: u32 = spi_device::MAX_READ_BUFFER_SIZE as u32;

#[derive(Copy, Clone, Debug)]
pub enum SpiProcessorError {
    FromWire(FromWireError),
    ToWire(ToWireError),
    Tock,
    Manticore(manticore::server::Error),
    UnsupportedContentType(payload::ContentType),
    UnsupportedOpCode(OpCode),
    InvalidAddress(Option<u32>),
    Format(core::fmt::Error),
}

impl From<FromWireError> for SpiProcessorError {
    fn from(err: FromWireError) -> Self {
        SpiProcessorError::FromWire(err)
    }
}

impl From<ToWireError> for SpiProcessorError {
    fn from(err: ToWireError) -> Self {
        SpiProcessorError::ToWire(err)
    }
}

impl From<TockError> for SpiProcessorError {
    fn from(_err: TockError) -> Self {
        SpiProcessorError::Tock
    }
}

impl From<manticore::server::Error> for SpiProcessorError {
    fn from(err: manticore::server::Error) -> Self {
        SpiProcessorError::Manticore(err)
    }
}

impl From<core::fmt::Error> for SpiProcessorError {
    fn from(err: core::fmt::Error) -> Self {
        SpiProcessorError::Format(err)
    }
}

//////////////////////////////////////////////////////////////////////////////

pub struct SpiProcessor<'a> {
    // The Manticore protocol server.
    pub server: PaRot<'a, Identity, Reset, NoRsa>,

    // Whether to print incoming flash headers.
    pub print_flash_headers: bool,
}

const SPI_TX_BUF_SIZE : usize = 512;

pub type SpiProcessorResult<T> = Result<T, SpiProcessorError>;

impl<'a> SpiProcessor<'a> {

    fn send_data(&mut self, tx_header: &payload::Header, tx_buf: &mut[u8]) -> SpiProcessorResult<()> {
        {
            // Scope for tx_cursor (which doesn't implement Drop).
            // We need tx_cursor to go out of scope so that we can use tx_buf further down.
            let tx_cursor = SpiutilsCursor::new(tx_buf);
            tx_header.to_wire(tx_cursor)?;
        }
        spi_device::get().end_transaction_with_data(tx_buf, true, true)?;

        Ok(())
    }

    fn process_manticore(&mut self, mut data: &[u8]) -> SpiProcessorResult<()> {
        let mut console = Console::new();
        writeln!(console, "Device: Manticore!")?;

        let mut tx_buf : [u8; SPI_TX_BUF_SIZE] = [0xff; SPI_TX_BUF_SIZE];
        let payload_len : u16;
        {
            let mut tx_cursor = ManticoreCursor::new(&mut tx_buf[payload::HEADER_LEN..]);
            self.server.process_request(&mut data, &mut tx_cursor)?;
            payload_len = u16::try_from(tx_cursor.consumed_len())
                .map_err(|_| SpiProcessorError::FromWire(FromWireError::OutOfRange))?;
        }
        let tx_header = payload::Header {
            content: payload::ContentType::Manticore,
            content_len: payload_len,
        };
        self.send_data(&tx_header, &mut tx_buf)?;
        writeln!(console, "Device: Data sent")?;
        Ok(())
    }

    fn process_spi_payload(&mut self, mut data: &[u8]) -> SpiProcessorResult<()> {
        let mut console = Console::new();
        let header = payload::Header::from_wire(&mut data)?;
        writeln!(console, "Device: payload header: {:?}", header)?;
        match header.content {
            payload::ContentType::Manticore => {
                self.process_manticore(&data[..header.content_len as usize])
            }
            _ => {
                Err(SpiProcessorError::UnsupportedContentType(header.content))
            }
        }
    }

    // Send data via the SPI host.
    // The transaction is split into smaller transactions that fit into the SPI host's buffer.
    // The write enable status bit is set before each transaction is executed.
    // The `pre_transaction_fn` is executed prior to each transaction.
    fn spi_host_send<AddrType, F>(&self, header: &flash::Header::<AddrType>, mut data: &[u8], pre_transaction_fn: &F) -> SpiProcessorResult<()>
    where AddrType: Address,
        F: Fn() -> SpiProcessorResult<()>
    {
        // We need to update the header so copy it.
        let mut header = *header;
        loop {
            pre_transaction_fn()?;

            let mut tx_buf = [0xff; spi_host::MAX_READ_BUFFER_LENGTH];
            let tx_len : usize;
            let data_len_to_send : usize;
            {
                let mut tx_cursor = SpiutilsCursor::new(&mut tx_buf);
                header.to_wire(&mut tx_cursor)?;
                if header.opcode.has_dummy_byte() {
                    // Skip one dummy byte (send 0x0)
                    tx_cursor.write_bytes(&[0x0; 1])
                        .map_err(|err| SpiProcessorError::ToWire(ToWireError::Io(err)))?;
                }

                data_len_to_send = min(spi_host::MAX_READ_BUFFER_LENGTH - tx_cursor.consumed_len(), data.len());
                tx_cursor.write_bytes(&data[..data_len_to_send])
                    .map_err(|err| SpiProcessorError::ToWire(ToWireError::Io(err)))?;

                tx_len = tx_cursor.consumed_len()
            }

            spi_host_h1::get().set_wait_busy_clear_in_transactions(header.opcode.wait_busy_clear())?;
            spi_host::get().read_write_bytes(&mut tx_buf, tx_len)?;
            spi_host::get().wait_read_write_done();

            // Move data and address forward
            data = &data[data_len_to_send..];
            if let Some(addr) = header.address {
                let delta : u32 = core::convert::TryFrom::<usize>::try_from(data_len_to_send)
                    .map_err(|_| SpiProcessorError::FromWire(FromWireError::OutOfRange))?;
                let next_addr = addr.into() + delta;
                header.address = Some(AddrType::try_from(next_addr)
                    .map_err(|_| SpiProcessorError::FromWire(FromWireError::OutOfRange))?);
            }

            if data.len() == 0 { break; }
        }
        Ok(())
    }

    // Send a "write enable" command via the SPI host.
    fn spi_host_write_enable(&self) -> SpiProcessorResult<()> {
        let header = flash::Header::<u32> {
            opcode: OpCode::WriteEnable,
            address: None,
        };

        // The command has no data.
        let data : [u8; 0] = [0; 0];
        self.spi_host_send(&header, &data, &|| Ok(()))
    }

    // Send a "write" type command (e.g. PageProgram, *Erase) via the SPI host.
    // This splits the data into smaller transactions as needed and executes
    // "enable write" for each transaction.
    fn spi_host_write<AddrType>(&self, header: &flash::Header::<AddrType>, data: &[u8]) -> SpiProcessorResult<()>
    where AddrType: Address {
        self.spi_host_send(header, data, &|| self.spi_host_write_enable())
    }

    fn clear_device_status(&self, clear_busy: bool, clear_write_enable: bool) -> SpiProcessorResult<()> {
        spi_device::get().end_transaction_with_status(clear_busy, clear_write_enable)?;
        Ok(())
    }

    fn process_spi_header<AddrType>(&mut self, header: &flash::Header::<AddrType>, rx_buf: &[u8]) -> SpiProcessorResult<()>
    where AddrType: Address {
        let mut data: &[u8] = rx_buf;
        if header.opcode.has_dummy_byte() {
            // Skip dummy byte
            data = &rx_buf[1..];
        }
        match header.opcode {
            OpCode::PageProgram => {
                match header.get_address() {
                    Some(x) if x >= SPI_MAILBOX_ADDRESS && x < SPI_MAILBOX_ADDRESS + SPI_MAILBOX_SIZE => {
                        if spi_device::get().is_write_enable_set() {
                            self.process_spi_payload(data)?;
                        }
                        self.clear_device_status(true, true)
                    }
                    Some(x) if x < SPI_MAILBOX_ADDRESS || x >= SPI_MAILBOX_ADDRESS + SPI_MAILBOX_SIZE => {
                        if spi_device::get().is_write_enable_set() {
                            // Pass through to SPI host
                            self.spi_host_write(header, data)?;
                        }
                        self.clear_device_status(true, true)
                    }
                    _ => return Err(SpiProcessorError::InvalidAddress(header.get_address())),
                }
            }
            OpCode::SectorErase | OpCode::BlockErase32KB | OpCode::BlockErase64KB => {
                match header.get_address() {
                    Some(x) if x >= SPI_MAILBOX_ADDRESS && x < SPI_MAILBOX_ADDRESS + SPI_MAILBOX_SIZE => {
                        // Nothing to do.
                        self.clear_device_status(true, true)
                    }
                    Some(x) if x < SPI_MAILBOX_ADDRESS || x >= SPI_MAILBOX_ADDRESS + SPI_MAILBOX_SIZE => {
                        if spi_device::get().is_write_enable_set() {
                            // Pass through to SPI host
                            self.spi_host_write(header, data)?;
                        }
                        self.clear_device_status(true, true)
                    }
                    _ => return Err(SpiProcessorError::InvalidAddress(header.get_address())),
                }
            }
            OpCode::ChipErase | OpCode::ChipErase2 => {
                if spi_device::get().is_write_enable_set() {
                    // Pass through to SPI host
                    self.spi_host_write(header, data)?;
                }
                self.clear_device_status(true, true)
            }
            _ => return Err(SpiProcessorError::UnsupportedOpCode(header.opcode)),
        }
    }

    pub fn process_spi_packet(&mut self, mut rx_buf: &[u8]) -> SpiProcessorResult<()> {
        let mut console = Console::new();
        match spi_device::get().get_address_mode() {
            AddressMode::ThreeByte => {
                let header = flash::Header::<ux::u24>::from_wire(&mut rx_buf)?;
                if self.print_flash_headers {
                    writeln!(console, "Device: flash header (3B): {:?}", header)?;
                }
                self.process_spi_header(&header, rx_buf)
            }
            AddressMode::FourByte => {
                let header = flash::Header::<u32>::from_wire(&mut rx_buf)?;
                if self.print_flash_headers {
                    writeln!(console, "Device: flash header (4B): {:?}", header)?;
                }
                self.process_spi_header(&header, rx_buf)
            }
        }
    }
}
