use core::mem;
use hil::digest::{DigestEngine, DigestMode, DigestError};
use kernel::common::volatile_cell::VolatileCell;
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
    current_mode: Option<DigestMode>,
}

impl ShaEngine {
    const unsafe fn new(regs: *mut Registers) -> ShaEngine {
        ShaEngine {
            regs: regs,
            current_mode: None,
        }
    }
}

pub static mut KEYMGR0_SHA: ShaEngine = unsafe { ShaEngine::new(KEYMGR0_REGS) };

impl DigestEngine for ShaEngine {
    fn initialize(&mut self, mode: DigestMode) -> Result<(), DigestError> {
        let ref regs = unsafe { &*self.regs }.sha;

        // Compile-time check for DigestMode exhaustiveness
        match mode {
            DigestMode::Sha1 |
            DigestMode::Sha256 => (),
        };
        self.current_mode = Some(mode);

        regs.trig.set(ShaTrigMask::Stop as u32);

        let mut flags = ShaCfgEnMask::Livestream as u32 | ShaCfgEnMask::IntEnDone as u32;
        match mode {
            DigestMode::Sha1 => flags |= ShaCfgEnMask::Sha1 as u32,
            DigestMode::Sha256 => (),
        }
        regs.cfg_en.set(flags);

        regs.trig.set(ShaTrigMask::Go as u32);

        Ok(())
    }

    fn update(&mut self, data: &[u8]) -> Result<usize, DigestError> {
        let ref regs = unsafe { &*self.regs }.sha;

        if self.current_mode.is_none() {
            return Err(DigestError::NotConfigured);
        }

        let fifo_u8: &VolatileCell<u8> = unsafe { mem::transmute(&regs.input_fifo) };

        // TODO(yuriks): Feed FIFO word at a time when possible
        for b in data {
            fifo_u8.set(*b);
        }

        Ok(data.len())
    }

    fn finalize(&mut self, output: &mut [u8]) -> Result<usize, DigestError> {
        let ref regs = unsafe { &*self.regs }.sha;

        let expected_output_size = match self.current_mode {
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
