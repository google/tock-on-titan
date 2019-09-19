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

#![allow(unused)]

use kernel::ReturnCode;
use kernel::common::cells::VolatileCell;
use kernel::common::registers::ReadWrite;

// The hardware flash controller. Cannot be used in userspace (accessing will
// trigger a fault), and should only be manipulated by the flash hardware.
pub static mut H1B_HW: *const H1bHw = 0x40720000 as *const H1bHw;

pub const H1B_FLASH_START: usize     = 0x40000;
pub const H1B_FLASH_BANK_SIZE: usize = 0x40000;
pub const H1B_FLASH_SIZE: usize      = 0x80000; // Two banks
pub const H1B_FLASH_PAGE_SIZE: usize = 0x00800; // 2kB

pub const H1B_INFO_0_START: usize    = 0x20000;
pub const H1B_INFO_1_START: usize    = 0x28000;
pub const H1B_INFO_SIZE: usize       = 0x00800;

register_bitfields![u32,
    TransactionParameters [
        Offset OFFSET(0) NUMBITS(16) [],
        Bank   OFFSET(16) NUMBITS(1) [],
        Size   OFFSET(17) NUMBITS(5) []
    ]
];

#[repr(C)]
pub struct H1bHw {
    /// Read/Program/Erase control for flash macro 0.
    _pe_control_0: VolatileCell<u32>,

    /// Read/Program/Erase control for flash macro 1.
    pe_control_1: VolatileCell<u32>,

    /// Write size and offset.
    transaction_parameters: ReadWrite<u32, TransactionParameters::Register>,

    /// Read/erase/program lockdown modes for the various flash regions.
    _lockdown_triggers: VolatileCell<u32>,

    /// Triggers a read of info 0 data to check for secure data write functionality. Only available
    /// in test mode.
    _enable_info0_shadow_read: VolatileCell<u32>,

    /// Operation-completion interrupt controls.
    _interrupt_control: VolatileCell<u32>,

    /// Operation-completion state. Cleared on read.
    _interrupt_state: VolatileCell<u32>,

    /// Macro 0 override signal unlock.
    _override_0_unlock: VolatileCell<u32>,

    /// Macro 1 override signal unlock.
    _override_1_unlock: VolatileCell<u32>,

    /// DIN override.
    _din_override: VolatileCell<u32>,

    /// Offset override.
    _offset_override: VolatileCell<u32>,

    /// Signal override values.
    _override_signal_values: VolatileCell<u32>,

    /// Signal override controls.
    _override_signal_control: VolatileCell<u32>,

    /// Controls whether the system may powerdown without waiting for graceful brownout completion.
    disable_brownout_wait: VolatileCell<u32>,

    /// The most recent value of Macro 0's DOUT.
    dout_value_0: VolatileCell<u32>,

    /// TC override read value.
    _tc_override_value_0: VolatileCell<u32>,

    /// The most recent value of Macro 1's DOUT.
    dout_value_1: VolatileCell<u32>,

    /// TC override read value.
    _tc_override_value_1: VolatileCell<u32>,

    /// Write data buffer.
    write_data: [VolatileCell<u32>; 32],

    /// Program and erase enable.
    program_erase_enable: VolatileCell<u32>,

    /// Redundancy remapping enablement and control for macro 0.
    redundancy_remapping_0: VolatileCell<u32>,

    /// Redundancy remapping enablement and control for macro 1.
    redundancy_remapping_1: VolatileCell<u32>,

    /// Error signalling.
    error_code: VolatileCell<u32>,

    /// Read operation duration.
    read_cycles: VolatileCell<u32>,

    /// The first cycle XE should be asserted during reads.
    read_xe_first_cycle: VolatileCell<u32>,

    /// The last cycle XE should be asserted during reads.
    read_xe_last_cycle: VolatileCell<u32>,

    /// The first cycle YE should be asserted during reads.
    read_ye_first_cycle: VolatileCell<u32>,

    /// The last cycle YE should be asserted during reads.
    read_ye_last_cycle: VolatileCell<u32>,

    /// The first cycle SE should be asserted during reads.
    read_se_first_cycle: VolatileCell<u32>,

    /// The last cycle SE should be asserted during reads.
    read_se_last_cycle: VolatileCell<u32>,

    /// The first cycle PV should be asserted during program verify reads.
    read_pv_first_cycle: VolatileCell<u32>,

    /// The last cycle PV should be asserted during program verify reads.
    read_pv_last_cycle: VolatileCell<u32>,

    /// The first cycle EV should be asserted during erase verify reads.
    read_ev_first_cycle: VolatileCell<u32>,

    /// The last cycle EV should be asserted during erase verify reads.
    read_ev_last_cycle: VolatileCell<u32>,

    /// Smart program algorithm enablement.
    enable_smart_program: VolatileCell<u32>,

    /// Single-word program timing.
    program_cycles: VolatileCell<u32>,

    /// The first cycle XE should be asserted during program.
    program_xe_first_cycle: VolatileCell<u32>,

    /// The last cycle XE should be asserted during program.
    program_xe_last_cycle: VolatileCell<u32>,

    /// The first cycle YE should be asserted during program.
    program_ye_first_cycle: VolatileCell<u32>,

    /// The last cycle YE should be asserted during program.
    program_ye_last_cycle: VolatileCell<u32>,

    /// The first cycle DIN and YADR are ready during program.
    program_din_yadr_first_cycle: VolatileCell<u32>,

    /// The last cycle DIN and YADR are ready during program.
    program_din_yadr_last_cycle: VolatileCell<u32>,

    /// The first cycle PROG should be asserted during program.
    program_prog_first_cycle: VolatileCell<u32>,

    /// The last cycle PROG should be asserted during program.
    program_prog_last_cycle: VolatileCell<u32>,

    /// The first cycle NVSTR should be asserted during program.
    program_nvstr_first_cycle: VolatileCell<u32>,

    /// The last cycle NVSTR should be asserted during program.
    program_nvstr_last_cycle: VolatileCell<u32>,

    /// Smart erase algorithm enablement.
    enable_smart_erase: VolatileCell<u32>,

    /// Flash erase cycle count.
    erase_cycles: VolatileCell<u32>,

    /// The first cycle XE should be asserted during erase.
    erase_xe_first_cycle: VolatileCell<u32>,

    /// The last cycle XE should be asserted during erase.
    erase_xe_last_cycle: VolatileCell<u32>,

    /// The first cycle ERASE should be asserted during erase.
    erase_erase_first_cycle: VolatileCell<u32>,

    /// The last cycle ERASE should be asserted during erase.
    erase_erase_last_cycle: VolatileCell<u32>,

    /// The first cycle NVSTR should be asserted during erase.
    erase_nvstr_first_cycle: VolatileCell<u32>,

    /// The last cycle NVSTR should be asserted during erase.
    erase_nvstr_last_cycle: VolatileCell<u32>,

    /// Enable smart erase for bulk erases.
    enable_bulk_smart_erase: VolatileCell<u32>,

    /// Duration of bulk erase operations.
    bulk_erase_cycles: VolatileCell<u32>,

    /// The first cycle XE should be asserted during bulkerase.
    bulkerase_xe_first_cycle: VolatileCell<u32>,

    /// The last cycle XE should be asserted during bulkerase.
    bulkerase_xe_last_cycle: VolatileCell<u32>,

    /// The first cycle BULKERASE should be asserted during bulkerase.
    bulkerase_bulkerase_first_cycle: VolatileCell<u32>,

    /// The last cycle BULKERASE should be asserted during bulkerase.
    bulkerase_bulkerase_last_cycle: VolatileCell<u32>,

    /// The first cycle MAS1 should be asserted during bulkerase.
    bulkerase_mas1_first_cycle: VolatileCell<u32>,

    /// The last cycle MAS1 should be asserted during bulkerase.
    bulkerase_mas1_last_cycle: VolatileCell<u32>,

    /// The first cycle NVSTR should be asserted during bulkerase.
    bulkerase_nvstr_first_cycle: VolatileCell<u32>,

    /// The last cycle NVSTR should be asserted during bulkerase.
    bulkerase_nvstr_last_cycle: VolatileCell<u32>,

    /// Controller debug signals.
    debug_signals: VolatileCell<u32>,

    // The integration test controls are omitted because they are unnecessary
    // and are after a large address space gap.
}

impl super::hardware::Hardware for H1bHw {
    fn is_programming(&self) -> bool {
        // TODO(jrvanwhy): Only checks the second flash bank.
        self.pe_control_1.get() != 0
    }

    fn read(&self, offset: usize) -> ReturnCode {
        // The two flash macros are in consecutive memory locations, so they can
        // be addressed as one.
        if offset > H1B_FLASH_SIZE {
            ReturnCode::ESIZE
        } else {
            unsafe {
                ReturnCode::SuccessWithValue {
                    value: ::core::ptr::read_volatile((H1B_FLASH_START as *const u32).add(offset)) as usize
                }
            }
        }
    }

    fn read_error(&self) -> u16 {
        self.error_code.get() as u16
    }

    fn set_transaction(&self, offset: usize, size: usize) {
        use self::TransactionParameters::{Offset,Size};
        // The offset is relative to the beginning of the flash module. There
        // are 128 pages per flash module.
        // TODO(jrvanwhy): Assumes the read is from the second flash bank.
        if offset > H1B_FLASH_SIZE {
           return; // TODO(pal): Fails silently!
        }

        let offset = offset - 128 * super::WORDS_PER_PAGE;
        self.transaction_parameters.write(Offset.val(offset as u32) + Size.val(size as u32));
    }

    fn set_write_data(&self, data: &[u32]) {
        for (i, &v) in data.iter().enumerate() { self.write_data[i].set(v); }
    }

    fn trigger(&self, opcode: u32) {
        self.program_erase_enable.set(0xb11924e1);
        // TODO(jrvanwhy): Assumes the write is to the second flash bank (where
        // the nvmem counter is).
        self.pe_control_1.set(opcode);
    }
}
