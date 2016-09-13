use cortexm3;
use main::Chip;
use gpio;
use uart;

pub struct Hotel {
    mpu: cortexm3::mpu::MPU,
    systick: &'static cortexm3::systick::SysTick,
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
            cortexm3::nvic::next_pending().map(|nvic_num| {
                match nvic_num {
                    174 => uart::UART0.handle_rx_interrupt(),
                    177 => uart::UART0.handle_tx_interrupt(),
                    181 => uart::UART1.handle_rx_interrupt(),
                    184 => uart::UART1.handle_tx_interrupt(),
                    188 => uart::UART2.handle_rx_interrupt(),
                    191 => uart::UART2.handle_tx_interrupt(),

                    pin @ 65 ... 80 => {
                        gpio::PORT0.pins[(pin - 65) as usize].handle_interrupt();
                    },
                    81 => { /* GPIO Combined interrupt... why does this remain asserted? */ },
                    _ => panic!("Unexected ISR {}", nvic_num),
                }
                cortexm3::nvic::Nvic::new(nvic_num).clear_pending();
                cortexm3::nvic::Nvic::new(nvic_num).enable();
            });
        }
    }

    fn mpu(&self) -> &Self::MPU {
        &self.mpu
    }

    fn systick(&self) -> &Self::SysTick {
        &self.systick
    }
}
