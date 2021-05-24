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

use crate::flash;


use libtock::println;
use libtock::result::TockError;
use libtock::result::TockResult;

use spiutils::compat::firmware::BUILD_INFO_LEN;
use spiutils::compat::firmware::BUILD_INFO_OFFSET;
use spiutils::compat::firmware::BuildInfo;
use spiutils::driver::firmware::SegmentInfo;
use spiutils::driver::firmware::UNKNOWN_SEGMENT;
use spiutils::protocol::wire::FromWire;

#[derive(Copy, Clone, Debug)]
pub enum FirmwareControllerError {
    Tock,
    FlashReadError,
    FlashWriteError,
    FlashOperationFailed,
    Format(core::fmt::Error),
}

impl From<TockError> for FirmwareControllerError {
    fn from(_err: TockError) -> Self {
        FirmwareControllerError::Tock
    }
}

impl From<core::fmt::Error> for FirmwareControllerError {
    fn from(err: core::fmt::Error) -> Self {
        FirmwareControllerError::Format(err)
    }
}

//////////////////////////////////////////////////////////////////////////////

static mut WRITE_BUF : [u8; flash::MAX_BUFFER_LENGTH] = [0u8; flash::MAX_BUFFER_LENGTH];

pub struct FirmwareController {
    erase_segment: SegmentInfo,
    erase_page: usize,

    write_segment: SegmentInfo,
    write_offset: usize,
    write_length: usize,
}

pub type FirmwareControllerResult<T> = Result<T, FirmwareControllerError>;

impl FirmwareController {

    pub fn new() -> FirmwareController {
        FirmwareController {
            erase_segment: UNKNOWN_SEGMENT,
            erase_page: 0,
            write_segment: UNKNOWN_SEGMENT,
            write_offset: 0,
            write_length: 0,
        }
    }

    fn check_operation_result(&self) -> FirmwareControllerResult<()> {
        let flash_op_result = flash::get().get_operation_result();
        flash::get().clear_operation();
        if flash_op_result < 0 {
            println!("flash operation error {}", flash_op_result);
            return Err(FirmwareControllerError::FlashOperationFailed);
        }

        Ok(())
    }

    fn erase_segment_start(&mut self, segment: SegmentInfo) -> FirmwareControllerResult<()> {
        self.erase_segment = segment;
        self.erase_page = self.erase_segment.start_page as usize;
        flash::get().erase(self.erase_page)?;

        Ok(())
    }

    fn erase_segment_continue(&mut self) -> FirmwareControllerResult<bool> {
        if self.erase_page >= (self.erase_segment.start_page + self.erase_segment.page_count - 1) as usize {
            // We're done.
            return Ok(false);
        }

        self.erase_page += 1;
        flash::get().erase(self.erase_page)?;
        Ok(true)
    }

    fn get_write_flash_offset(&self) -> usize {
        self.write_segment.address as usize + self.write_offset
    }

    fn write_segment_chunk(&mut self, segment: SegmentInfo, offset: usize, data: &[u8]) -> FirmwareControllerResult<()> {
        // Copy write data into local buffer so we can do the compare later.
        self.write_segment = segment;
        self.write_offset = offset;
        self.write_length = data.len();
        for idx in 0..self.write_length {
            let val = data[idx];
            unsafe {
                // TODO(osk): We need the unsafe block since we're accessing WRITE_BUF as &mut.
                WRITE_BUF[idx] = val;
            }
        }

        // Write data
        unsafe {
            // TODO(osk): We need the unsafe block since we're accessing WRITE_BUF as &mut.
            if flash::get().write(self.get_write_flash_offset(), &mut WRITE_BUF, self.write_length).is_err() {
                println!("flash write failed");
                return Err(FirmwareControllerError::FlashWriteError);
            }
        }

        Ok(())
    }

    fn verify_segment_chunk(&self) -> FirmwareControllerResult<bool> {
        // Read data back
        let mut read_buf = [0u8; flash::MAX_BUFFER_LENGTH];
        if flash::get().read(self.get_write_flash_offset(), &mut read_buf, self.write_length).is_err() {
            println!("flash read failed");
            return Err(FirmwareControllerError::FlashReadError);
        }

        // Compare data
        for idx in 0..self.write_length {
            let write_val;
            unsafe {
                write_val = WRITE_BUF[idx];
            }
            if read_buf[idx] != write_val {
                println!("flash compare failed");
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn erase_segment(&mut self, segment: SegmentInfo) -> FirmwareControllerResult<()> {
        self.erase_segment_start(segment)?;
        flash::get().wait_operation_done();
        self.check_operation_result()?;
        while self.erase_segment_continue()? {
            flash::get().wait_operation_done();
            self.check_operation_result()?;
        }
        Ok(())
    }

    pub fn write_and_verify_segment_chunk(&mut self, segment: SegmentInfo, offset: usize, data: &[u8]) -> FirmwareControllerResult<bool> {
        self.write_segment_chunk(segment, offset, data)?;
        flash::get().wait_operation_done();
        self.check_operation_result()?;
        self.verify_segment_chunk()
    }

    pub fn get_max_write_chunk_length(&self) -> usize {
        flash::MAX_BUFFER_LENGTH
    }
}

pub fn get_build_info(segment: SegmentInfo) -> TockResult<BuildInfo> {
    let mut buf = [0u8; BUILD_INFO_LEN];
    flash::get().read(segment.address as usize + BUILD_INFO_OFFSET, &mut buf, BUILD_INFO_LEN)?;

    let maybe_build_info = BuildInfo::from_wire(buf.as_ref());
    if maybe_build_info.is_err() {
        return Err(TockError::Format);
    }

    Ok(maybe_build_info.unwrap())
}
