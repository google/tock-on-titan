use cortexm3;
use crypto;
use gpio;
use kernel::Chip;
use timels;
use trng;
use uart;
use usb;

pub struct Hotel {
    mpu: cortexm3::mpu::MPU,
    systick: cortexm3::systick::SysTick,
}

impl Hotel {
    pub unsafe fn new() -> Hotel {
        Hotel {
            mpu: cortexm3::mpu::MPU::new(),
            systick: cortexm3::systick::SysTick::new(),
        }
    }
}

impl Chip for Hotel {
    type MPU = cortexm3::mpu::MPU;
    type SysTick = cortexm3::systick::SysTick;

    fn has_pending_interrupts(&self) -> bool {
        unsafe { cortexm3::nvic::next_pending().is_some() }
    }

    fn service_pending_interrupts(&mut self) {
        unsafe {
            while let Some(nvic_num) = cortexm3::nvic::next_pending() {
                match nvic_num {
                    1 | 3 | 6 | 7 | 8 | 9 | 10 | 11 => crypto::dcrypto::DCRYPTO.handle_error_interrupt(nvic_num),
                    4 => crypto::dcrypto::DCRYPTO.handle_done_interrupt(),
                    5 => crypto::dcrypto::DCRYPTO.handle_receive_interrupt(),
                    
                    104...109 => crypto::aes::KEYMGR0_AES.handle_interrupt(nvic_num),

                    110 => (), // KEYMGR0_DSHA_INT, currently polled
                    111 => (), // KEYMGR0_SHA_WFIFO_FULL

                    159 => timels::TIMELS0.handle_interrupt(),
                    160 => timels::TIMELS1.handle_interrupt(),

                    169 => trng::TRNG0.handle_interrupt(),

                    174 => uart::UART0.handle_rx_interrupt(),
                    177 => uart::UART0.handle_tx_interrupt(),
                    181 => uart::UART1.handle_rx_interrupt(),
                    184 => uart::UART1.handle_tx_interrupt(),
                    188 => uart::UART2.handle_rx_interrupt(),
                    191 => uart::UART2.handle_tx_interrupt(),

                    193 => {
                        usb::USB0.handle_interrupt()
                    },

                    pin @ 65...80 => {
                        gpio::PORT0.pins[(pin - 65) as usize].handle_interrupt();
                    }
                    81 => {
                        // GPIO Combined interrupt... why does this remain asserted?
                    }
                    pin @ 82...97 => {
                        gpio::PORT1.pins[(pin - 82) as usize].handle_interrupt();
                    }
                    98 => {
                        // GPIO Combined interrupt... why does this remain asserted?
                    }
                    _ => panic!("Unexected ISR {}", nvic_num),
                }
                cortexm3::nvic::Nvic::new(nvic_num).clear_pending();
                cortexm3::nvic::Nvic::new(nvic_num).enable();
            }
        }
    }

    fn mpu(&self) -> &Self::MPU {
        &self.mpu
    }

    fn systick(&self) -> &Self::SysTick {
        &self.systick
    }

}
