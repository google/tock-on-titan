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

use crate::spi_host;

use core::fmt::Write;

use libtock::console::Console;
use libtock::result::TockResult;

pub struct SpiHostHelper;

impl SpiHostHelper {
    pub fn enter_4b(&self) -> TockResult<()> {
        spi_host::get().read_write_bytes(&mut [0xb7], 1)?;
        spi_host::get().wait_read_write_done();
        Ok(())
    }

    pub fn exit_4b(&self) -> TockResult<()> {
        spi_host::get().read_write_bytes(&mut [0xe9], 1)?;
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
