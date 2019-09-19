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

#![no_std]
#![no_main]
#![feature(asm, const_fn, lang_items, compiler_builtins_lib)]
#![feature(in_band_lifetimes)]
#![feature(infer_outlives_requirements)]
#![feature(panic_implementation)]
#![feature(core_intrinsics)]

extern crate capsules;
extern crate h1b;
#[macro_use(static_init, debug, create_capability)]
extern crate kernel;
extern crate cortexm3;

#[macro_use]
pub mod io;

pub mod digest;
pub mod aes;
pub mod dcrypto;
pub mod dcrypto_test;
pub mod debug_syscall;
mod flash_test;
pub mod personality;
pub mod virtual_flash;

use capsules::alarm::AlarmDriver;
use capsules::console;
use capsules::virtual_alarm::VirtualMuxAlarm;
use capsules::virtual_uart::{UartDevice, UartMux};

use kernel::{Chip, Platform};
use kernel::capabilities;
use kernel::hil;
use kernel::hil::entropy::Entropy32;
use kernel::hil::rng::Rng;
use kernel::mpu::MPU;

use h1b::crypto::dcrypto::Dcrypto;
use h1b::hil::flash::Flash;
use h1b::timels::Timels;
use h1b::usb::{Descriptor, StringDescriptor};


// State for loading apps
const NUM_PROCS: usize = 1;

// how should the kernel respond when a process faults
const FAULT_RESPONSE: kernel::procs::FaultResponse = kernel::procs::FaultResponse::Panic;

#[link_section = ".app_memory"]
static mut APP_MEMORY: [u8; 0xc000] = [0; 0xc000];

static mut PROCESSES: [Option<&'static kernel::procs::ProcessType>; NUM_PROCS] = [None];

/// Dummy buffer that causes the linker to reserve enough space for the stack.
#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x2000] = [0; 0x2000];

pub struct Golf {
    console: &'static capsules::console::Console<'static, UartDevice<'static>>,
    gpio: &'static capsules::gpio::GPIO<'static, h1b::gpio::GPIOPin>,
    timer: &'static AlarmDriver<'static, VirtualMuxAlarm<'static, Timels<'static>>>,
    ipc: kernel::ipc::IPC,
    digest: &'static digest::DigestDriver<'static, h1b::crypto::sha::ShaEngine>,
    aes: &'static aes::AesDriver<'static>,
    rng: &'static capsules::rng::RngDriver<'static>,
    dcrypto: &'static dcrypto::DcryptoDriver<'static>,
    u2f_usb: &'static h1b::usb::driver::U2fSyscallDriver<'static>,
    uint_printer: debug_syscall::UintPrinter,
    personality: &'static personality::PersonalitySyscall<'static>,
}

static mut STRINGS: [StringDescriptor; 7] = [
    StringDescriptor {
        b_length: 4,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0409], // English
    },
    StringDescriptor {
        b_length: 24,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0047, 0x006f, 0x006f, 0x0067, 0x006c, 0x0065, 0x0020, 0x0049, 0x006e, 0x0063, 0x002e], // Google Inc.
    },
    StringDescriptor {
        b_length: 14,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0070, 0x0072, 0x006f, 0x0074, 0x006f, 0x0032], // proto2
    },
    StringDescriptor {
        b_length: 54,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0070, 0x0072, 0x006F, 0x0074, 0x006F, 0x0032, 0x005F, 0x0076, 0x0031, 0x002E, 0x0031, 0x002E, 0x0038, 0x0037, 0x0031, 0x0033, 0x002D, 0x0030, 0x0031, 0x0033, 0x0032, 0x0031, 0x0037, 0x0064, 0x0039, 0x0031], // proto2-...
    },
    // Why does this need 3 l (0x6C)? Linux seems to be truncating last one.
    // Verified GetDescriptor for the String is returning complete information.
    // -pal
    StringDescriptor {
        b_length: 12,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0053, 0x0068, 0x0065, 0x006C, 0x006C, 0x006C], // Shell
    },
    StringDescriptor {
        b_length: 8,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0042, 0x004C, 0x0041, 0x0048],  // BLAH
    },
    StringDescriptor {
        b_length: 20,
        b_descriptor_type: Descriptor::String as u8,
        b_string: &[0x0048, 0x006F, 0x0074, 0x0065, 0x006C, 0x0020, 0x0055, 0x0032, 0x0046], // Hotel U2F
    },
];

#[no_mangle]
pub unsafe fn reset_handler() {

    h1b::init();

    let timerhs = {
        use h1b::pmu::*;
        use h1b::timeus::Timeus;
        Clock::new(PeripheralClock::Bank1(PeripheralClock1::TimeUs0Timer)).enable();
        Clock::new(PeripheralClock::Bank1(PeripheralClock1::TimeLs0)).enable();
        let timer = Timeus::new(0);
        timer
    };

    timerhs.start();
    let start = timerhs.now();

    {
        use h1b::pmu::*;
        Clock::new(PeripheralClock::Bank0(PeripheralClock0::Gpio0)).enable();
        let pinmux = &mut *h1b::pinmux::PINMUX;
        // LED_0
        pinmux.dioa11.select.set(h1b::pinmux::Function::Gpio0Gpio0);

        // SW1
        pinmux.gpio0_gpio1.select.set(h1b::pinmux::SelectablePin::Diom2);
        pinmux.diom2.select.set(h1b::pinmux::Function::Gpio0Gpio1);
        pinmux.diom2.control.set(1 << 2 | 1 << 4);

        pinmux.diob1.select.set(h1b::pinmux::Function::Uart0Tx);
        pinmux.diob6.control.set(1 << 2 | 1 << 4);
        pinmux.uart0_rx.select.set(h1b::pinmux::SelectablePin::Diob6);
    }

    // Create capabilities that the board needs to call certain protected kernel
    // functions.
    let process_mgmt_cap = create_capability!(capabilities::ProcessManagementCapability);
    let main_cap = create_capability!(capabilities::MainLoopCapability);
    let grant_cap = create_capability!(capabilities::MemoryAllocationCapability);

    let kernel = static_init!(kernel::Kernel, kernel::Kernel::new(&PROCESSES));

    let uart_mux = static_init!(
        UartMux<'static>,
        UartMux::new(
            &h1b::uart::UART0,
            &mut capsules::virtual_uart::RX_BUF,
            115200
        )
    );
    hil::uart::UART::set_client(&h1b::uart::UART0, uart_mux);

    // Create virtual device for console.
    let console_uart = static_init!(UartDevice, UartDevice::new(uart_mux, true));
    console_uart.setup();

    let console = static_init!(
        console::Console<UartDevice>,
        console::Console::new(
            console_uart,
            115200,
            &mut console::WRITE_BUF,
            &mut console::READ_BUF,
            kernel.create_grant(&grant_cap)
        )
    );
    hil::uart::UART::set_client(console_uart, console);
    console.initialize();

    // Create virtual device for kernel debug.
    let debugger_uart = static_init!(UartDevice, UartDevice::new(uart_mux, false));
    debugger_uart.setup();
    let debugger = static_init!(
        kernel::debug::DebugWriter,
        kernel::debug::DebugWriter::new(
            debugger_uart,
            &mut kernel::debug::OUTPUT_BUF,
            &mut kernel::debug::INTERNAL_BUF,
        )
    );
    hil::uart::UART::set_client(debugger_uart, debugger);

    let debug_wrapper = static_init!(
        kernel::debug::DebugWriterWrapper,
        kernel::debug::DebugWriterWrapper::new(debugger)
    );
    kernel::debug::set_debug_writer_wrapper(debug_wrapper);

    //debug!("Booting.");
    let gpio_pins = static_init!(
        [&'static h1b::gpio::GPIOPin; 2],
        [&h1b::gpio::PORT0.pins[0], &h1b::gpio::PORT0.pins[1]]);

    let gpio = static_init!(
        capsules::gpio::GPIO<'static, h1b::gpio::GPIOPin>,
        capsules::gpio::GPIO::new(gpio_pins));
    for pin in gpio_pins.iter() {
        pin.set_client(gpio)
    }

    let alarm_mux = static_init!(
        capsules::virtual_alarm::MuxAlarm<'static, Timels<'static>>,
        capsules::virtual_alarm::MuxAlarm::new(&h1b::timels::TIMELS0));
    h1b::timels::TIMELS0.set_client(alarm_mux);

    // Create flash driver and its virtualization
    let flash_virtual_alarm = static_init!(VirtualMuxAlarm<'static, Timels<'static>>,
                                           VirtualMuxAlarm::new(alarm_mux));
    let flash = static_init!(
        h1b::hil::flash::FlashImpl<'static, VirtualMuxAlarm<'static, Timels<'static>>>,
        h1b::hil::flash::FlashImpl::new(flash_virtual_alarm, &*h1b::hil::flash::h1b_hw::H1B_HW));
    flash_virtual_alarm.set_client(flash);

    let flash_mux = static_init!(
        virtual_flash::MuxFlash<'static>,
        virtual_flash::MuxFlash::new(flash));

    let flash_user = static_init!(
        virtual_flash::FlashUser<'static>,
        virtual_flash::FlashUser::new(flash_mux));

    flash.set_client(flash_mux);

    let timer_virtual_alarm = static_init!(VirtualMuxAlarm<'static, Timels<'static>>,
                                           VirtualMuxAlarm::new(alarm_mux));
    let timer = static_init!(
        AlarmDriver<'static, VirtualMuxAlarm<'static, Timels<'static>>>,
        AlarmDriver::new(timer_virtual_alarm, kernel.create_grant(&grant_cap)));
    timer_virtual_alarm.set_client(timer);

    let digest = static_init!(
        digest::DigestDriver<'static, h1b::crypto::sha::ShaEngine>,
        digest::DigestDriver::new(
                &mut h1b::crypto::sha::KEYMGR0_SHA,
                kernel.create_grant(&grant_cap)));

    let aes = static_init!(
        aes::AesDriver,
        aes::AesDriver::new(&mut h1b::crypto::aes::KEYMGR0_AES, kernel.create_grant(&grant_cap)));
    h1b::crypto::aes::KEYMGR0_AES.set_client(aes);
    aes.initialize(&mut aes::AES_BUF);

    h1b::crypto::dcrypto::DCRYPTO.initialize();
    let dcrypto = static_init!(
        dcrypto::DcryptoDriver<'static>,
        dcrypto::DcryptoDriver::new(&mut h1b::crypto::dcrypto::DCRYPTO));

    h1b::crypto::dcrypto::DCRYPTO.set_client(dcrypto);

    let u2f = static_init!(
        h1b::usb::driver::U2fSyscallDriver<'static>,
        h1b::usb::driver::U2fSyscallDriver::new(&mut h1b::usb::USB0, kernel.create_grant(&grant_cap)));
    h1b::usb::u2f::UsbHidU2f::set_u2f_client(&h1b::usb::USB0, u2f);


    h1b::trng::TRNG0.init();
    let entropy_to_random = static_init!(
        capsules::rng::Entropy32ToRandom<'static>,
        capsules::rng::Entropy32ToRandom::new(&h1b::trng::TRNG0)
    );

    let rng = static_init!(
        capsules::rng::RngDriver<'static>,
        capsules::rng::RngDriver::new(
            entropy_to_random,
            kernel.create_grant(&grant_cap)
        )
    );
    h1b::trng::TRNG0.set_client(entropy_to_random);
    entropy_to_random.set_client(rng);

    let personality = static_init!(
        personality::PersonalitySyscall<'static>,
        personality::PersonalitySyscall::new(&mut h1b::personality::PERSONALITY,
                                             kernel.create_grant(&grant_cap)));

    h1b::personality::PERSONALITY.set_flash(flash_user);
    h1b::personality::PERSONALITY.set_buffer(&mut h1b::personality::BUFFER);
    h1b::personality::PERSONALITY.set_client(personality);
    flash_user.set_client(&h1b::personality::PERSONALITY);

    // ** GLOBALSEC **
    // TODO(alevy): refactor out
    {
        use core::intrinsics::volatile_store as vs;
        const GLOBALSEC_BASE:      usize = 0x40090000;

        const CPU0_D_REGION0_CTRL: usize = GLOBALSEC_BASE + 0x0;
        const CPU0_D_REGION1_CTRL: usize = GLOBALSEC_BASE + 0x4;
        const CPU0_D_REGION2_CTRL: usize = GLOBALSEC_BASE + 0x8;
        const CPU0_D_REGION3_CTRL: usize = GLOBALSEC_BASE + 0xc;

        const DDMA0_REGION0_CTRL: usize = GLOBALSEC_BASE + 0x80;
        const DDMA0_REGION1_CTRL: usize = GLOBALSEC_BASE + 0x84;
        const DDMA0_REGION2_CTRL: usize = GLOBALSEC_BASE + 0x88;
        const DDMA0_REGION3_CTRL: usize = GLOBALSEC_BASE + 0x8c;

        const DUSB0_REGION0_CTRL: usize = GLOBALSEC_BASE + 0xc0;
        const DUSB0_REGION1_CTRL: usize = GLOBALSEC_BASE + 0xc4;
        const DUSB0_REGION2_CTRL: usize = GLOBALSEC_BASE + 0xc8;
        const DUSB0_REGION3_CTRL: usize = GLOBALSEC_BASE + 0xcc;

        const FLASH_REGION2_BASE: usize = GLOBALSEC_BASE + 0x240;
        const FLASH_REGION2_SIZE: usize = GLOBALSEC_BASE + 0x244;
        const FLASH_REGION2_CTRL: usize = GLOBALSEC_BASE + 0x0e8;

        vs(CPU0_D_REGION0_CTRL as *mut u32, !0);
        vs(CPU0_D_REGION1_CTRL as *mut u32, !0);
        vs(CPU0_D_REGION2_CTRL as *mut u32, !0);
        vs(CPU0_D_REGION3_CTRL as *mut u32, !0);

        // GLOBALSEC_DDMA0-DDMA3
        vs(DDMA0_REGION0_CTRL as *mut u32, !0);
        vs(DDMA0_REGION1_CTRL as *mut u32, !0);
        vs(DDMA0_REGION2_CTRL as *mut u32, !0);
        vs(DDMA0_REGION3_CTRL as *mut u32, !0);

        // GLOBALSEC_DUSB_REGION0-DUSB_REGION3
        vs(DUSB0_REGION0_CTRL as *mut u32, !0);
        vs(DUSB0_REGION1_CTRL as *mut u32, !0);
        vs(DUSB0_REGION2_CTRL as *mut u32, !0);
        vs(DUSB0_REGION3_CTRL as *mut u32, !0);

        // Flash region initialization. We initialize a single region for the
        // last three pages of the second flash macro, used by Personality (n-3)
        // and the non-volatile counter implementation (n-2, n-1).
        const FLASH_START: usize = 0x40000;
        const FLASH_SIZE: usize = 512 * 1024;
        const FLASH_PAGE_SIZE: usize = 2048;
        vs(FLASH_REGION2_BASE as *mut u32, (FLASH_START + FLASH_SIZE - 3*FLASH_PAGE_SIZE) as u32);
        // The value of the SIZE register is one less than the size of the
        // region, i.e. the last address within the region is the start address
        // + the size register.
        vs(FLASH_REGION2_SIZE as *mut u32, (3*FLASH_PAGE_SIZE - 1) as u32);
        // Enable the region for reads and writes.
        vs(FLASH_REGION2_CTRL as *mut u32, 0b111);
    }

    let mut _ctr = 0;
    let chip = static_init!(h1b::chip::Hotel, h1b::chip::Hotel::new());
    chip.mpu().enable_mpu();

    let end = timerhs.now();
    println!("Tock: booted in {} tics; initializing USB and loading processes.",
             end.wrapping_sub(start));

    h1b::usb::USB0.init(&mut h1b::usb::EP0_OUT_DESCRIPTORS,
                        &mut h1b::usb::EP0_OUT_BUFFERS,
                        &mut h1b::usb::EP0_IN_DESCRIPTORS,
                        &mut h1b::usb::EP0_IN_BUFFER,
                        &mut h1b::usb::EP1_OUT_DESCRIPTOR,
                        &mut h1b::usb::EP1_OUT_BUFFER,
                        &mut h1b::usb::EP1_IN_DESCRIPTOR,
                        &mut h1b::usb::EP1_IN_BUFFER,
                        &mut h1b::usb::CONFIGURATION_BUFFER,
                        h1b::usb::PHY::A,
                        None,
                        Some(0x18d1),  // Google vendor ID
                        Some(0x5026),  // proto2
                        &mut STRINGS);
    let golf2 = Golf {
        console: console,
        gpio: gpio,
        timer: timer,
        ipc: kernel::ipc::IPC::new(kernel, &grant_cap),
        digest: digest,
        aes: aes,
        dcrypto: dcrypto,
        rng: rng,
        u2f_usb: u2f,
        personality: personality,
        uint_printer: debug_syscall::UintPrinter::new(),
    };

    #[allow(unused)]
    let flash_test = static_init!(
        flash_test::FlashTest<
            h1b::hil::flash::FlashImpl<'static, VirtualMuxAlarm<'static, Timels<'static>>>>,
        flash_test::FlashTest::<
            h1b::hil::flash::FlashImpl<'static,
                                       VirtualMuxAlarm<'static, Timels<'static>>>>::new(flash));

    // dcrypto_test::run_dcrypto();
    //    rng_test::run_rng();
    //flash_test.run();

    extern "C" {
        /// Beginning of the ROM region containing app images.
        static _sapps: u8;
    }
    kernel::procs::load_processes(
        kernel,
        chip,
        &_sapps as *const u8,
        &mut APP_MEMORY,
        &mut PROCESSES,
        FAULT_RESPONSE,
        &process_mgmt_cap,
    );
    debug!("Tock: starting main loop.");
    debug!(" ");
    kernel.kernel_loop(&golf2, chip, Some(&golf2.ipc), &main_cap);
}

impl Platform for Golf {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
        where F: FnOnce(Option<&kernel::Driver>) -> R
    {
        match driver_num {
            capsules::console::DRIVER_NUM => f(Some(self.console)),
            capsules::gpio::DRIVER_NUM    => f(Some(self.gpio)),
            digest::DRIVER_NUM            => f(Some(self.digest)),
            capsules::alarm::DRIVER_NUM   => f(Some(self.timer)),
            aes::DRIVER_NUM               => f(Some(self.aes)),
            capsules::rng::DRIVER_NUM     => f(Some(self.rng)),
            kernel::ipc::DRIVER_NUM       => f(Some(&self.ipc)),
            dcrypto::DRIVER_NUM           => f(Some(self.dcrypto)),
            h1b::usb::driver::DRIVER_NUM  => f(Some(self.u2f_usb)),
            personality::DRIVER_NUM       => f(Some(self.personality)),
            debug_syscall::DRIVER_NUM     => f(Some(&self.uint_printer)),
            _ =>  f(None),
        }
    }
}
