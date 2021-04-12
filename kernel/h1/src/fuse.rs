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

use crate::hil::fuse::Fuse;

use kernel::common::registers::register_structs;
use kernel::common::registers::ReadOnly;
use kernel::common::StaticRef;

// Registers for the Fuse controller
register_structs! {
    Registers {
        (0x0000 => _reserved0000),

        (0x0044 => dev_id0: ReadOnly<u32>),
        (0x0048 => dev_id1: ReadOnly<u32>),

        (0x004c => _reserved004c),
        (0x0448 => @END),
    }
}

const FUSE_BASE_ADDR: u32 = 0x4045_0000;
const FUSE_REGISTERS: StaticRef<Registers> =
    unsafe { StaticRef::new(FUSE_BASE_ADDR as *const Registers) };

pub static mut FUSE: FuseController = FuseController::new(FUSE_REGISTERS);

/// Fuse Controller
pub struct FuseController {
    registers: StaticRef<Registers>,
}

impl FuseController {
    const fn new(base_addr: StaticRef<Registers>) -> FuseController {
        FuseController {
            registers: base_addr,
        }
    }
}

impl Fuse for FuseController {
    fn get_dev_id(&self) -> u64 {
        ((self.registers.dev_id0.get() as u64) << 32)
            | (self.registers.dev_id1.get() as u64)
    }
}
