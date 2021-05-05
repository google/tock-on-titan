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

use crate::console_reader;
use crate::gpio_processor::GpioProcessor;
use crate::reset;

use core::fmt::Write;

use libtock::console::Console;
use libtock::result::TockResult;

pub struct ConsoleProcessor<'a> {
    gpio_processor: &'a GpioProcessor,
}

impl<'a> ConsoleProcessor<'a> {
    pub fn new(gpio_processor: &'a GpioProcessor) -> ConsoleProcessor<'a> {
        ConsoleProcessor {
            gpio_processor: gpio_processor,
        }
    }

    fn print_help(&self) -> TockResult<()> {
        let mut console = Console::new();

        writeln!(console, "Available commands:")?;
        writeln!(console, "? : This help screen.")?;
        writeln!(console, "1 : Assert BMC_CPU_RST.")?;
        writeln!(console, "! : Deassert BMC_CPU_RST.")?;
        writeln!(console, "2 : Assert BMC_SRST.")?;
        writeln!(console, "@ : Deassert BMC_SRST.")?;
        writeln!(console, "R : Reset chip.")?;

        Ok(())
    }

    pub fn process_input(&self) -> TockResult<()> {
        let mut console = Console::new();

        let data = console_reader::get().get_data();
        if data.len() < 1 {
            return Ok(());
        }

        match data[0] as char {
            '?' => self.print_help()?,
            '1' => {
                writeln!(console, "Asserting BMC_CPU_RST")?;
                self.gpio_processor.set_bmc_cpu_rst(true)?;
            },
            '!' => {
                writeln!(console, "Deasserting BMC_CPU_RST")?;
                self.gpio_processor.set_bmc_cpu_rst(false)?;
            },
            '2' => {
                writeln!(console, "Asserting BMC_SRST")?;
                self.gpio_processor.set_bmc_srst(true)?;
            },
            '@' => {
                writeln!(console, "Deasserting BMC_SRST")?;
                self.gpio_processor.set_bmc_srst(false)?;
            },
            'R' => {
                writeln!(console, "resetting ...")?;
                reset::get().reset()?;
            }
            _ => (),
        }

        Ok(())
    }
}
