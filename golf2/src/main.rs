#![no_std]
#![no_main]
#![feature(asm, const_fn, lang_items, compiler_builtins_lib, const_cell_new)]

extern crate capsules;
extern crate compiler_builtins;
extern crate hotel;
#[macro_use(static_init)]
extern crate kernel;

#[macro_use]
pub mod io;

pub mod rng_test;
pub mod digest;
pub mod aes;

use kernel::{Chip, Platform};
use kernel::mpu::MPU;
use kernel::hil::uart::UART;
use kernel::hil::rng::RNG;

// State for loading apps
const NUM_PROCS: usize = 2;

// how should the kernel respond when a process faults
const FAULT_RESPONSE: kernel::process::FaultResponse = kernel::process::FaultResponse::Panic;

#[link_section = ".app_memory"]
static mut APP_MEMORY: [u8; 16384] = [0; 16384];

static mut PROCESSES: [Option<kernel::Process<'static>>; NUM_PROCS] = [None, None];

pub struct Golf {
    console: &'static capsules::console::Console<'static, hotel::uart::UART>,
    gpio: &'static capsules::gpio::GPIO<'static, hotel::gpio::GPIOPin>,
    timer: &'static capsules::alarm::AlarmDriver<'static, hotel::timels::Timels<'static>>,
    ipc: kernel::ipc::IPC,
    digest: &'static digest::DigestDriver<'static, hotel::crypto::sha::ShaEngine>,
    aes: &'static aes::AesDriver<'static>,
    rng: &'static capsules::rng::SimpleRng<'static, hotel::trng::Trng<'static>>,
}

#[no_mangle]
pub unsafe fn reset_handler() {
    hotel::init();

    let timerhs = {
        use hotel::pmu::*;
        use hotel::timeus::Timeus;
        Clock::new(PeripheralClock::Bank1(PeripheralClock1::TimeUs0Timer)).enable();
        Clock::new(PeripheralClock::Bank1(PeripheralClock1::TimeLs0)).enable();
        let timer = Timeus::new(0);
        timer
    };

    timerhs.start();
    let start = timerhs.now();

    {
        use hotel::pmu::*;
        Clock::new(PeripheralClock::Bank0(PeripheralClock0::Gpio0)).enable();
        let pinmux = &mut *hotel::pinmux::PINMUX;
        // LED_0
        pinmux.dioa11.select.set(hotel::pinmux::Function::Gpio0Gpio0);

        // SW1
        pinmux.gpio0_gpio1.select.set(hotel::pinmux::SelectablePin::Diom2);
        pinmux.diom2.select.set(hotel::pinmux::Function::Gpio0Gpio1);
        pinmux.diom2.control.set(1 << 2 | 1 << 4);

        pinmux.diob1.select.set(hotel::pinmux::Function::Uart0Tx);
        pinmux.diob6.control.set(1 << 2 | 1 << 4);
        pinmux.uart0_rx.select.set(hotel::pinmux::SelectablePin::Diob6);
    }

    let console = static_init!(
        capsules::console::Console<'static, hotel::uart::UART>,
        capsules::console::Console::new(&hotel::uart::UART0,
                                        115200,
                                       &mut capsules::console::WRITE_BUF,
                                       kernel::grant::Grant::create()),
        24);
    hotel::uart::UART0.set_client(console);
    console.initialize();
    let kc = static_init!(capsules::console::App, capsules::console::App::default());
    kernel::debug::assign_console_driver(Some(console), kc);
    
    let gpio_pins = static_init!(
        [&'static hotel::gpio::GPIOPin; 2],
        [&hotel::gpio::PORT0.pins[0], &hotel::gpio::PORT0.pins[1]],
        8);

    let gpio = static_init!(
        capsules::gpio::GPIO<'static, hotel::gpio::GPIOPin>,
        capsules::gpio::GPIO::new(gpio_pins),
        20);
    for pin in gpio_pins.iter() {
        pin.set_client(gpio)
    }

    let timer = static_init!(
        capsules::alarm::AlarmDriver<'static, hotel::timels::Timels<'static>>,
        capsules::alarm::AlarmDriver::new(
            &hotel::timels::TIMELS0, kernel::Grant::create()),
        12);
    hotel::timels::TIMELS0.set_client(timer);

    let digest = static_init!(
        digest::DigestDriver<'static, hotel::crypto::sha::ShaEngine>,
        digest::DigestDriver::new(
                &mut hotel::crypto::sha::KEYMGR0_SHA,
                kernel::Grant::create()),
        16);

    let aes = static_init!(
        aes::AesDriver,
        aes::AesDriver::new(&mut hotel::crypto::aes::KEYMGR0_AES, kernel::Grant::create()),
        16);
    hotel::crypto::aes::KEYMGR0_AES.set_client(aes);

    hotel::trng::TRNG0.init();
    let rng = static_init!(
        capsules::rng::SimpleRng<'static, hotel::trng::Trng>,
        capsules::rng::SimpleRng::new(&mut hotel::trng::TRNG0, kernel::grant::Grant::create()),
        8);
    hotel::trng::TRNG0.set_client(rng);
 
    let golf2 = static_init!(Golf, Golf {
        console: console,
        gpio: gpio,
        timer: timer,
        ipc: kernel::ipc::IPC::new(),
        digest: digest,
        aes: aes,
        rng: rng,
    }, 8);

    hotel::usb::USB0.init(&mut hotel::usb::OUT_DESCRIPTORS,
                          &mut hotel::usb::OUT_BUFFERS,
                          &mut hotel::usb::IN_DESCRIPTORS,
                          &mut hotel::usb::IN_BUFFERS,
                          hotel::usb::PHY::A,
                          None,
                          Some(0x0011),
                          Some(0x7788));


    let end = timerhs.now();

    println!("Tock 1.0 booting. Initialization took {} tics.\r",
              end.wrapping_sub(start));


    let mut chip = hotel::chip::Hotel::new();
    chip.mpu().enable_mpu();

//    rng_test::run_rng();

    extern "C" {
        /// Beginning of the ROM region containing app images.
        static _sapps: u8;
    }

    kernel::process::load_processes(
        &_sapps as *const u8,
        &mut APP_MEMORY,
        &mut PROCESSES,
        FAULT_RESPONSE,
    );

    //debug!("Loaded processes: finish boot sequence.\r");
    kernel::main(golf2, &mut chip, &mut PROCESSES, &golf2.ipc);
}

impl Platform for Golf {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
        where F: FnOnce(Option<&kernel::Driver>) -> R
    {
        match driver_num {
            capsules::console::DRIVER_NUM => f(Some(self.console)),
            capsules::gpio::DRIVER_NUM    => f(Some(self.gpio)),
            digest::DRIVER_NUM             => f(Some(self.digest)),
            capsules::alarm::DRIVER_NUM => f(Some(self.timer)),
            aes::DRIVER_NUM   => f(Some(self.aes)),
            capsules::rng::DRIVER_NUM   => f(Some(self.rng)),
            kernel::ipc::DRIVER_NUM     => f(Some(&self.ipc)),
            _ => f(None),
        }
    }
}
