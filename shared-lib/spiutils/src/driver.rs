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

//! Kernel interface

use core::convert::TryFrom;
use core::default::Default;

/// Handler mode.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum HandlerMode {
    /// Do not handle request.
    Disabled = 0,

    /// Handle request in user space.
    UserSpace = 1,

    /// Handle request in kernel space.
    KernelSpace = 2,
}

impl Default for HandlerMode {
    fn default() -> Self { Self::Disabled }
}

/// Error for invalid handler mode conversion.
pub struct InvalidHandlerMode;

impl TryFrom<usize> for HandlerMode {
    type Error = InvalidHandlerMode;

    fn try_from(item: usize) -> Result<HandlerMode, Self::Error> {
        match item {
            0 => Ok(HandlerMode::Disabled),
            1 => Ok(HandlerMode::UserSpace),
            2 => Ok(HandlerMode::KernelSpace),
            _ => Err(InvalidHandlerMode),
        }
    }
}
