// Copyright 2018 Google LLC
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

use kernel::common::cells::VolatileCell;
use super::KEYMGR0_BASE_ADDRESS;

#[repr(C)]
pub struct Registers {
    pub aes: AesRegisters, // 0x0000 - 0x00C0
    int: IntRegisters, // 0x00C4 - 0x00CC

    _padding_00d0: [u8; 0x0400 - 0x00d0], // 0x00D0

    pub sha: ShaRegisters, // 0x0400 - 0x04b0

    _padding_04b4: [u8; 0x2100 - 0x04b4], // 0x04B4

    tm_pw: TmPwRegisters, // 0x2100 - 0x2120

    _padding_2124: [u8; 0x3000 - 0x2124], // 0x2124

    pub hkey: HkeyRegisters, // 0x3000 - 0x3330
}

#[repr(C)]
pub struct AesRegisters {
    pub ctrl: VolatileCell<u32>, // 0x0000

    _padding_004: [u8; 0x0008 - 0x0004], // 0x0004

    pub wfifo_data: VolatileCell<u32>, // 0x0008
    pub rfifo_data: VolatileCell<u32>, // 0x000c

    _padding_010: [u8; 0x002c - 0x0010], // 0x002c

    pub key: [VolatileCell<u32>; 8], // 0x002c
    pub key_start: VolatileCell<u32>, // 0x004c
    pub ctr: [VolatileCell<u32>; 4], // 0x0050
    pub rand_stall_ctl: VolatileCell<u32>, // 0x0060

    pub wfifo_level: VolatileCell<u32>, // 0x0064
    pub wfifo_full: VolatileCell<u32>, // 0x0068
    pub rfifo_level: VolatileCell<u32>, // 0x006c
    pub rfifo_empty: VolatileCell<u32>, // 0x0070

    pub execute_count_state: VolatileCell<u32>, // 0x0074
    pub execute_count_max: VolatileCell<u32>, // 0x0078

    pub gcm_do_acc: VolatileCell<u32>, // 0x007c
    pub gcm_h: [VolatileCell<u32>; 4], // 0x0080
    pub gcm_mac: [VolatileCell<u32>; 4], // 0x0090
    pub gcm_hash_in: [VolatileCell<u32>; 4], // 0x00a0

    pub wipe_secrets: VolatileCell<u32>, // 0x00b0
    pub int_enable: VolatileCell<u32>, // 0x000b4
    pub int_state: VolatileCell<u32>, // 0x000b8
    pub int_test: VolatileCell<u32>, // 0x000bc
    pub use_hidden_key: VolatileCell<u32>, // 0x000c0
}

#[repr(C)]
struct IntRegisters {
    enable: VolatileCell<u32>, // 0x00c4
    state: VolatileCell<u32>, // 0x00c8
    test: VolatileCell<u32>, // 0x00cc
}

#[repr(C)]
pub struct ShaRegisters {
    pub cfg_msglen_lo: VolatileCell<u32>, // 0x0400
    pub cfg_msglen_hi: VolatileCell<u32>, // 0x0404
    pub cfg_en: VolatileCell<u32>, // 0x0408
    pub cfg_wr_en: VolatileCell<u32>, // 0x040C
    pub trig: VolatileCell<u32>, // 0x0410

    _padding_414: [u8; 0x0440 - 0x0414], // 0x0414

    pub input_fifo: VolatileCell<u32>, // 0x0440
    pub sts_h: [VolatileCell<u32>; 8], // 0x0444
    pub key_w: [VolatileCell<u32>; 8], // 0x0464
    pub sts: VolatileCell<u32>, // 0x0484
    pub itcr: VolatileCell<u32>, // 0x0488
    pub itop: VolatileCell<u32>, // 0x048C
    pub use_hidden_key: VolatileCell<u32>, // 0x0490
    pub use_cert: VolatileCell<u32>, // 0x0494
    pub cert_override: VolatileCell<u32>, // 0x0498
    pub rand_stall_ctl: VolatileCell<u32>, // 0x049C
    pub execute_count_state: VolatileCell<u32>, // 0x04A0
    pub execute_count_max: VolatileCell<u32>, // 0x04A4
    pub cert_revoke_ctrl: [VolatileCell<u32>; 3], // 0x04A8
}

#[repr(C)]
struct TmPwRegisters {
    attempt: [VolatileCell<u32>; 8], // 0x2100
    unlock: VolatileCell<u32>, // 0x2120
}

#[repr(C)]
pub struct HkeyRegisters {
    pub rwr: [VolatileCell<u32>; 8], // 0x3000
    pub rwr_vld: VolatileCell<u32>, // 0x3020
    rwr_lock: VolatileCell<u32>, // 0x3024

    _padding_3028: [u8; 0x3100 - 0x3028], // 0x3028

    fwr: [VolatileCell<u32>; 8], // 0x3100
    fwr_vld: VolatileCell<u32>, // 0x3120
    fw_major_version: VolatileCell<u32>, // 0x3124
    fwr_lock: VolatileCell<u32>, // 0x3128

    _padding_312c: [u8; 0x3200 - 0x312c], // 0x312c

    hwr: [VolatileCell<u32>; 8], // 0x3200
    hwr_vld: VolatileCell<u32>, // 0x3220
    hwr_lock: VolatileCell<u32>, // 0x3224

    _padding_3228: [u8; 0x3300 - 0x3224], // 0x3224

    frr: [VolatileCell<u32>; 8], // 0x3300

    flash_rcv_wipe: VolatileCell<u32>, // 0x3320

    pub err_flags: VolatileCell<u32>, // 0x3324
    pub err_ctr: VolatileCell<u32>, // 0x3328

    flash_rcv_status: VolatileCell<u32>, // 0x332c
    testmode_unlocked_status: VolatileCell<u32>, // 0x3330
}

pub const KEYMGR0_REGS: *mut Registers = KEYMGR0_BASE_ADDRESS as *mut Registers;
