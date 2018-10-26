#![no_std]
#![no_main]
#![feature(asm, const_fn, lang_items, compiler_builtins_lib, const_cell_new)]
#![feature(core_intrinsics)]

extern crate capsules;
extern crate hotel;
#[macro_use(static_init,debug)]
extern crate kernel;

#[macro_use]
pub mod io;

pub mod digest;
pub mod aes;
pub mod dcrypto;
pub mod dcrypto_test;

use kernel::{Chip, Platform};
use kernel::mpu::MPU;
use kernel::hil::uart::UART;

use hotel::crypto::dcrypto::Dcrypto;
use hotel::usb::{Descriptor, StringDescriptor};
    
//use kernel::hil::rng::RNG;

// State for loading apps
const NUM_PROCS: usize = 2;

// how should the kernel respond when a process faults
const FAULT_RESPONSE: kernel::procs::FaultResponse = kernel::procs::FaultResponse::Panic;

#[link_section = ".app_memory"]
static mut APP_MEMORY: [u8; 16384] = [0; 16384];

static mut PROCESSES: [Option<&'static mut kernel::procs::Process<'static>>; NUM_PROCS] = [None, None];

pub struct Golf {
    console: &'static capsules::console::Console<'static, hotel::uart::UART>,
    gpio: &'static capsules::gpio::GPIO<'static, hotel::gpio::GPIOPin>,
    timer: &'static capsules::alarm::AlarmDriver<'static, hotel::timels::Timels<'static>>,
    ipc: kernel::ipc::IPC,
    digest: &'static digest::DigestDriver<'static, hotel::crypto::sha::ShaEngine>,
    aes: &'static aes::AesDriver<'static>,
    //rng: &'static capsules::rng::SimpleRng<'static, hotel::trng::Trng<'static>>,
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
        b_string: &[0x0048, 0x0061, 0x0076, 0x0065, 0x006E, 0x0020, 0x0055, 0x0032, 0x0046], // Haven U2F
    },
];

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
                                        &mut capsules::console::READ_BUF,
                                       kernel::Grant::create()));
    hotel::uart::UART0.set_client(console);
    console.initialize();
    let kc = static_init!(capsules::console::App, capsules::console::App::default());
    kernel::debug::assign_console_driver(Some(console), kc);

    let gpio_pins = static_init!(
        [&'static hotel::gpio::GPIOPin; 2],
        [&hotel::gpio::PORT0.pins[0], &hotel::gpio::PORT0.pins[1]]);

    let gpio = static_init!(
        capsules::gpio::GPIO<'static, hotel::gpio::GPIOPin>,
        capsules::gpio::GPIO::new(gpio_pins));
    for pin in gpio_pins.iter() {
        pin.set_client(gpio)
    }

    let timer = static_init!(
        capsules::alarm::AlarmDriver<'static, hotel::timels::Timels<'static>>,
        capsules::alarm::AlarmDriver::new(
            &hotel::timels::TIMELS0, kernel::Grant::create()));
    hotel::timels::TIMELS0.set_client(timer);

    let digest = static_init!(
        digest::DigestDriver<'static, hotel::crypto::sha::ShaEngine>,
        digest::DigestDriver::new(
                &mut hotel::crypto::sha::KEYMGR0_SHA,
                kernel::Grant::create()));

    let aes = static_init!(
        aes::AesDriver,
        aes::AesDriver::new(&mut hotel::crypto::aes::KEYMGR0_AES, kernel::Grant::create()));
    hotel::crypto::aes::KEYMGR0_AES.set_client(aes);

    hotel::crypto::dcrypto::DCRYPTO.initialize();
    let dcrypto = static_init!(
        dcrypto::DcryptoDriver<'static>,
        dcrypto::DcryptoDriver::new(&mut hotel::crypto::dcrypto::DCRYPTO));
    
    hotel::crypto::dcrypto::DCRYPTO.set_client(dcrypto);
        
    /*    hotel::trng::TRNG0.init();
    let rng = static_init!(
        capsules::rng::SimpleRng<'static, hotel::trng::Trng>,
        capsules::rng::SimpleRng::new(&mut hotel::trng::TRNG0, kernel::grant::Grant::create()),
        8);
    hotel::trng::TRNG0.set_client(rng);*/
 
    let golf2 = static_init!(Golf, Golf {
        console: console,
        gpio: gpio,
        timer: timer,
        ipc: kernel::ipc::IPC::new(),
        digest: digest,
        aes: aes,
        dcrypto: dcrypto
//        rng: rng,
    });

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


    
    hotel::usb::USB0.init(&mut hotel::usb::OUT_DESCRIPTORS,
                          &mut hotel::usb::OUT_BUFFERS,
                          &mut hotel::usb::IN_DESCRIPTORS,
                          &mut hotel::usb::IN_BUFFERS,
                          &mut hotel::usb::CONFIGURATION_BUFFER,
                          hotel::usb::PHY::A,
                          None,
                          Some(0x18d1),
                          Some(0x5026),
                          &mut STRINGS);


    let end = timerhs.now();

    println!("Tock 1.0 booting. Initialization took {} tics.",
              end.wrapping_sub(start));


    let mut chip = hotel::chip::Hotel::new();
    chip.mpu().enable_mpu();

// dcrypto_test::run_dcrypto();
//    rng_test::run_rng();

    extern "C" {
        /// Beginning of the ROM region containing app images.
        static _sapps: u8;
    }

    kernel::procs::load_processes(
        &_sapps as *const u8,
        &mut APP_MEMORY,
        &mut PROCESSES,
        FAULT_RESPONSE,
    );

    debug!("Start main loop.");
    debug!("");
    kernel::main(golf2, &mut chip, &mut PROCESSES, Some(&golf2.ipc));
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
