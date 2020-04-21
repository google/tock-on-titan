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
#![feature(asm, const_fn, lang_items)]
#![feature(in_band_lifetimes)]
#![feature(core_intrinsics)]

extern crate capsules;
#[macro_use(print, println)]
extern crate h1;
#[macro_use(static_init, debug, create_capability)]
extern crate kernel;
extern crate cortexm3;

use capsules::alarm::AlarmDriver;
use capsules::console;
use capsules::virtual_alarm::VirtualMuxAlarm;
use capsules::virtual_uart::UartDevice;


use kernel::{Chip, Platform};
use kernel::capabilities;
use kernel::common::dynamic_deferred_call::{DynamicDeferredCall, DynamicDeferredCallClientState};
use kernel::common::RingBuffer;
use kernel::component::Component;
use kernel::hil;
use kernel::hil::entropy::Entropy32;
use kernel::hil::rng::Rng;
use kernel::mpu::MPU;

use h1::crypto::dcrypto::Dcrypto;
use h1::hil::flash::Flash;
use h1::nvcounter::{FlashCounter,NvCounter};
use h1::timels::Timels;
use h1::usb::{Descriptor, StringDescriptor};

// State for loading apps
const NUM_PROCS: usize = 1;

// how should the kernel respond when a process faults
const FAULT_RESPONSE: kernel::procs::FaultResponse = kernel::procs::FaultResponse::Panic;

// Used by panic_fmt to print chip-specific debugging information.
static mut CHIP: Option<&'static h1::chip::Hotel> = None;

/// Panic handler.
#[cfg(not(test))]
#[panic_handler]
pub unsafe extern "C" fn panic_fmt(pi: &core::panic::PanicInfo) -> ! {
    let led = &mut kernel::hil::led::LedLow::new(&mut h1::gpio::PORT0.pins[0]);
    let writer = &mut h1::io::WRITER;
    kernel::debug::panic(&mut [led], writer, pi, &cortexm3::support::nop, &crate::PROCESSES, &CHIP)
}

#[link_section = ".app_memory"]
static mut APP_MEMORY: [u8; 0xc000] = [0; 0xc000];

static mut PROCESSES: [Option<&'static dyn kernel::procs::ProcessType>; NUM_PROCS] = [None];

/// Dummy buffer that causes the linker to reserve enough space for the stack.
#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x2000] = [0; 0x2000];

pub struct Golf {
    console: &'static capsules::console::Console<'static>,
    gpio: &'static capsules::gpio::GPIO<'static>,
    timer: &'static AlarmDriver<'static, VirtualMuxAlarm<'static, Timels>>,
    ipc: kernel::ipc::IPC,
    digest: &'static h1_syscalls::digest::DigestDriver<'static, h1::crypto::sha::ShaEngine>,
    aes: &'static h1_syscalls::aes::AesDriver<'static>,
    rng: &'static capsules::rng::RngDriver<'static>,
    dcrypto: &'static h1_syscalls::dcrypto::DcryptoDriver<'static>,
    nvcounter: &'static h1_syscalls::nvcounter_syscall::NvCounterSyscall<'static,
        FlashCounter<'static, h1::hil::flash::virtual_flash::FlashUser<'static>>>,
    u2f_usb: &'static h1::usb::driver::U2fSyscallDriver<'static>,
    uint_printer: h1_syscalls::debug_syscall::UintPrinter,
    personality: &'static h1_syscalls::personality::PersonalitySyscall<'static>,
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
    use kernel::hil::time::Alarm;

    h1::init();

    let timerhs = {
        use h1::pmu::*;
        use h1::timeus::Timeus;
        Clock::new(PeripheralClock::Bank1(PeripheralClock1::TimeUs0Timer)).enable();
        Clock::new(PeripheralClock::Bank1(PeripheralClock1::TimeLs0)).enable();
        let timer = Timeus::new(0);
        timer
    };

    timerhs.start();
    let start = timerhs.now();

    {
        use h1::pmu::*;
        Clock::new(PeripheralClock::Bank0(PeripheralClock0::Gpio0)).enable();
        let pinmux = &mut *h1::pinmux::PINMUX;
        // LED_0
        pinmux.dioa11.select.set(h1::pinmux::Function::Gpio0Gpio0);

        // SW1
        pinmux.gpio0_gpio1.select.set(h1::pinmux::SelectablePin::Diom2);
        pinmux.diom2.select.set(h1::pinmux::Function::Gpio0Gpio1);
        pinmux.diom2.control.set(1 << 2 | 1 << 4);

        pinmux.diob1.select.set(h1::pinmux::Function::Uart0Tx);
        pinmux.diob6.control.set(1 << 2 | 1 << 4);
        pinmux.uart0_rx.select.set(h1::pinmux::SelectablePin::Diob6);
    }

    // Create capabilities that the board needs to call certain protected kernel
    // functions.
    let process_mgmt_cap = create_capability!(capabilities::ProcessManagementCapability);
    let main_cap = create_capability!(capabilities::MainLoopCapability);
    let grant_cap = create_capability!(capabilities::MemoryAllocationCapability);

    let kernel = static_init!(kernel::Kernel, kernel::Kernel::new(&PROCESSES));

    let dynamic_deferred_call_clients =
        static_init!([DynamicDeferredCallClientState; 2], Default::default());
    let dynamic_deferred_caller = static_init!(
        DynamicDeferredCall,
        DynamicDeferredCall::new(dynamic_deferred_call_clients)
    );
    DynamicDeferredCall::set_global_instance(dynamic_deferred_caller);

    let uart_mux = components::console::UartMuxComponent::new(&h1::uart::UART0, 115200, dynamic_deferred_caller)
        .finalize(());
    hil::uart::Transmit::set_transmit_client(&h1::uart::UART0, uart_mux);

    // Configure UART speed
    let uart = &h1::uart::UART0;
    uart.config(115200);

    // Create virtual device for console.
    let console_uart = static_init!(UartDevice, UartDevice::new(uart_mux, true));
    console_uart.setup();

    let console = static_init!(
        console::Console<'static>,
        console::Console::new(
            console_uart,
            &mut console::WRITE_BUF,
            &mut console::READ_BUF,
            kernel.create_grant(&grant_cap)
        )
    );
    hil::uart::Transmit::set_transmit_client(console_uart, console);

    // Create virtual device for kernel debug.
    components::debug_writer::DebugWriterComponent::new(uart_mux).finalize(());

    //debug!("Booting.");
    let gpio_pins = static_init!(
        [&'static dyn kernel::hil::gpio::InterruptValuePin; 2],
        [&h1::gpio::PORT0.pins[0], &h1::gpio::PORT0.pins[1]]);

    let gpio = static_init!(
        capsules::gpio::GPIO<'static>,
        capsules::gpio::GPIO::new(gpio_pins, kernel.create_grant(&grant_cap)));
    for pin in gpio_pins.iter() {
        pin.set_client(gpio)
    }

    let alarm_mux = static_init!(
        capsules::virtual_alarm::MuxAlarm<'static, Timels>,
        capsules::virtual_alarm::MuxAlarm::new(&h1::timels::TIMELS0));
    h1::timels::TIMELS0.set_client(alarm_mux);

    // Create flash driver and its virtualization
    let flash_virtual_alarm = static_init!(VirtualMuxAlarm<'static, Timels>,
                                           VirtualMuxAlarm::new(alarm_mux));
    let flash = static_init!(
        h1::hil::flash::FlashImpl<'static, VirtualMuxAlarm<'static, Timels>>,
        h1::hil::flash::FlashImpl::new(flash_virtual_alarm, &*h1::hil::flash::h1_hw::H1_HW));
    flash_virtual_alarm.set_client(flash);

    let flash_mux = static_init!(
        h1::hil::flash::virtual_flash::MuxFlash<'static>,
        h1::hil::flash::virtual_flash::MuxFlash::new(flash));

    let flash_user = static_init!(
        h1::hil::flash::virtual_flash::FlashUser<'static>,
        h1::hil::flash::virtual_flash::FlashUser::new(flash_mux));

    let nvcounter_flash = static_init!(h1::hil::flash::virtual_flash::FlashUser<'static>,
                                       h1::hil::flash::virtual_flash::FlashUser::new(flash_mux));

    flash.set_client(flash_mux);

    let timer_virtual_alarm = static_init!(VirtualMuxAlarm<'static, Timels>,
                                           VirtualMuxAlarm::new(alarm_mux));
    let timer = static_init!(
        AlarmDriver<'static, VirtualMuxAlarm<'static, Timels>>,
        AlarmDriver::new(timer_virtual_alarm, kernel.create_grant(&grant_cap)));
    timer_virtual_alarm.set_client(timer);

    let digest = static_init!(
        h1_syscalls::digest::DigestDriver<'static, h1::crypto::sha::ShaEngine>,
        h1_syscalls::digest::DigestDriver::new(
                &mut h1::crypto::sha::KEYMGR0_SHA,
                kernel.create_grant(&grant_cap)));

    let aes = static_init!(
        h1_syscalls::aes::AesDriver,
        h1_syscalls::aes::AesDriver::new(&mut h1::crypto::aes::KEYMGR0_AES, kernel.create_grant(&grant_cap)));
    h1::crypto::aes::KEYMGR0_AES.set_client(aes);
    aes.initialize(&mut h1_syscalls::aes::AES_BUF);

    h1::crypto::dcrypto::DCRYPTO.initialize();
    let dcrypto = static_init!(
        h1_syscalls::dcrypto::DcryptoDriver<'static>,
        h1_syscalls::dcrypto::DcryptoDriver::new(&mut h1::crypto::dcrypto::DCRYPTO));

    h1::crypto::dcrypto::DCRYPTO.set_client(dcrypto);

    let nvcounter_buffer = static_init!([u32; 1], [0]);
    let nvcounter = static_init!(
        FlashCounter<'static, h1::hil::flash::virtual_flash::FlashUser<'static>>,
        FlashCounter::new(nvcounter_buffer, nvcounter_flash));
    nvcounter_flash.set_client(nvcounter);

    let nvcounter_syscall = static_init!(
        h1_syscalls::nvcounter_syscall::NvCounterSyscall<'static,
            FlashCounter<'static, h1::hil::flash::virtual_flash::FlashUser<'static>>>,
        h1_syscalls::nvcounter_syscall::NvCounterSyscall::new(nvcounter, kernel.create_grant(&grant_cap)));
    nvcounter.set_client(nvcounter_syscall);

    let u2f = static_init!(
        h1::usb::driver::U2fSyscallDriver<'static>,
        h1::usb::driver::U2fSyscallDriver::new(&mut h1::usb::USB0, kernel.create_grant(&grant_cap)));
    h1::usb::u2f::UsbHidU2f::set_u2f_client(&h1::usb::USB0, u2f);


    h1::trng::TRNG0.init();
    let entropy_to_random = static_init!(
        capsules::rng::Entropy32ToRandom<'static>,
        capsules::rng::Entropy32ToRandom::new(&h1::trng::TRNG0)
    );

    let rng = static_init!(
        capsules::rng::RngDriver<'static>,
        capsules::rng::RngDriver::new(
            entropy_to_random,
            kernel.create_grant(&grant_cap)
        )
    );
    h1::trng::TRNG0.set_client(entropy_to_random);
    entropy_to_random.set_client(rng);

    let personality = static_init!(
        h1_syscalls::personality::PersonalitySyscall<'static>,
        h1_syscalls::personality::PersonalitySyscall::new(&mut h1::personality::PERSONALITY,
                                                          kernel.create_grant(&grant_cap)));

    h1::personality::PERSONALITY.set_flash(flash_user);
    h1::personality::PERSONALITY.set_buffer(&mut h1::personality::BUFFER);
    h1::personality::PERSONALITY.set_client(personality);
    flash_user.set_client(&h1::personality::PERSONALITY);

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
    let chip = static_init!(h1::chip::Hotel, h1::chip::Hotel::new());
    chip.mpu().enable_mpu();
    CHIP = Some(chip);

    let end = timerhs.now();
    println!("Tock: booted in {} tics; initializing USB and loading processes.",
             end.wrapping_sub(start));

    h1::usb::USB0.init(&mut h1::usb::EP0_OUT_DESCRIPTORS,
                       &mut h1::usb::EP0_OUT_BUFFERS,
                       &mut h1::usb::EP0_IN_DESCRIPTORS,
                       &mut h1::usb::EP0_IN_BUFFER,
                       &mut h1::usb::EP1_OUT_DESCRIPTOR,
                       &mut h1::usb::EP1_OUT_BUFFER,
                       &mut h1::usb::EP1_IN_DESCRIPTOR,
                       &mut h1::usb::EP1_IN_BUFFER,
                       &mut h1::usb::CONFIGURATION_BUFFER,
                       h1::usb::PHY::A,
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
        nvcounter: nvcounter_syscall,
        rng: rng,
        u2f_usb: u2f,
        personality: personality,
        uint_printer: h1_syscalls::debug_syscall::UintPrinter::new(),
    };

    // Uncomment to initialize NvCounter
    //nvcounter_syscall.initialize();

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
        where F: FnOnce(Option<&dyn kernel::Driver>) -> R
    {
        match driver_num {
            capsules::alarm::DRIVER_NUM                => f(Some(self.timer)),
            capsules::console::DRIVER_NUM              => f(Some(self.console)),
            capsules::gpio::DRIVER_NUM                 => f(Some(self.gpio)),
            capsules::rng::DRIVER_NUM                  => f(Some(self.rng)),
            h1::usb::driver::DRIVER_NUM                => f(Some(self.u2f_usb)),
            h1_syscalls::aes::DRIVER_NUM               => f(Some(self.aes)),
            h1_syscalls::dcrypto::DRIVER_NUM           => f(Some(self.dcrypto)),
            h1_syscalls::debug_syscall::DRIVER_NUM     => f(Some(&self.uint_printer)),
            h1_syscalls::digest::DRIVER_NUM            => f(Some(self.digest)),
            h1_syscalls::nvcounter_syscall::DRIVER_NUM => f(Some(self.nvcounter)),
            h1_syscalls::personality::DRIVER_NUM       => f(Some(self.personality)),
            kernel::ipc::DRIVER_NUM                    => f(Some(&self.ipc)),
            _ =>  f(None),
        }
    }
}
