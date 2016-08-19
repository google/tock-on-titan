use cortexm3;
use main::Chip;
use usb;

pub struct Hotel {
    mpu: cortexm3::mpu::MPU,
    systick: &'static cortexm3::systick::SysTick
}

impl Hotel {
    pub unsafe fn new() -> Hotel {
        Hotel {
            mpu: cortexm3::mpu::MPU::new(),
            systick: cortexm3::systick::SysTick::new()
        }
    }
}

impl Chip for Hotel {
    type MPU = cortexm3::mpu::MPU;
    type SysTick = cortexm3::systick::SysTick;

    fn has_pending_interrupts(&self) -> bool {
        unsafe {
            cortexm3::nvic::next_pending().is_some()
        }
    }

    fn service_pending_interrupts(&mut self) {
        unsafe {
            loop {
                match cortexm3::nvic::next_pending() {
                    Some(nvic_num) => {
                        match nvic_num {
                            193 => usb::USB0.handle_interrupt(),
                            _   => {}
                        }
                        cortexm3::nvic::Nvic::new(nvic_num).clear_pending();
                    },
                    None => break
                }
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
