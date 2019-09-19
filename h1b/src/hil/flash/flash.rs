// Copyright 2019 Google LLC
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

use ::kernel::ReturnCode;

/// Flash client -- receives callbacks when flash operations complete.
pub trait Client<'d> {
        fn erase_done(&self, ReturnCode);
        fn write_done(&self, data: &'d mut [u32], ReturnCode);
}

/// Flash driver API.
pub trait Flash<'d> {
        /// Erases the specified flash page, setting it to all ones.
        fn erase(&self, page: usize) -> ReturnCode;

        /// Reads the given word from flash. Successful read returns
        /// ReturnCode::SuccessWithValue with the value read; if the
        /// offset is out of bounds, returns ReturnCode::ESIZE.
        fn read(&self, offset: usize) -> ReturnCode;

        /// Writes a buffer (of up to 32 words) into the given location in flash.
        /// The target location is specified as an offset from the beginning of
        /// flash in units of words.
        fn write(&self, target: usize, data: &'d mut [u32]) -> (ReturnCode, Option<&'d mut [u32]>);

        /// Links this driver to its client.
        fn set_client(&'d self, client: &'d Client<'d>);
}
