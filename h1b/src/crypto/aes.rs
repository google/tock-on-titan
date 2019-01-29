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
use kernel::hil::symmetric_encryption::{AES128, AES128Ctr, AES128CBC,  Client};
use kernel::hil::symmetric_encryption::{AES128_BLOCK_SIZE};
use kernel::common::cells::OptionalCell;
use kernel::common::cells::TakeCell;
use kernel::ReturnCode;

pub trait AES128Ecb {
    /// Call before `AES128::crypt()` to perform AES128Ecb
    fn set_mode_aes128ecb(&self, encrypting: bool);
}

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
    output: TakeCell<'a, [u8]>, // If output is None, put result into input buffer
    input: TakeCell<'a, [u8]>,
    read_index: Cell<usize>,
    write_index: Cell<usize>,
    stop_index: Cell<usize>,
}

impl<'a> AES128<'a> for AesEngine<'a> {
    fn enable(&self) {
        self.setup();
    }

    fn disable(&self) {}

    fn set_client(&'a self, client: &'a Client<'a>) {
        self.set_client(client);
    }

    fn set_key(&self, key: &[u8]) -> ReturnCode {
        if key.len() != 16 {
            return ReturnCode::ESIZE;
        }
        let mut key32: [u32; 8] = [0; 8];
        for i in 0..4 {
            key32[i] = (key[4 * i + 0] as u32) |
                       (key[4 * i + 1] as u32) << 8 |
                       (key[4 * i + 2] as u32) << 16 |
                       (key[4 * i + 3] as u32) << 24;
        }
        self.install_key(KeySize::KeySize128, &key32);
        ReturnCode::SUCCESS
    }


    fn set_iv(&self, iv: &[u8]) -> ReturnCode {
        let ref regs = unsafe { &*self.regs }.aes;
        if iv.len() != AES128_BLOCK_SIZE {
            return ReturnCode::ESIZE;
        }
        // For each of 4 words in CTR
        for i in 0..4 {
            let mut val: u32 = 0;
            // OR in each byte of the word
            for b in 0..4 {
                let index = (4 * i) + b;
                val |= (iv[index] as u32) << (b * 8);
            }
            regs.ctr[i].set(val);
        }
        ReturnCode::SUCCESS
    }

    fn start_message(&self) {
        // Initialization vector not supported yet.
    }

    fn crypt(
        &'a self,
        source: Option<&'a mut [u8]>,
        dest: &'a mut [u8],
        start_index: usize,
        stop_index: usize,
    ) -> Option<(ReturnCode, Option<&'a mut [u8]>, &'a mut [u8])> {
        if self.input.is_some() {
            Some((ReturnCode::EBUSY, source, dest))
        } else {
            self.input.put(source);
            self.output.replace(dest);
            if self.try_set_indices(start_index, stop_index) {
                if self.input.is_some() {
                    self.input.map(|buf| self.crypt(&buf[start_index..stop_index]));
                } else {
                    self.output.map(|buf| self.crypt(&buf[start_index..stop_index]));
                }
                None
            } else {
                Some((ReturnCode::EINVAL,
                      self.input.take(),
                      self.output.take().unwrap(),
                ))
            }
        }
    }
}

impl<'a> AES128Ecb for AesEngine<'a> {
    fn set_mode_aes128ecb(&self, encrypting: bool) {
        self.set_cipher_mode(CipherMode::Ecb);
        self.set_encrypt_mode(encrypting);
    }
}

impl<'a> AES128CBC for AesEngine<'a> {
    fn set_mode_aes128cbc(&self, encrypting: bool) {
        self.set_cipher_mode(CipherMode::Cbc);
        self.set_encrypt_mode(encrypting);
    }
}

impl<'a> AES128Ctr for AesEngine<'a> {
    fn set_mode_aes128ctr(&self, _encrypting: bool) {
        self.set_cipher_mode(CipherMode::Ctr);
        self.set_encrypt_mode(true);
    }
}



impl<'a> AesEngine<'a> {
    const unsafe fn new(regs: *mut Registers) -> AesEngine<'a> {
        AesEngine {
            regs: regs,
            client: OptionalCell::empty(),
            input: TakeCell::empty(),
            output: TakeCell::empty(),
            read_index: Cell::new(0),
            write_index: Cell::new(0),
            stop_index: Cell::new(0),
        }
    }

    fn try_set_indices(&self, start_index: usize, stop_index: usize)  -> bool {
        stop_index.checked_sub(start_index).map_or(false, |sublen| {
            sublen % AES128_BLOCK_SIZE == 0 && {
                self.input.map_or_else(
                    || {
                        // The destination buffer is also the input
                        if self.output.map_or(false, |dest| stop_index <= dest.len()) {
                            self.write_index.set(start_index);
                            self.read_index.set(start_index);
                            self.stop_index.set(stop_index);
                            true
                        } else {
                            false
                        }
                    },
                    |source| {
                        if sublen == source.len()
                            && self.output.map_or(false, |dest| stop_index <= dest.len())
                        {
                            // We will start writing to the AES from the beginning of `source`,
                            // and end at its end
                            self.write_index.set(0);

                            // We will start reading from the AES into `dest` at `start_index`,
                            // and continue until `stop_index`
                            self.read_index.set(start_index);
                            self.stop_index.set(stop_index);
                            true
                        } else {
                            false
                        }
                    },
                )
            }
        })
    }

    pub fn set_client(&self, client: &'a Client<'a>) {
        self.client.set(client);
    }

    pub fn setup(&self) {
        let ref regs = unsafe { &*self.regs }.aes;
        self.enable_interrupt(Interrupt::DoneCipher);
        self.enable_interrupt(Interrupt::DoneKeyExpansion);
        let mut control = regs.ctrl.get();
        control |= AesModule::Enable as u32;
        regs.ctrl.set(control);
    }

    pub fn set_cipher_mode(&self, mode: CipherMode) {
        let ref regs = unsafe { &*self.regs }.aes;
        let mut control = regs.ctrl.get();
        control &= !0x18; // strip out cipher mode bits
        control |= (mode as u32) << 3; //
        regs.ctrl.set(control);
    }

    pub fn install_key(&self, key_size: KeySize, key: &[u32; 8]) {
        let ref regs = unsafe { &*self.regs }.aes;
        regs.ctrl.set((regs.ctrl.get() & !0x6) | (key_size as u32) << 1);
        for (i, word) in key.iter().enumerate() {
            regs.key[i].set(*word);
        }
        regs.key_start.set(1);
        // Wait for key expansion.
        // Blocking here is better than tossing a callback to userspace.
        // Flag will clear when expansion is complete.
        while regs.key_start.get() != 0 {}
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
            self.client.map(|client| match int {
                Interrupt::DoneCipher => client.crypt_done(self.input.take(), self.output.take().unwrap() ),
                _ => {}
            });
            self.clear_interrupt(int);
        } else {
            panic!("AesEngine: Unexpected interrupt: {}", interrupt);
        }
    }
}

pub static mut KEYMGR0_AES: AesEngine = unsafe { AesEngine::new(KEYMGR0_REGS) };
