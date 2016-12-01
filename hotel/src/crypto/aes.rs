use core::cell::Cell;
use hil::aes::{self, AesClient, Interrupt, AesModule, ParsedInterrupt};
use super::keymgr::{KEYMGR0_REGS, Registers};

pub struct AesEngine {
    regs: *mut Registers,
    client: Cell<Option<&'static AesClient>>,
}

impl AesEngine {
    const unsafe fn new(regs: *mut Registers) -> AesEngine {
        AesEngine {
            regs: regs,
            client: Cell::new(None),
        }
    }

    pub fn set_client(&self, client: &'static AesClient) {
        self.client.set(Some(client));
    }

    pub fn setup(&self, key_size: aes::KeySize, key: &[u32; 8]) {
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

        let flag = aes::Mode::Encrypt as u32;
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
            self.client.get().map(|client| match int {
                Interrupt::DoneCipher => client.done_cipher(),
                Interrupt::DoneKeyExpansion => client.done_key_expansion(),
                Interrupt::DoneWipeSecrets => client.done_wipe_secrets(),
                _ => println!("Interrupt {:?} fired", int),
            });
            self.clear_interrupt(int);
        } else {
            panic!("AesEngine: Unexpected interrupt: {}", interrupt);
        }
    }
}

pub static mut KEYMGR0_AES: AesEngine = unsafe { AesEngine::new(KEYMGR0_REGS) };
