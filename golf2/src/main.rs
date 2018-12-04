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

use capsules::console;
use capsules::virtual_uart::{UartDevice, UartMux};

use kernel::{Chip, Platform};
use kernel::capabilities;
use kernel::mpu::MPU;
use kernel::hil;

use h1b::crypto::dcrypto::Dcrypto;
use h1b::usb::{Descriptor, StringDescriptor};
use h1b::usb::constants::{U2F_REPORT_SIZE, U2fHidCommand};
use h1b::usb::types::U2fHidCommandFrame;

//use kernel::hil::rng::RNG;

// State for loading apps
const NUM_PROCS: usize = 2;

// how should the kernel respond when a process faults
const FAULT_RESPONSE: kernel::procs::FaultResponse = kernel::procs::FaultResponse::Panic;

#[link_section = ".app_memory"]
static mut APP_MEMORY: [u8; 16384] = [0; 16384];

static mut PROCESSES: [Option<&'static kernel::procs::ProcessType>; NUM_PROCS] = [None, None];

/// Dummy buffer that causes the linker to reserve enough space for the stack.
#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x2000] = [0; 0x2000];

pub struct Golf {
    console: &'static capsules::console::Console<'static, UartDevice<'static>>,
    gpio: &'static capsules::gpio::GPIO<'static, h1b::gpio::GPIOPin>,
    timer: &'static capsules::alarm::AlarmDriver<'static, h1b::timels::Timels<'static>>,
    ipc: kernel::ipc::IPC,
    digest: &'static digest::DigestDriver<'static, h1b::crypto::sha::ShaEngine>,
    aes: &'static aes::AesDriver<'static>,
    //rng: &'static capsules::rng::SimpleRng<'static, h1b::trng::Trng<'static>>,
    dcrypto: &'static dcrypto::DcryptoDriver<'static>,
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

    let timer = static_init!(
        capsules::alarm::AlarmDriver<'static, h1b::timels::Timels<'static>>,
        capsules::alarm::AlarmDriver::new(
            &h1b::timels::TIMELS0, kernel.create_grant(&grant_cap)));
    h1b::timels::TIMELS0.set_client(timer);

    let digest = static_init!(
        digest::DigestDriver<'static, h1b::crypto::sha::ShaEngine>,
        digest::DigestDriver::new(
                &mut h1b::crypto::sha::KEYMGR0_SHA,
                kernel.create_grant(&grant_cap)));

    let aes = static_init!(
        aes::AesDriver,
        aes::AesDriver::new(&mut h1b::crypto::aes::KEYMGR0_AES, kernel.create_grant(&grant_cap)));
    h1b::crypto::aes::KEYMGR0_AES.set_client(aes);

    h1b::crypto::dcrypto::DCRYPTO.initialize();
    let dcrypto = static_init!(
        dcrypto::DcryptoDriver<'static>,
        dcrypto::DcryptoDriver::new(&mut h1b::crypto::dcrypto::DCRYPTO));

    h1b::crypto::dcrypto::DCRYPTO.set_client(dcrypto);

    /*    h1b::trng::TRNG0.init();
    let rng = static_init!(
        capsules::rng::SimpleRng<'static, h1b::trng::Trng>,
        capsules::rng::SimpleRng::new(&mut h1b::trng::TRNG0, kernel::grant::Grant::create()),
        8);
    h1b::trng::TRNG0.set_client(rng);*/

    let golf2 = Golf {
        console: console,
        gpio: gpio,
        timer: timer,
        ipc: kernel::ipc::IPC::new(kernel, &grant_cap),
        digest: digest,
        aes: aes,
        dcrypto: dcrypto
//        rng: rng,
    };

    // ** GLOBALSEC **
    // TODO(alevy): refactor out
    {
        use core::intrinsics::volatile_store as vs;

        vs(0x40090000 as *mut u32, !0);
        vs(0x40090004 as *mut u32, !0);
        vs(0x40090008 as *mut u32, !0);
        vs(0x4009000c as *mut u32, !0);

        // GLOBALSEC_DDMA0-DDMA3
        vs(0x40090080 as *mut u32, !0);
        vs(0x40090084 as *mut u32, !0);
        vs(0x40090088 as *mut u32, !0);
        vs(0x4009008c as *mut u32, !0);

        // GLOBALSEC_DUSB_REGION0-DUSB_REGION3
        vs(0x400900c0 as *mut u32, !0);
        vs(0x400900c4 as *mut u32, !0);
        vs(0x400900c8 as *mut u32, !0);
        vs(0x400900cc as *mut u32, !0);
    }

    let mut _ctr = 0;
    let end = timerhs.now();

    println!("Tock 1.0 booting. Initialization took {} tics.",
             end.wrapping_sub(start));

    let chip = static_init!(h1b::chip::Hotel, h1b::chip::Hotel::new());

    chip.mpu().enable_mpu();

    for _i in 0..1_000_000 {
        _ctr += timerhs.now();
    }

    println!("Tock 1.0 booting. About to initialize USB.");

    h1b::usb::USB0.init(&mut h1b::usb::EP0_OUT_DESCRIPTORS,
                        &mut h1b::usb::EP0_OUT_BUFFERS,
                        &mut h1b::usb::EP0_IN_DESCRIPTORS,
                        &mut h1b::usb::EP0_IN_BUFFERS,
                        &mut h1b::usb::EP1_OUT_DESCRIPTOR,
                        &mut h1b::usb::EP1_OUT_BUFFER,
                        &mut h1b::usb::EP1_IN_DESCRIPTOR,
                        &mut h1b::usb::EP1_IN_BUFFER,
                        &mut h1b::usb::CONFIGURATION_BUFFER,
                        h1b::usb::PHY::A,
                        None,
                        Some(0x18d1),
                        Some(0x5026),
                        &mut STRINGS);



    h1b::usb::UsbHidU2f::reset(&h1b::usb::USB0);
    let mut f = U2fHidCommandFrame {
        channel_id: 0xaa,
        frame_type: 0,
        command: U2fHidCommand::Error as u8,
        bcount_high: 0,
        bcount_low: 1,
        data: [0; U2F_REPORT_SIZE as usize -8],
    };
    f.data[0] = 0x3;
    let mut buf: [u32; 16] = [0; 16];
    f.into_u32_buf(&mut buf);
    h1b::usb::UsbHidU2f::put_frame(&h1b::usb::USB0, &buf);

// dcrypto_test::run_dcrypto();
//    rng_test::run_rng();

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
    debug!("Start main loop.");
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
//            capsules::rng::DRIVER_NUM   => f(Some(self.rng)),
            kernel::ipc::DRIVER_NUM       => f(Some(&self.ipc)),
            dcrypto::DRIVER_NUM           => f(Some(self.dcrypto)),
            _ =>  f(None),
        }
    }
}
