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

mod console_reader;
mod fuse;
mod manticore_support;
mod sfdp;
mod spi_host;
mod spi_host_h1;
mod spi_host_helper;
mod spi_device;
mod spi_processor;

use crate::spi_host_helper::SpiHostHelper;
use crate::spi_processor::SpiProcessor;

use core::fmt::Write;

use libtock::console::Console;
use libtock::result::TockError;
use libtock::result::TockResult;
use libtock::syscalls::raw::yieldk;

use spiutils::driver::AddressConfig;
use spiutils::driver::HandlerMode;
use spiutils::protocol::flash::AddressMode;

//////////////////////////////////////////////////////////////////////////////

fn run_host_helper_demo() -> TockResult<()> {
    let mut console = Console::new();

    // We cannot use the SPI host if passthrough is enabled.
    spi_host_h1::get().set_passthrough(false)?;

    let host_helper = SpiHostHelper {};

    writeln!(console, "Host: Entering 4B mode")?;
    host_helper.enter_4b()?;

    writeln!(console, "Host: Reading data")?;
    host_helper.read_and_print_data(0x0)?;
    host_helper.read_and_print_data(0x1)?;

    if spi_device::get().get_address_mode() == AddressMode::ThreeByte {
        writeln!(console, "Host: Exiting 4B mode")?;
        host_helper.exit_4b()?;
    }

    Ok(())
}

fn run() -> TockResult<()> {
    let mut console = Console::new();

    //////////////////////////////////////////////////////////////////////////////

    run_host_helper_demo()?;

    //////////////////////////////////////////////////////////////////////////////

    let mut identity = manticore_support::Identity {
        version: [0; 32],
        device_id: [0; 64],
    };

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

    let mut spi_processor = SpiProcessor {
        server: manticore_support::get_pa_rot(&identity),
        print_flash_headers: false,  // Enable to print incoming SPI flash headers
    };

    //////////////////////////////////////////////////////////////////////////////

    writeln!(console, "Device: Configuring address_mode handling to KernelSpace")?;
    spi_device::get().set_address_mode_handling(HandlerMode::KernelSpace)?;
    spi_device::get().configure_addresses(AddressConfig {
        flash_virtual_base: 0x0,
        flash_physical_base: 0x0,
        flash_physical_size: spi_processor::SPI_FLASH_SIZE,
        ram_virtual_base: spi_processor::SPI_MAILBOX_ADDRESS,
        virtual_size: spi_processor::SPI_FLASH_SIZE,
    })?;

    //////////////////////////////////////////////////////////////////////////////

    // OpenTitan JEDEC ID
    spi_device::get().set_jedec_id(&mut [
        0x26, // Manufacturer (Visic, should actually be
              // 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x26)
        0x31, // Device (OpenTitan)
        0x19, // Size (2^25 = 256 Mb)
        ])?;

    //////////////////////////////////////////////////////////////////////////////

    {
        let mut sfdp = [0xff; 128];
        sfdp::get_table(
            &mut sfdp,
            spi_processor::SPI_FLASH_SIZE * 8, // image_size_bits
            spi_device::get().get_address_mode(), // startup_address_mode
            spi_device::get().get_address_mode() == AddressMode::ThreeByte, // support_address_mode_switch
            spi_processor::SPI_MAILBOX_ADDRESS, // mailbox_offset
            spi_device::MAX_READ_BUFFER_SIZE as u32, // mailbox_size
            0 // google_capabilities
            ).map_err(|_| TockError::Format)?;
        spi_device::get().set_sfdp(&mut sfdp)?;
    }

    //////////////////////////////////////////////////////////////////////////////

    // We need SPI passthrough to be fully operational.
    spi_host_h1::get().set_passthrough(true)?;

    //////////////////////////////////////////////////////////////////////////////

    loop {
        while !spi_device::get().have_transaction()
            && !console_reader::get().have_data() {

            // Note: Do NOT use the console here, as that results in a "hidden"
            // yieldk() which causes us to lose track of the conditions above.
            unsafe { yieldk(); }
        }

        if spi_device::get().have_transaction() {
            let rx_buf = spi_device::get().get_read_buffer();
            match spi_processor.process_spi_packet(rx_buf) {
                Ok(()) => {}
                Err(why) => {
                    // Ignore error from writeln. There's nothing we can do here anyway.
                    let _ = writeln!(console, "Device: Error processing SPI packet: {:?}", why);
                    if spi_device::get().is_busy_set() {
                        if let Err(_) = spi_device::get().end_transaction_with_status(true, false) {
                            // Ignore error from writeln. There's nothing we can do here anyway.
                            let _ = writeln!(console, "Device: Error ending transaction.");
                        }
                    } else {
                        spi_device::get().end_transaction();
                    }
                }
            }
        }

        if console_reader::get().have_data() {
            let data = console_reader::get().get_data();
            writeln!(console, "Have data (len={}): 0x{:x}", data.len(), data[0])?;
            console_reader::get().allow_read(1)?;
        }
    }
}

const BANNER: &'static str = concat!(
    env!("CARGO_PKG_NAME"), ' ',
    env!("CARGO_PKG_VERSION"), ' ',
    include_str!("../../../build/gitlongtag")
);

#[libtock::main]
async fn main() -> TockResult<()> {
    let mut console = Console::new();
    writeln!(console, "Starting {}", BANNER)?;
    writeln!(console, "DEV ID: 0x{:x}", fuse::get().get_dev_id()?)?;
    let result = run();
    if result.is_ok() {
        writeln!(console, "Returned OK.")?;
    } else {
        writeln!(console, "Returned error.")?;
    }
    result
}
