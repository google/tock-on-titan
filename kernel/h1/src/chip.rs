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

use cortexm3;
use crate::crypto;
use crate::gpio;
use kernel::Chip;
use crate::spi_host;
use crate::spi_device;
use crate::timels;
use crate::trng;
use crate::uart;
use crate::usb;

pub struct Hotel {
    mpu: cortexm3::mpu::MPU,
    userspace_kernel_boundary: cortexm3::syscall::SysCall,
    systick: cortexm3::systick::SysTick,
}

impl Hotel {
    pub unsafe fn new() -> Hotel {
        Hotel {
            mpu: cortexm3::mpu::MPU::new(),
            userspace_kernel_boundary: cortexm3::syscall::SysCall::new(),
            systick: cortexm3::systick::SysTick::new(),
        }
    }
}

impl Chip for Hotel {
    type MPU = cortexm3::mpu::MPU;
    type UserspaceKernelBoundary = cortexm3::syscall::SysCall;
    type SysTick = cortexm3::systick::SysTick;

    fn has_pending_interrupts(&self) -> bool {
        unsafe { cortexm3::nvic::next_pending().is_some() }
    }

    fn service_pending_interrupts(&self) {
        unsafe {
            while let Some(nvic_num) = cortexm3::nvic::next_pending() {
                match nvic_num {
                    1 | 3 | 6 | 7 | 8 | 9 | 10 | 11 => crypto::dcrypto::DCRYPTO.handle_error_interrupt(nvic_num),
                    2 => crypto::dcrypto::DCRYPTO.handle_wipe_interrupt(),
                    4 => crypto::dcrypto::DCRYPTO.handle_done_interrupt(),
                    5 => crypto::dcrypto::DCRYPTO.handle_receive_interrupt(),

                    //54 => (), // KEYMGR HKEY ALERT, ignored
                    104..=109 => crypto::aes::KEYMGR0_AES.handle_interrupt(nvic_num),

                    110 => crypto::sha::KEYMGR0_SHA.handle_interrupt(nvic_num),
                    111 => (), // KEYMGR0_SHA_WFIFO_FULL

                    127 => spi_host::SPI_HOST0.handle_interrupt(),
                    128 => spi_host::SPI_HOST1.handle_interrupt(),

                    131 => spi_device::SPI_DEVICE0.handle_interrupt_cmd_addr_fifo_not_empty(),

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

                    pin @ 65..=80 => {
                        gpio::PORT0.pins[(pin - 65) as usize].handle_interrupt();
                    }
                    81 => {
                        // GPIO Combined interrupt... why does this remain asserted?
                    }
                    pin @ 82..=97 => {
                        gpio::PORT1.pins[(pin - 82) as usize].handle_interrupt();
                    }
                    98 => {
                        // GPIO Combined interrupt... why does this remain asserted?
                    }
                    _ => panic!("Unexpected ISR {}", nvic_num),
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

    fn userspace_kernel_boundary(&self) -> &cortexm3::syscall::SysCall {
        &self.userspace_kernel_boundary
    }

    fn sleep(&self) {
        unsafe {
                cortexm3::scb::unset_sleepdeep();
        }

        unsafe {
            cortexm3::support::wfi();
        }
    }

    unsafe fn atomic<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        cortexm3::support::atomic(f)
    }

    unsafe fn print_state(&self, writer: &mut dyn core::fmt::Write) {
        cortexm3::print_cortexm3_state(writer);
    }
}
