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

#![no_std]

mod sfdp;
mod spi_host;
mod spi_host_h1;
mod spi_device;

use core::convert::TryFrom;
use core::fmt::Write;
use core::time::Duration;

use libtock::console::Console;
use libtock::result::TockError;
use libtock::result::TockResult;

use manticore::crypto::rsa;
use manticore::hardware;
use manticore::io::Cursor as ManticoreCursor;
use manticore::protocol::capabilities::*;
use manticore::protocol::device_id;
use manticore::server::pa_rot::{PaRot, Options};

use spiutils::io::Cursor as SpiutilsCursor;
use spiutils::io::Write as _;
use spiutils::driver::HandlerMode;
use spiutils::protocol::flash;
use spiutils::protocol::flash::AddressMode;
use spiutils::protocol::flash::OpCode;
use spiutils::protocol::payload;
use spiutils::protocol::wire::FromWire;
use spiutils::protocol::wire::FromWireError;
use spiutils::protocol::wire::ToWire;
use spiutils::protocol::wire::ToWireError;

//////////////////////////////////////////////////////////////////////////////

struct SpiHostDemo;

impl SpiHostDemo {
    pub fn enable_4b(&self) -> TockResult<()> {
        spi_host::get().read_write_bytes(&mut [0xb7], 1)?;
        spi_host::get().wait_read_write_done();
        Ok(())
    }

    fn create_tx_buf(&self, cmd: u8, addr: u32) -> ([u8; spi_host::MAX_READ_BUFFER_LENGTH], usize) {
        let mut tx = [0xff; spi_host::MAX_READ_BUFFER_LENGTH];
        tx[0] = cmd;
        tx[1..5].copy_from_slice(&addr.to_be_bytes());
        (tx, 5)
    }

    pub fn read_data<'a>(&self, addr: u32, rx_len: usize) -> TockResult<&'static[u8]> {
        let (mut tx, tx_len) = self.create_tx_buf(0x03, addr);
        spi_host::get().read_write_bytes(&mut tx, tx_len + rx_len)?;
        spi_host::get().wait_read_write_done();
        Ok(&spi_host::get().get_read_buffer()[tx_len..])
    }

    pub fn read_and_print_data(&self, addr: u32) -> TockResult<()> {
        let mut console = Console::new();

        let rx_buf = self.read_data(addr, 8)?;
        writeln!(console, "Host: Result: {:02x?}", rx_buf)?;
        Ok(())
    }
}

//////////////////////////////////////////////////////////////////////////////

const NETWORKING: Networking = Networking {
    max_message_size: 1024,
    max_packet_size: 256,
    mode: RotMode::Platform,
    roles: BusRole::HOST,
};

const TIMEOUTS: Timeouts = Timeouts {
    regular: Duration::from_millis(30),
    crypto: Duration::from_millis(200),
};

const DEVICE_ID: device_id::DeviceIdentifier =
    device_id::DeviceIdentifier {
        vendor_id: 1,
        device_id: 2,
        subsys_vendor_id: 3,
        subsys_id: 4,
    };

struct Identity {
    version: [u8; 32],
    device_id: [u8; 64],
}
impl hardware::Identity for Identity {
    fn firmware_version(&self) -> &[u8; 32] {
        &self.version
    }
    fn unique_device_identity(&self) -> &[u8] {
        &self.device_id
    }
}

struct Reset;
impl hardware::Reset for Reset {
    fn resets_since_power_on(&self) -> u32 {
        0
    }
    fn uptime(&self) -> Duration {
        Duration::from_millis(1)
    }
}

struct NoRsaPubKey;
impl rsa::PublicKey for NoRsaPubKey {
    fn len(&self) -> rsa::ModulusLength {
        unreachable!()
    }
}

struct NoRsaEngine;
impl rsa::Engine for NoRsaEngine {
    type Error = ();
    type Key = NoRsaPubKey;

    fn verify_signature(
        &mut self,
        _signature: &[u8],
        _message: &[u8],
    ) -> Result<(), ()> {
        Err(())
    }
}

struct NoRsa;
impl rsa::Builder for NoRsa {
    type Engine = NoRsaEngine;

    fn supports_modulus(&self, _: rsa::ModulusLength) -> bool {
        true
    }

    fn new_engine(&self, _key: NoRsaPubKey) -> Result<NoRsaEngine, ()> {
        Err(())
    }
}

//////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug)]
enum SpiProcessorError {
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

struct SpiProcessor<'a> {
    server: PaRot<'a, Identity, Reset, NoRsa>,
}

const SPI_TX_BUF_SIZE : usize = 512;

type SpiProcessorResult<T> = Result<T, SpiProcessorError>;

impl<'a> SpiProcessor<'a> {

    fn send_data(&mut self, tx_header: &payload::Header, tx_buf: &mut[u8]) -> SpiProcessorResult<()> {
        {
            // Scope for tx_cursor (which doesn't implement Drop).
            // We need tx_cursor to go out of scope so that we can use tx_buf further down.
            let tx_cursor = SpiutilsCursor::new(tx_buf);
            tx_header.to_wire(tx_cursor)?;
        }
        spi_device::get().send_data(tx_buf, true, true)?;

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

    fn spi_host_write_enable(&mut self) -> SpiProcessorResult<()> {
        let header = flash::Header::<u32> {
            opcode: OpCode::WriteEnable,
            address: None,
        };
        let data : [u8; 0] = [0; 0];
        self.spi_host_send(&header, &data)
    }

    fn spi_write_to_buf<H>(&mut self, header: &H, data: &[u8], mut buf: &mut[u8])
    -> SpiProcessorResult<usize>
    where H: flash::SpiHeader + ToWire {
        let mut tx_cursor = SpiutilsCursor::new(&mut buf);
        header.to_wire(&mut tx_cursor)?;
        if header.get_opcode().has_dummy_byte() {
            // Skip one dummy byte (send 0x0)
            tx_cursor.write_bytes(&[0x0; 1])
                .map_err(|err| SpiProcessorError::ToWire(ToWireError::Io(err)))?;
        }

        tx_cursor.write_bytes(&data)
            .map_err(|err| SpiProcessorError::ToWire(ToWireError::Io(err)))?;

        Ok(tx_cursor.consumed_len())
    }

    fn spi_host_send<H>(&mut self, header: &H, data: &[u8]) -> SpiProcessorResult<()>
    where H: flash::SpiHeader + ToWire {
        let mut tx_buf = [0xff; spi_host::MAX_READ_BUFFER_LENGTH];
        let tx_len = self.spi_write_to_buf(header, data, &mut tx_buf)?;

        spi_host_h1::get().set_wait_busy_clear_in_transactions(header.get_opcode().wait_busy_clear())?;
        spi_host::get().read_write_bytes(&mut tx_buf, tx_len)?;
        spi_host::get().wait_read_write_done();

        Ok(())
    }

    fn clear_device_status(&self, clear_busy: bool, clear_write_enable: bool) -> SpiProcessorResult<()> {
        spi_device::get().clear_status(clear_busy, clear_write_enable)?;
        Ok(())
    }

    fn process_spi_header<H>(&mut self, header: &H, rx_buf: &[u8]) -> SpiProcessorResult<()>
    where H: flash::SpiHeader + ToWire
    {
        let mut data: &[u8] = rx_buf;
        if header.get_opcode().has_dummy_byte() {
            // Skip dummy byte
            data = &rx_buf[1..];
        }
        match header.get_opcode() {
            OpCode::PageProgram => {
                match header.get_address() {
                    Some(0x02000000) => {
                        if spi_device::get().is_write_enable_set() {
                            self.process_spi_payload(data)?;
                        }
                        self.clear_device_status(true, true)
                    }
                    Some(x) if x < 0x02000000 => {
                        if spi_device::get().is_write_enable_set() {
                            // Pass through to SPI host
                            self.spi_host_write_enable()?;
                            self.spi_host_send(header, data)?;
                        }
                        self.clear_device_status(true, true)
                    }
                    _ => return Err(SpiProcessorError::InvalidAddress(header.get_address())),
                }
            }
            OpCode::SectorErase | OpCode::BlockErase32KB | OpCode::BlockErase64KB => {
                match header.get_address() {
                    Some(0x02000000) => {
                        // Nothing to do.
                        self.clear_device_status(true, true)
                    }
                    Some(x) if x < 0x02000000 => {
                        if spi_device::get().is_write_enable_set() {
                            // Pass through to SPI host
                            self.spi_host_write_enable()?;
                            self.spi_host_send(header, data)?;
                        }
                        self.clear_device_status(true, true)
                    }
                    _ => return Err(SpiProcessorError::InvalidAddress(header.get_address())),
                }
            }
            OpCode::ChipErase | OpCode::ChipErase2 => {
                if spi_device::get().is_write_enable_set() {
                    // Pass through to SPI host
                    self.spi_host_write_enable()?;
                    self.spi_host_send(header, data)?;
                }
                self.clear_device_status(true, true)
            }
            _ => return Err(SpiProcessorError::UnsupportedOpCode(header.get_opcode())),
        }
    }

    fn process_spi_packet(&mut self, mut rx_buf: &[u8]) -> SpiProcessorResult<()> {
        let mut console = Console::new();
        match spi_device::get().get_address_mode() {
            AddressMode::ThreeByte => {
                let header = flash::Header::<ux::u24>::from_wire(&mut rx_buf)?;
                writeln!(console, "Device: flash header (3B): {:?}", header)?;
                self.process_spi_header(&header, rx_buf)
            }
            AddressMode::FourByte => {
                let header = flash::Header::<u32>::from_wire(&mut rx_buf)?;
                writeln!(console, "Device: flash header (4B): {:?}", header)?;
                self.process_spi_header(&header, rx_buf)
            }
        }
    }
}

//////////////////////////////////////////////////////////////////////////////

fn run() -> TockResult<()> {
    let mut console = Console::new();

    //////////////////////////////////////////////////////////////////////////////

    // We cannot use the SPI host if passthrough is enabled.
    spi_host_h1::get().set_passthrough(false)?;

    let host_demo = SpiHostDemo {};

    writeln!(console, "Host: Enabling 4B mode")?;
    host_demo.enable_4b()?;

    writeln!(console, "Host: Reading data")?;
    host_demo.read_and_print_data(0x0)?;
    host_demo.read_and_print_data(0x1)?;

    let mut identity = Identity {
        version: [0; 32],
        device_id: [0; 64],
    };

    //////////////////////////////////////////////////////////////////////////////

    {
        let mut idx : usize = 0;
        for val in "v1.00".as_bytes() {
            if idx > identity.version.len() { break; }
            identity.version[idx] = *val;
            idx = idx + 1;
        }
    }

    {
        let mut idx : usize = 0;
        for val in "1234567890".as_bytes() {
            if idx > identity.device_id.len() { break; }
            identity.device_id[idx] = *val;
            idx = idx + 1;
        }
    }

    let mut processor = SpiProcessor {
        server: PaRot::new(Options {
            identity: &identity,
            reset: &Reset,
            rsa: &NoRsa,
            device_id: DEVICE_ID,
            networking: NETWORKING,
            timeouts: TIMEOUTS,
        }),
    };

    writeln!(console, "Device: Configuring address_mode handling to KernelSpace")?;
    spi_device::get().set_address_mode_handling(HandlerMode::KernelSpace)?;

    // OpenTitan JEDEC ID
    /*
    spi_device::get().set_jedec_id(&mut [
        0x26, // Manufacturer (Visic, should actually be
              // 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x26)
        0x31, // Device (OpenTitan)
        0x19, // Size (2^25 = 256 Mb)
        ])?;
    */

    // Legacy JEDEC ID
    spi_device::get().set_jedec_id(&mut [
        0x26, // Manufacturer
        0x02, // Device
        0x17, // Size
        ])?;

    {
        let mut sfdp = [0xff; 128];
        sfdp::get_table(
            &mut sfdp,
            0x2000000 * 8, // image_size_bits
            spi_device::get().get_address_mode(), // startup_address_mode
            spi_device::get().get_address_mode() == AddressMode::ThreeByte, // support_address_mode_switch
            0x2000000, // mailbox_offset
            spi_device::MAX_READ_BUFFER_SIZE as u32, // mailbox_size
            0 // google_capabilities
            ).map_err(|_| TockError::Format)?;
        spi_device::get().set_sfdp(&mut sfdp)?;
    }

    //////////////////////////////////////////////////////////////////////////////

    // We need SPI passthrough to be fully operational.
    spi_host_h1::get().set_passthrough(true)?;

    loop {
        writeln!(console, "Device: Waiting for transaction")?;
        spi_device::get().wait_for_transaction();

        let rx_buf = spi_device::get().get_read_buffer();
        writeln!(console, "Device: RX: {:02x?} busy={} wel={}",
            rx_buf, spi_device::get().is_busy_set(), spi_device::get().is_write_enable_set())?;

        match processor.process_spi_packet(rx_buf) {
            Ok(()) => {}
            Err(why) => {
                writeln!(console, "Device: Error processing SPI packet: {:?}", why)?;
                if spi_device::get().is_busy_set() {
                    spi_device::get().clear_status(true, false)?;
                }
            }
        }
    }
}

#[libtock::main]
async fn main() -> TockResult<()> {
    let mut console = Console::new();
    writeln!(console, "Starting ...")?;
    let result = run();
    if result.is_ok() {
        writeln!(console, "Returned OK.")?;
    } else {
        writeln!(console, "Returned error.")?;
    }
    result
}
