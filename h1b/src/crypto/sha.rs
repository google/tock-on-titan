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

use core::cell::Cell;
use core::mem;
use hil::digest::{DigestEngine, DigestMode, DigestError};
use kernel::common::cells::VolatileCell;
use super::keymgr::{KEYMGR0_REGS, Registers};

#[allow(unused)]
enum ShaTrigMask {
    Go = 0x1,
    Reset = 0x2,
    Step = 0x4,
    Stop = 0x8,
}

#[allow(unused)]
enum ShaCfgEnMask {
    BigEndian = 0x01,
    Sha1 = 0x02,

    BusError = 0x08,
    Livestream = 0x10,
    Hmac = 0x20,

    IntEnDone = 0x1_0000,
    IntMaskDone = 0x2_0000,
}

pub struct ShaEngine {
    regs: *mut Registers,
    current_mode: Cell<Option<DigestMode>>,
}

impl ShaEngine {
    const unsafe fn new(regs: *mut Registers) -> ShaEngine {
        ShaEngine {
            regs: regs,
            current_mode: Cell::new(None),
        }
    }
}

pub static mut KEYMGR0_SHA: ShaEngine = unsafe { ShaEngine::new(KEYMGR0_REGS) };

const HMAC_KEY_SIZE_BYTES: usize = 32;
const HMAC_KEY_SIZE_WORDS: usize = HMAC_KEY_SIZE_BYTES / 4;

impl DigestEngine for ShaEngine {
    fn initialize(&self, mode: DigestMode) -> Result<(), DigestError> {
        let ref regs = unsafe { &*self.regs }.sha;

        regs.itop.set(0); // clear status

        // Compile-time check for DigestMode exhaustiveness
        match mode {
            DigestMode::Sha1 |
            DigestMode::Sha256 |
            DigestMode::Sha256Hmac => (),
        };
        self.current_mode.set(Some(mode));

        regs.trig.set(ShaTrigMask::Stop as u32);

        let mut flags = ShaCfgEnMask::Livestream as u32 | ShaCfgEnMask::IntEnDone as u32;
        match mode {
            DigestMode::Sha1 => flags |= ShaCfgEnMask::Sha1 as u32,
            DigestMode::Sha256 => (),
            DigestMode::Sha256Hmac => flags |= ShaCfgEnMask::Hmac as u32,
        }
        regs.cfg_en.set(flags);

        regs.trig.set(ShaTrigMask::Go as u32);

        Ok(())
    }

    fn  initialize_hmac(&self, key: &[u8]) -> Result<(), DigestError> {
        let ref regs = unsafe { &*self.regs }.sha;

        regs.itop.set(0); // clear status
        self.current_mode.set(Some(DigestMode::Sha256Hmac));

        if key.len() < HMAC_KEY_SIZE_BYTES {
            print!("Key too small: {}\n", key.len());
            return Err(DigestError::BufferTooSmall(HMAC_KEY_SIZE_BYTES));
        }
        for i in 0..HMAC_KEY_SIZE_WORDS {
            let word: u32 = (key[4 * i + 0] as u32) << 0  |
                            (key[4 * i + 1] as u32) << 8  |
                            (key[4 * i + 2] as u32) << 16 |
                            (key[4 * i + 3] as u32) << 24;
            regs.key_w[i].set(word);
        }

        let flags = ShaCfgEnMask::Livestream as u32 |
                    ShaCfgEnMask::IntEnDone as u32 |
                    ShaCfgEnMask::Hmac as u32;

        regs.cfg_en.set(flags);
        regs.trig.set(ShaTrigMask::Go as u32);

        return Ok(());
    }

    fn update(&self, data: &[u8]) -> Result<usize, DigestError> {
        let ref regs = unsafe { &*self.regs }.sha;

        if self.current_mode.get().is_none() {
            print!("ERROR: SHA::update called but engine not initialized!\n");
            return Err(DigestError::NotConfigured);
        }

        let fifo_u8: &VolatileCell<u8> = unsafe { mem::transmute(&regs.input_fifo) };

        // TODO(yuriks): Feed FIFO word at a time when possible
        for b in data {
            fifo_u8.set(*b);
        }
        Ok(data.len())
    }

    fn finalize(&self, output: &mut [u8]) -> Result<usize, DigestError> {
        let ref regs = unsafe { &*self.regs }.sha;

        let expected_output_size = match self.current_mode.get() {
            None => return Err(DigestError::NotConfigured),
            Some(mode) => mode.output_size(),
        };
        if output.len() < expected_output_size {
            return Err(DigestError::BufferTooSmall(expected_output_size));
        }

        // Tell hardware we're done streaming and then wait for the hash calculation to finish.
        regs.itop.set(0);
        regs.trig.set(ShaTrigMask::Stop as u32);
        while regs.itop.get() == 0 {}

        for i in 0..(expected_output_size / 4) {
            let word = regs.sts_h[i].get();
            output[i * 4 + 0] = (word >> 0) as u8;
            output[i * 4 + 1] = (word >> 8) as u8;
            output[i * 4 + 2] = (word >> 16) as u8;
            output[i * 4 + 3] = (word >> 24) as u8;
        }

        regs.itop.set(0);

        Ok(expected_output_size)
    }
}
