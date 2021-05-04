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

use crate::hil::globalsec::GlobalSec;

use kernel::common::registers::register_bitfields;
use kernel::common::registers::register_structs;
use kernel::common::registers::ReadWrite;
use kernel::common::StaticRef;

use spiutils::driver::firmware::RuntimeSegmentInfo;
use spiutils::driver::firmware::SegmentInfo;
use spiutils::driver::firmware::UNKNOWN_RUNTIME_SEGMENT_INFO;

// Registers for the Fuse controller
register_structs! {
    Registers {
        (0x0000 => cpu0_d_region0_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x0004 => cpu0_d_region1_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x0008 => cpu0_d_region2_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x000c => cpu0_d_region3_ctrl: ReadWrite<u32, REGION_CTRL::Register>),

        (0x0010 => _reserved0010),

        (0x0080 => ddma0_region0_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x0084 => ddma0_region1_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x0088 => ddma0_region2_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x008c => ddma0_region3_ctrl: ReadWrite<u32, REGION_CTRL::Register>),

        (0x0090 => _reserved0090),

        (0x00c0 => dusb0_region0_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x00c4 => dusb0_region1_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x00c8 => dusb0_region2_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x00cc => dusb0_region3_ctrl: ReadWrite<u32, REGION_CTRL::Register>),

        (0x00d0 => _reserved00d0),

        (0x00e0 => flash_region0_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x00e4 => flash_region1_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x00e8 => flash_region2_ctrl: ReadWrite<u32, REGION_CTRL::Register>),
        (0x00ec => flash_region3_ctrl: ReadWrite<u32, REGION_CTRL::Register>),

        (0x00f0 => _reserved00f0),

        (0x0230 => flash_region0_base_addr: ReadWrite<u32>),
        (0x0234 => flash_region0_size: ReadWrite<u32>),
        (0x0238 => flash_region1_base_addr: ReadWrite<u32>),
        (0x023c => flash_region1_size: ReadWrite<u32>),
        (0x0240 => flash_region2_base_addr: ReadWrite<u32>),
        (0x0244 => flash_region2_size: ReadWrite<u32>),
        (0x0248 => flash_region3_base_addr: ReadWrite<u32>),
        (0x024c => flash_region3_size: ReadWrite<u32>),

        (0x0250 => @END),
    }
}

register_bitfields![u32,
    REGION_CTRL [
        /// Enable for region. 0 means region accepts no transactions
        EN OFFSET(0) NUMBITS(1) [],
        /// Read enable for region. 1 means region accepts read transaction if EN is 1
        RD_EN OFFSET(1) NUMBITS(1) [],
        /// Write enable for region. 1 means region accepts write transaction if EN is 1
        WR_EN OFFSET(2) NUMBITS(1) []
    ]
];

const GLOBALSEC_BASE_ADDR: u32 = 0x4009_0000;
const GLOBALSEC_REGISTERS: StaticRef<Registers> =
    unsafe { StaticRef::new(GLOBALSEC_BASE_ADDR as *const Registers) };

pub static mut GLOBALSEC: GlobalSecHardware = GlobalSecHardware::new(GLOBALSEC_REGISTERS);

pub struct Segments {
    pub ro_a: SegmentInfo,
    pub ro_b: SegmentInfo,
    pub rw_a: SegmentInfo,
    pub rw_b: SegmentInfo,
}

/// GlobalSec
pub struct GlobalSecHardware {
    registers: StaticRef<Registers>,
    runtime_segment_info: RuntimeSegmentInfo,
}

impl GlobalSecHardware {
    const fn new(base_addr: StaticRef<Registers>) -> GlobalSecHardware {
        GlobalSecHardware {
            registers: base_addr,
            runtime_segment_info: UNKNOWN_RUNTIME_SEGMENT_INFO,
        }
    }

    pub fn init(&mut self, segments: Segments) {
        self.registers.cpu0_d_region0_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);
        self.registers.cpu0_d_region1_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);
        self.registers.cpu0_d_region2_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);
        self.registers.cpu0_d_region3_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);

        self.registers.ddma0_region0_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);
        self.registers.ddma0_region1_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);
        self.registers.ddma0_region2_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);
        self.registers.ddma0_region3_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);

        self.registers.dusb0_region0_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);
        self.registers.dusb0_region1_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);
        self.registers.dusb0_region2_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);
        self.registers.dusb0_region3_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);

        // Flash regions:
        // - REGION0 : Active RO image, already locked
        // - REGION1 : Active RW image, already locked
        // - REGION2 : inactive RO image
        // - REGION3 : inactive RW image

        const H1_FLASH_START: u32 = crate::hil::flash::h1_hw::H1_FLASH_START as u32;

        // Determine the inactive RO.
        match self.registers.flash_region0_base_addr.get() {
            addr if addr == H1_FLASH_START + segments.ro_a.address => {
                self.runtime_segment_info.active_ro = segments.ro_a;
                self.runtime_segment_info.inactive_ro = segments.ro_b;
            },
            addr if addr == H1_FLASH_START + segments.ro_b.address => {
                self.runtime_segment_info.active_ro = segments.ro_b;
                self.runtime_segment_info.inactive_ro = segments.ro_a;
            },
            _ => println!("Tock: Unknown flash_region0_base")
        }
        // Enable the inactive RO for reads and writes.
        self.registers.flash_region2_base_addr.set(
            H1_FLASH_START + self.runtime_segment_info.inactive_ro.address);
        self.registers.flash_region2_size.set(self.runtime_segment_info.inactive_ro.size);
        self.registers.flash_region2_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);

        // Determine the inactive RW.
        match self.registers.flash_region1_base_addr.get() {
            addr if addr == H1_FLASH_START + segments.rw_a.address => {
                self.runtime_segment_info.active_rw = segments.rw_a;
                self.runtime_segment_info.inactive_rw = segments.rw_b;
            },
            addr if addr == H1_FLASH_START + segments.rw_b.address => {
                self.runtime_segment_info.active_rw = segments.rw_b;
                self.runtime_segment_info.inactive_rw = segments.rw_a;
            }
            _ => println!("Tock: Unknown flash_region1_base")
        }
        // Enable the inactive RW for reads and writes.
        self.registers.flash_region3_base_addr.set(
            H1_FLASH_START + self.runtime_segment_info.inactive_rw.address);
        self.registers.flash_region3_size.set(self.runtime_segment_info.inactive_rw.size);
        self.registers.flash_region3_ctrl.write(
            REGION_CTRL::EN::SET +
            REGION_CTRL::RD_EN::SET +
            REGION_CTRL::WR_EN::SET);
    }
}

impl GlobalSec for GlobalSecHardware {
    fn get_runtime_segment_info(&self) -> RuntimeSegmentInfo {
        self.runtime_segment_info
    }
}
