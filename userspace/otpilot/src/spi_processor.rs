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

use crate::firmware_controller::FirmwareController;
use crate::globalsec;
use crate::manticore_support::Identity;
use crate::manticore_support::NoRsa;
use crate::manticore_support::Reset;
use crate::reset;
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
use spiutils::driver::firmware::SegmentInfo;
use spiutils::protocol::error;
use spiutils::protocol::error::Message as ErrorMessage;
use spiutils::protocol::firmware;
use spiutils::protocol::firmware::Message;
use spiutils::protocol::flash as spi_flash;
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
// TODO(osenft): Modify this to be read from the actual SPI flash chip at runtime.
pub const SPI_FLASH_SIZE: u32 = 0x4000000;

// The location of the mailbox.
// TODO(osenft): Make this configurable, possibly by reading it from the SPI flash chip.
pub const SPI_MAILBOX_ADDRESS: u32 = 0x80000;

// The size of the mailbox.
const SPI_MAILBOX_SIZE: u32 = spi_device::MAX_READ_BUFFER_SIZE as u32;

#[derive(Copy, Clone, Debug)]
pub enum SpiProcessorError {
    FromWire(FromWireError),
    ToWire(ToWireError),
    Tock,
    Manticore(manticore::server::Error),
    UnsupportedFirmwareOperation(firmware::ContentType),
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

    pub firmware: FirmwareController,
}

const SPI_TX_BUF_SIZE : usize = 512;

// TODO(osk): We need to have this tx_buf somewhere, but putting it on the stack
// doesn't work, since that's currently limited to 2048 bytes. Declaring it
// static here for now until we have a better place for it to live.
static mut SPI_TX_BUF : [u8; SPI_TX_BUF_SIZE] = [0xff; SPI_TX_BUF_SIZE];

pub type SpiProcessorResult<T> = Result<T, SpiProcessorError>;

impl<'a> SpiProcessor<'a> {

    fn send_data(&mut self, content_type: payload::ContentType, content_len: u16, tx_buf: &mut[u8]) -> SpiProcessorResult<()> {
        let mut header = payload::Header {
            content: content_type,
            content_len: content_len,
            checksum: 0,
        };
        header.checksum = payload::compute_checksum(&header, &tx_buf[payload::HEADER_LEN..]);
        {
            // Scope for tx_cursor.
            // We need tx_cursor to go out of scope so that we can use tx_buf further down.
            let tx_cursor = SpiutilsCursor::new(tx_buf);
            header.to_wire(tx_cursor)?;
        }
        spi_device::get().end_transaction_with_data(
            &mut tx_buf[..payload::HEADER_LEN + content_len as usize], true, true)?;

        Ok(())
    }

    fn send_error<'m, M: ErrorMessage<'m>>(&mut self, msg: M) -> SpiProcessorResult<()> {
        let payload_len : u16;
        unsafe {
            // TODO(osk): We need the unsafe block since we're accessing SPI_TX_BUF as &mut.
            let mut tx_cursor = SpiutilsCursor::new(&mut SPI_TX_BUF[payload::HEADER_LEN..]);

            let header = error::Header {
                content: M::TYPE
            };
            header.to_wire(&mut tx_cursor)?;
            msg.to_wire(&mut tx_cursor)?;
            payload_len = u16::try_from(tx_cursor.consumed_len())
                    .map_err(|_| SpiProcessorError::FromWire(FromWireError::OutOfRange))?;
        }
        unsafe {
            // TODO(osk): We need the unsafe block since we're accessing SPI_TX_BUF as &mut.
            self.send_data(payload::ContentType::Error, payload_len, &mut SPI_TX_BUF)?;
        }
        Ok(())
    }

    fn process_manticore(&mut self, mut data: &[u8]) -> SpiProcessorResult<()> {
        let payload_len : u16;
        {
            unsafe {
                // TODO(osk): We need the unsafe block since we're accessing SPI_TX_BUF as &mut.
                let mut tx_cursor = ManticoreCursor::new(&mut SPI_TX_BUF[payload::HEADER_LEN..]);
                self.server.process_request(&mut data, &mut tx_cursor)?;
                payload_len = u16::try_from(tx_cursor.consumed_len())
                    .map_err(|_| SpiProcessorError::FromWire(FromWireError::OutOfRange))?;
            }
        }
        unsafe {
            // TODO(osk): We need the unsafe block since we're accessing SPI_TX_BUF as &mut.
            self.send_data(payload::ContentType::Manticore, payload_len, &mut SPI_TX_BUF)?;
        }
        Ok(())
    }

    fn send_firmware_response<'m, M: Message<'m>>(&mut self, response: M) -> SpiProcessorResult<()> {
        let payload_len : u16;
        unsafe {
            // TODO(osk): We need the unsafe block since we're accessing SPI_TX_BUF as &mut.
            let mut tx_cursor = SpiutilsCursor::new(&mut SPI_TX_BUF[payload::HEADER_LEN..]);

            let fw_header = firmware::Header {
                content: M::TYPE
            };
            fw_header.to_wire(&mut tx_cursor)?;
            response.to_wire(&mut tx_cursor)?;
            payload_len = u16::try_from(tx_cursor.consumed_len())
                .map_err(|_| SpiProcessorError::FromWire(FromWireError::OutOfRange))?;
        }
        unsafe {
            // TODO(osk): We need the unsafe block since we're accessing SPI_TX_BUF as &mut.
            self.send_data(payload::ContentType::Firmware, payload_len, &mut SPI_TX_BUF)?;
        }
        Ok(())
    }

    fn process_firmware_inactive_segments(&mut self, mut data: &[u8]) -> SpiProcessorResult<()> {
        let _ = firmware::InactiveSegmentsInfoRequest::from_wire(&mut data)?;

        let response = firmware::InactiveSegmentsInfoResponse {
            ro: globalsec::get().get_inactive_ro(),
            rw: globalsec::get().get_inactive_rw(),
        };
        self.send_firmware_response(response)
    }

    fn process_firmware_update_prepare(&mut self, mut data: &[u8]) -> SpiProcessorResult<()> {
        let req = firmware::UpdatePrepareRequest::from_wire(&mut data)?;
        let segment: SegmentInfo;

        if req.segment_and_location == globalsec::get().get_inactive_rw().identifier {
            segment = globalsec::get().get_inactive_rw();
        } else if req.segment_and_location == globalsec::get().get_inactive_ro().identifier {
            segment = globalsec::get().get_inactive_ro();
        } else {
            let response = firmware::UpdatePrepareResponse {
                segment_and_location: req.segment_and_location,
                max_chunk_length: 0,
                result: firmware::UpdatePrepareResult::InvalidSegmentAndLocation,
            };
            return self.send_firmware_response(response);
        }

        match self.firmware.erase_segment(segment) {
            Ok(()) => {
                let response = firmware::UpdatePrepareResponse {
                    segment_and_location: req.segment_and_location,
                    max_chunk_length: self.firmware.get_max_write_chunk_length() as u16,
                    result: firmware::UpdatePrepareResult::Success,
                };
                self.send_firmware_response(response)
            },
            Err(why) => {
                let mut console = Console::new();
                let _ = writeln!(console, "update_prepare failed: {:?}", why);
                let response = firmware::UpdatePrepareResponse {
                    segment_and_location: req.segment_and_location,
                    max_chunk_length: 0,
                    result: firmware::UpdatePrepareResult::Error,
                };
                self.send_firmware_response(response)
            }
        }
    }

    fn send_firmware_write_chunk_response(&mut self, req: &firmware::WriteChunkRequest, result: firmware::WriteChunkResult) -> SpiProcessorResult<()> {
        let response = firmware::WriteChunkResponse {
            segment_and_location: req.segment_and_location,
            offset: req.offset,
            result: result,
        };
        self.send_firmware_response(response)
    }

    fn process_firmware_write_chunk(&mut self, mut data: &[u8]) -> SpiProcessorResult<()> {
        let req: firmware::WriteChunkRequest;
        {
            req = firmware::WriteChunkRequest::from_wire(&mut data)?;
        }
        let segment: SegmentInfo;

        if req.segment_and_location == globalsec::get().get_inactive_rw().identifier {
            segment = globalsec::get().get_inactive_rw();
        } else if req.segment_and_location == globalsec::get().get_inactive_ro().identifier {
            segment = globalsec::get().get_inactive_ro();
        } else {
            return self.send_firmware_write_chunk_response(&req, firmware::WriteChunkResult::InvalidSegmentAndLocation);
        }

        if req.offset >= segment.size {
            return self.send_firmware_write_chunk_response(&req, firmware::WriteChunkResult::InvalidOffset);
        }

        if req.offset + req.data.len() as u32 > segment.size || req.data.len() > self.firmware.get_max_write_chunk_length() {
            return self.send_firmware_write_chunk_response(&req, firmware::WriteChunkResult::DataTooLong);
        }

        let result = match self.firmware.write_and_verify_segment_chunk(segment, req.offset as usize, req.data) {
            Err(_why) => firmware::WriteChunkResult::Error,
            Ok(false) => firmware::WriteChunkResult::CompareFailed,
            Ok(true) => firmware::WriteChunkResult::Success,
        };

        self.send_firmware_write_chunk_response(&req, result)
    }

    fn send_firmware_reboot_response(&mut self, req: &firmware::RebootRequest, result: firmware::RebootResult) -> SpiProcessorResult<()> {
        let response = firmware::RebootResponse {
            time: req.time,
            result: result,
        };
        self.send_firmware_response(response)
    }

    fn process_firmware_reboot(&mut self, mut data: &[u8]) -> SpiProcessorResult<()> {
        let req: firmware::RebootRequest;
        {
            req = firmware::RebootRequest::from_wire(&mut data)?;
        }

        let result = match req.time {
            firmware::RebootTime::Immediate => {
                if let Err(_) = reset::get().reset() {
                    firmware::RebootResult::Error
                } else {
                    firmware::RebootResult::Success
                }
            },
            firmware::RebootTime::Delayed => {
                // TODO(https://github.com/google/tock-on-titan/issues/236): Implement this.
                firmware::RebootResult::Error
            },
        };

        self.send_firmware_reboot_response(&req, result)
    }

    fn process_firmware(&mut self, mut data: &[u8]) -> SpiProcessorResult<()> {
        let header = firmware::Header::from_wire(&mut data)?;

        let result = match header.content {
            firmware::ContentType::InactiveSegmentsInfoRequest => {
                self.process_firmware_inactive_segments(&mut data)
            },
            firmware::ContentType::UpdatePrepareRequest => {
                self.process_firmware_update_prepare(&mut data)
            },
            firmware::ContentType::WriteChunkRequest => {
                self.process_firmware_write_chunk(&mut data)
            },
            firmware::ContentType::RebootRequest => {
                self.process_firmware_reboot(&mut data)
            },
            _ => {
                Err(SpiProcessorError::UnsupportedFirmwareOperation(header.content))
            }
        };

        result
    }

    fn process_spi_payload(&mut self, mut data: &[u8]) -> SpiProcessorResult<()> {
        let header = payload::Header::from_wire(&mut data)?;
        if header.checksum != payload::compute_checksum(&header, data) {
            let error = error::BadChecksum {};
            return self.send_error(error);
        }

        match header.content {
            payload::ContentType::Manticore => {
                self.process_manticore(&data[..header.content_len as usize])
            }
            payload::ContentType::Firmware => {
                self.process_firmware(&data[..header.content_len as usize])
            }
            _ => {
                let error = error::ContentTypeNotSupported {};
                self.send_error(error)
            }
        }
    }

    // Send data via the SPI host.
    // The transaction is split into smaller transactions that fit into the SPI host's buffer.
    // The write enable status bit is set before each transaction is executed.
    // The `pre_transaction_fn` is executed prior to each transaction.
    fn spi_host_send<AddrType, F>(&self, header: &spi_flash::Header::<AddrType>, mut data: &[u8], pre_transaction_fn: &F) -> SpiProcessorResult<()>
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
        let header = spi_flash::Header::<u32> {
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
    fn spi_host_write<AddrType>(&self, header: &spi_flash::Header::<AddrType>, data: &[u8]) -> SpiProcessorResult<()>
    where AddrType: Address {
        self.spi_host_send(header, data, &|| self.spi_host_write_enable())
    }

    fn clear_device_status(&self, clear_busy: bool, clear_write_enable: bool) -> SpiProcessorResult<()> {
        spi_device::get().end_transaction_with_status(clear_busy, clear_write_enable)?;
        Ok(())
    }

    // Check if the specified address is within the mailbox address space.
    fn is_mailbox_address(&self, addr: u32) -> bool {
        addr >= SPI_MAILBOX_ADDRESS && addr < SPI_MAILBOX_ADDRESS + SPI_MAILBOX_SIZE
    }

    fn process_spi_header<AddrType>(&mut self, header: &spi_flash::Header::<AddrType>, rx_buf: &[u8]) -> SpiProcessorResult<()>
    where AddrType: Address {
        let mut data: &[u8] = rx_buf;
        if header.opcode.has_dummy_byte() {
            // Skip dummy byte
            data = &rx_buf[1..];
        }
        match header.opcode {
            OpCode::PageProgram => {
                match header.get_address() {
                    Some(addr) if self.is_mailbox_address(addr) => {
                        if spi_device::get().is_write_enable_set() {
                            self.process_spi_payload(data)?;
                        }
                        self.clear_device_status(true, true)
                    }
                    Some(addr) if !self.is_mailbox_address(addr) => {
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
                    Some(addr) if self.is_mailbox_address(addr) => {
                        // Nothing to do.
                        self.clear_device_status(true, true)
                    }
                    Some(addr) if !self.is_mailbox_address(addr) => {
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
                let header = spi_flash::Header::<ux::u24>::from_wire(&mut rx_buf)?;
                if self.print_flash_headers {
                    writeln!(console, "Device: flash header (3B): {:?}", header)?;
                }
                self.process_spi_header(&header, rx_buf)
            }
            AddressMode::FourByte => {
                let header = spi_flash::Header::<u32>::from_wire(&mut rx_buf)?;
                if self.print_flash_headers {
                    writeln!(console, "Device: flash header (4B): {:?}", header)?;
                }
                self.process_spi_header(&header, rx_buf)
            }
        }
    }
}
