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

mod alarm;
mod console_processor;
mod console_reader;
mod firmware_controller;
mod flash;
mod fuse;
mod globalsec;
mod gpio;
mod gpio_processor;
mod manticore_support;
mod reset;
mod sfdp;
mod spi_host;
mod spi_host_h1;
mod spi_host_helper;
mod spi_device;
mod spi_processor;

use crate::console_processor::ConsoleProcessor;
use crate::gpio_processor::GpioProcessor;
use crate::spi_host_helper::SpiHostHelper;
use crate::spi_processor::SpiProcessor;

use core::fmt::Write;

use libtock::console::Console;
use libtock::result::TockError;
use libtock::result::TockResult;
use libtock::syscalls::raw::yieldk;

use spiutils::driver::spi_device::AddressConfig;
use spiutils::driver::spi_device::HandlerMode;
use spiutils::protocol::flash::AddressMode;

//////////////////////////////////////////////////////////////////////////////

fn run_host_helper_demo() -> TockResult<()> {
    // We cannot use the SPI host if passthrough is enabled.
    spi_host_h1::get().set_passthrough(false)?;

    let host_helper = SpiHostHelper {};
    host_helper.enter_4b()?;

    host_helper.read_and_print_data(0x0)?;

    if spi_device::get().get_address_mode() == AddressMode::ThreeByte {
        host_helper.exit_4b()?;
    }

    Ok(())
}

fn run() -> TockResult<()> {
    use core::cmp::min;

    let mut console = Console::new();

    //////////////////////////////////////////////////////////////////////////////

    run_host_helper_demo()?;

    //////////////////////////////////////////////////////////////////////////////

    let mut identity = manticore_support::Identity {
        version: [0; 32],
        device_id: [0; 64],
    };

    let banner_bytes = "v1.00".as_bytes();
    let max_len = min(identity.version.len(), banner_bytes.len());
    identity.version[..max_len].copy_from_slice(&banner_bytes[..max_len]);

    let dev_id_bytes = fuse::get().get_dev_id()?.to_be_bytes();
    let max_len = min(identity.device_id.len(), dev_id_bytes.len());
    identity.device_id[..max_len].copy_from_slice(&dev_id_bytes[..max_len]);

    let mut spi_processor = SpiProcessor {
        server: manticore_support::get_pa_rot(&identity),
        print_flash_headers: false,  // Enable to print incoming SPI flash headers
        firmware: firmware_controller::FirmwareController::new(),
    };

    let gpio_processor = GpioProcessor::new();
    let console_processor = ConsoleProcessor::new(&gpio_processor);


    //////////////////////////////////////////////////////////////////////////////

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

    // We assume that we've already done all boot-time checks at this point.

    // Deassert BMC resets.
    // TODO(osk): Do something with the result codes.
    let _ = gpio_processor.set_bmc_cpu_rst(false);
    let _ = gpio_processor.set_bmc_srst(false);

    //////////////////////////////////////////////////////////////////////////////

    console_reader::get().allow_read(1)?;

    loop {
        while !spi_device::get().have_transaction()
            && !console_reader::get().have_data()
            && !gpio::get().have_events()
            && !alarm::get().is_expired() {

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
                    let _ = writeln!(console, "SPI processor: Error {:?}", why);
                    if spi_device::get().is_busy_set() {
                        if let Err(_) = spi_device::get().end_transaction_with_status(true, false) {
                            // Ignore error from writeln. There's nothing we can do here anyway.
                            let _ = writeln!(console, "SPI device: end_transaction error.");
                        }
                    } else {
                        spi_device::get().end_transaction();
                    }
                }
            }
        }

        if console_reader::get().have_data() {
            match console_processor.process_input() {
                Ok(()) => {}
                Err(_) => {
                    // Ignore error from writeln. There's nothing we can do here anyway.
                    let _ = writeln!(console, "Console processor: Error.");
                }
            }
            console_reader::get().allow_read(1)?;
        }

        if gpio::get().have_events() {
            match gpio_processor.process_gpio_events() {
                Ok(()) => {}
                Err(_) => {
                    // Ignore error from writeln. There's nothing we can do here anyway.
                    let _ = writeln!(console, "GPIO processor (event): Error.");
                }
            }
        }

        if alarm::get().is_expired() {
            match gpio_processor.alarm_expired() {
                Ok(()) => {}
                Err(_) => {
                    // Ignore error from writeln. There's nothing we can do here anyway.
                    let _ = writeln!(console, "GPIO processor (alarm): Error.");
                }
            }
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
    writeln!(console, "Reset source: {:?}", reset::get().get_reset_source()?)?;
    writeln!(console, "active RO: {:?}, {:?}", globalsec::get().get_active_ro(), firmware_controller::get_build_info(globalsec::get().get_active_ro())?)?;
    writeln!(console, "active RW: {:?}, {:?}", globalsec::get().get_active_rw(), firmware_controller::get_build_info(globalsec::get().get_active_rw())?)?;
    writeln!(console, "inactive RO: {:?}, {:?}", globalsec::get().get_inactive_ro(), firmware_controller::get_build_info(globalsec::get().get_inactive_ro())?)?;
    writeln!(console, "inactive RW: {:?}, {:?}", globalsec::get().get_inactive_rw(), firmware_controller::get_build_info(globalsec::get().get_inactive_rw())?)?;
    writeln!(console, "DEV ID: 0x{:x}", fuse::get().get_dev_id()?)?;
    writeln!(console, "clock_frequency: {}", alarm::get().get_clock_frequency())?;
    let result = run();
    if result.is_ok() {
        writeln!(console, "main: returning OK.")?;
    } else {
        writeln!(console, "main: returning error.")?;
    }
    result
}
