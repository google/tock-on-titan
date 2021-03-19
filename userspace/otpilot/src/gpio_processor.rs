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

use crate::alarm;
use crate::gpio;
use crate::gpio::GpioPin;
use crate::gpio::GpioValue;
use crate::spi_device;
use crate::spi_host_h1;
use crate::spi_host_helper::SpiHostHelper;

use core::cell::Cell;
use core::fmt::Write;

use libtock::console::Console;
use libtock::result::TockResult;

use spiutils::protocol::flash::AddressMode;

pub struct GpioProcessor {
    /// Whether to ignore bmc_rstmon_n events
    ignore_bmc_rstmon_n_events: Cell<bool>,

    /// The initial address mode after resetting the BMC.
    initial_address_mode: AddressMode,

    /// Alarm ticks
    alarm_ticks: usize,
}

const ALARM_MSECS: u64 = 62;
const MSECS_IN_SEC: u64 = 1000;

impl GpioProcessor {
    pub fn new() -> GpioProcessor {
        let alarm_ticks: u64 =
            ((alarm::get().get_clock_frequency() as u64) * ALARM_MSECS) / MSECS_IN_SEC;

        GpioProcessor {
            ignore_bmc_rstmon_n_events: Cell::new(false),
            initial_address_mode: spi_device::get().get_address_mode(),
            alarm_ticks: alarm_ticks as usize,
        }
    }

    fn set_alarm(&self) -> TockResult<()> {
        self.ignore_bmc_rstmon_n_events.set(true);
        alarm::get().set(self.alarm_ticks)
    }

    pub fn set_bmc_cpu_rst(&self, asserted: bool) -> TockResult<()> {
        if asserted {
            gpio::get().set(GpioPin::BMC_CPU_RST_N, GpioValue::Low)?;
        } else  {
            gpio::get().set(GpioPin::BMC_CPU_RST_N, GpioValue::High)?;
            self.set_alarm()?;
        }

        Ok(())
    }

    pub fn set_bmc_srst(&self, asserted: bool) -> TockResult<()> {
        if asserted {
            gpio::get().set(GpioPin::BMC_SRST_N, GpioValue::Low)?;
        } else  {
            gpio::get().set(GpioPin::BMC_SRST_N, GpioValue::High)?;
            self.set_alarm()?;
        }

        Ok(())
    }

    fn handle_bmc_rstmon(&self) -> TockResult<()> {
        // Put BMC into reset
        self.set_bmc_cpu_rst(true)?;

        // Disable SPI passthrough
        spi_host_h1::get().set_passthrough(false)?;

        // Read some stuff from the SPI host
        // TODO: Do something more useful with the data (e.g. checksum) here.
        let host_helper = SpiHostHelper {};
        host_helper.enter_4b()?;
        host_helper.read_and_print_data(0x0)?;

        // Set expected initial address mode
        let host_helper = SpiHostHelper {};
        match self.initial_address_mode {
            AddressMode::ThreeByte => host_helper.exit_4b()?,
            AddressMode::FourByte => host_helper.enter_4b()?,
        }
        spi_device::get().set_address_mode(self.initial_address_mode)?;

        // Enable SPI passthrough
        spi_host_h1::get().set_passthrough(true)?;

        // We don't care about any events that may have happened during reset.
        gpio::get().clear_event(GpioPin::BMC_RSTMON_N);

        // Let BMC out of reset
        self.set_bmc_cpu_rst(false)?;

        Ok(())
    }

    pub fn process_gpio_events(&self) -> TockResult<()> {
        let mut console = Console::new();

        let bmc_rstmon_n = gpio::get().consume_event(GpioPin::BMC_RSTMON_N);
        if bmc_rstmon_n {
            if self.ignore_bmc_rstmon_n_events.get() {
                writeln!(console, "Ignored bmc_rstmon_n")?;
            } else {
                writeln!(console, "Handling bmc_rstmon_n")?;
                self.handle_bmc_rstmon()?;
            }
        }

        Ok(())
    }

    pub fn alarm_expired(&self) -> TockResult<()> {
        let mut console = Console::new();

        writeln!(console, "GPIO: alarm expired")?;
        self.ignore_bmc_rstmon_n_events.set(false);
        alarm::get().clear()
    }
}
