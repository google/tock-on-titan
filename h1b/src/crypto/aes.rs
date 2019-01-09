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

use kernel::hil::symmetric_encryption::{AES128, AES128Ctr, AES128CBC, Client};
use kernel::common::cells::OptionalCell;

use super::keymgr::{KEYMGR0_REGS, Registers};

#[derive(Debug, Copy, Clone)]
pub enum KeySize {
    /// Uses 128 bit AES key
    KeySize128 = 0x0,
    /// Uses 192 bit AES key
    KeySize192 = 0x2,
    /// Uses 256 bit AES key
    KeySize256 = 0x4,
}

#[derive(Debug, Copy, Clone)]
pub enum CipherMode {
    /// Electronic Codebook mode.
    Ecb = 0x0,
    /// Counter mode.
    Ctr = 0x8,
    /// Cypher Block Chaining mode.
    Cbc = 0x10,
    /// Galois/Counter mode.
    Gcm = 0x18,
}

#[derive(Debug, Copy, Clone)]
pub enum Mode {
    /// Input data should be decrypted.
    Decrypt = 0x0,
    /// Input data should be encrypted.
    Encrypt = 0x20,
}

#[derive(Debug, Copy, Clone)]
pub enum CtrEndian {
    /// Counter should be treated as big endian (matches NIST spec).
    Big = 0x0,
    /// Counter should be treated as little endian.
    Little = 0x40,
}

pub enum AesModule {
    Reset = 0x1,
    Enable = 0x80,
}

#[derive(Debug, Clone, Copy)]
pub enum Interrupt {
    WFIFOOverflow = 0,
    RFIFOOverflow,
    RFIFOUnderflow,
    DoneCipher,
    DoneKeyExpansion,
    DoneWipeSecrets,
}

pub enum ParsedInterrupt {
    Found(Interrupt),
    None,
}

impl From<u32> for ParsedInterrupt {
    fn from(interrupt: u32) -> Self {
        match interrupt {
            104 => ParsedInterrupt::Found(Interrupt::DoneCipher),
            105 => ParsedInterrupt::Found(Interrupt::DoneKeyExpansion),
            106 => ParsedInterrupt::Found(Interrupt::DoneWipeSecrets),
            107 => ParsedInterrupt::Found(Interrupt::RFIFOOverflow),
            108 => ParsedInterrupt::Found(Interrupt::RFIFOUnderflow),
            109 => ParsedInterrupt::Found(Interrupt::WFIFOOverflow),
            _ => ParsedInterrupt::None,
        }
    }
}

pub struct AesEngine<'a>{
    regs: *mut Registers,
    client: OptionalCell<&'a Client<'a>>,
}

impl<'a> AesEngine<'a> {
    const unsafe fn new(regs: *mut Registers) -> AesEngine<'a> {
        AesEngine {
            regs: regs,
            client: OptionalCell::empty(),
        }
    }

    pub fn set_client(&self, client: &'a Client<'a>) {
        self.client.set(client);
    }

    pub fn setup(&self, key_size: KeySize, key: &[u32; 8]) {
        let ref regs = unsafe { &*self.regs }.aes;

        self.enable_all_interrupts();
        regs.ctrl.set(regs.ctrl.get() | key_size as u32 | AesModule::Enable as u32);

        for (i, word) in key.iter().enumerate() {
            regs.key[i].set(*word);
        }
        regs.key_start.set(1);
    }

    pub fn set_encrypt_mode(&self, encrypt: bool) {
        let ref regs = unsafe { &*self.regs }.aes;

        let flag = Mode::Encrypt as u32;
        if encrypt {
            regs.ctrl.set(regs.ctrl.get() | flag);
        } else {
            regs.ctrl.set(regs.ctrl.get() & !flag);
        }
    }

    pub fn crypt(&self, input: &[u8]) -> usize {
        let ref regs = unsafe { &*self.regs }.aes;

        let mut written_bytes = 0;
        let mut written_words = 0;
        for word in input.chunks(4) {
            if regs.wfifo_full.get() != 0 || written_bytes >= 16 {
                break;
            }
            let d = word.iter()
                .map(|b| *b as u32)
                .enumerate()
                .fold(0, |accm, (i, byte)| accm | (byte << (i * 8)));
            regs.wfifo_data.set(d);
            written_bytes += word.len();
            written_words += 1;
        }

        // Make sure we wrote 128 bits (4 words)
        for _ in written_words..4 {
            regs.wfifo_data.set(0);
        }

        written_bytes
    }

    pub fn read_data(&self, output: &mut [u8]) -> usize {
        let ref regs = unsafe { &*self.regs }.aes;

        let mut i = 0;
        while regs.rfifo_empty.get() == 0 {
            if output.len() > i + 3 {
                let word = regs.rfifo_data.get();
                output[i + 0] = (word >> 0) as u8;
                output[i + 1] = (word >> 8) as u8;
                output[i + 2] = (word >> 16) as u8;
                output[i + 3] = (word >> 24) as u8;
                i += 4;
            } else {
                println!("Can't read any more data");
                break;
            }
        }

        i
    }

    pub fn enable_all_interrupts(&self) {
        self.enable_interrupt(Interrupt::WFIFOOverflow);
        self.enable_interrupt(Interrupt::RFIFOOverflow);
        self.enable_interrupt(Interrupt::RFIFOUnderflow);
        self.enable_interrupt(Interrupt::DoneCipher);
        self.enable_interrupt(Interrupt::DoneKeyExpansion);
        self.enable_interrupt(Interrupt::DoneWipeSecrets);
    }

    pub fn finish(&self) {
        let ref regs = unsafe { &*self.regs }.aes;

        regs.int_enable.set(0);
        regs.ctrl.set(0);
        regs.wipe_secrets.set(1);
    }

    pub fn enable_interrupt(&self, interrupt: Interrupt) {
        let ref regs = unsafe { &*self.regs }.aes;

        let current = regs.int_enable.get();
        regs.int_enable.set(current | (1 << interrupt as usize));
    }

    pub fn clear_interrupt(&self, interrupt: Interrupt) {
        let ref regs = unsafe { &*self.regs }.aes;

        regs.int_state.set(1 << interrupt as usize);
    }

    pub fn handle_interrupt(&self, interrupt: u32) {
        if let ParsedInterrupt::Found(int) = interrupt.into() {
            self.client.map(|_client| match int {
                //Interrupt::DoneCipher => client.crypt_done(None, ),
                _ => println!("Interrupt {:?} fired", int),
            });
            self.clear_interrupt(int);
        } else {
            panic!("AesEngine: Unexpected interrupt: {}", interrupt);
        }
    }
}

pub static mut KEYMGR0_AES: AesEngine = unsafe { AesEngine::new(KEYMGR0_REGS) };
