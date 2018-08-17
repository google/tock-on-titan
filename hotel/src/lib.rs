#![crate_name = "hotel"]
#![crate_type = "rlib"]
#![no_std]
#![feature(asm,core_intrinsics,const_fn)]
#![feature(attr_literals)]

extern crate cortexm3;
extern crate kernel;

#[macro_use]
pub mod io;

pub mod chip;
pub mod crypto;
pub mod gpio;
pub mod hil;
pub mod pinmux;
pub mod pmu;
pub mod timels;
pub mod timeus;
pub mod trng;
pub mod uart;
pub mod usb;

pub mod test_rng;
pub mod test_dcrypto;

unsafe extern "C" fn unhandled_interrupt() {
    let mut interrupt_number: u32;

    // IPSR[8:0] holds the currently active interrupt
    asm!(
        "mrs    r0, ipsr                    "
        : "={r0}"(interrupt_number)
        :
        : "r0"
        :
        );

    interrupt_number = interrupt_number & 0x1ff;

    panic!("Unhandled Interrupt. ISR {} is active.", interrupt_number);
}

extern "C" {
    // _estack is not really a function, but it makes the types work
    // You should never actually invoke it!!
    fn _estack();

    fn SVC_Handler();

    fn generic_isr();

    fn systick_handler();

    fn reset_handler();

    static mut _ero: u32;
    static mut _sdata: u32;
    static mut _edata: u32;
    static mut _sbss: u32;
    static mut _ebss: u32;
}

#[link_section=".vectors"]
#[no_mangle]
#[cfg_attr(rustfmt, rustfmt_skip)]
pub static ISR_VECTOR: [Option<unsafe extern fn()>; 16] = [
    /* Stack top */     Option::Some(_estack),
    /* Reset */         Option::Some(reset_handler),
    /* NMI */           Option::Some(unhandled_interrupt),
    /* Hard Fault */    Option::Some(hard_fault_handler),
    /* MemManage */     Option::Some(unhandled_interrupt),
    /* BusFault */      Option::Some(unhandled_interrupt),
    /* UsageFault*/     Option::Some(unhandled_interrupt),
    None, None, None, None,
    /* SVC */           Option::Some(SVC_Handler),
    /* DebugMon */      Option::Some(unhandled_interrupt),
    None,
    /* PendSV */        Option::Some(unhandled_interrupt),
    /* SysTick */       Option::Some(systick_handler),
];

#[link_section=".irqs"]
#[no_mangle]
pub static IRQS: [unsafe extern "C" fn(); 203] = [generic_isr; 203];

pub unsafe fn init() {
    // Relocate data segment.
    // Assumes data starts right after text segment as specified by the linker
    // file.
    let mut pdest = &mut _sdata as *mut u32;
    let pend = &mut _edata as *mut u32;
    let mut psrc = &_ero as *const u32;

    if psrc != pdest {
        while (pdest as *const u32) < pend {
            *pdest = *psrc;
            pdest = pdest.offset(1);
            psrc = psrc.offset(1);
        }
    }

    // Clear the zero segment (BSS)
    let pzero = &_ebss as *const u32;
    pdest = &mut _sbss as *mut u32;

    while (pdest as *const u32) < pzero {
        *pdest = 0;
        pdest = pdest.offset(1);
    }

    cortexm3::nvic::disable_all();
    cortexm3::nvic::clear_all_pending();
    cortexm3::nvic::enable_all();

    // Disable DCRYPTO program receive interrupt
    cortexm3::nvic::Nvic::new(5).disable();
}

unsafe extern "C" fn hard_fault_handler() {
    use core::intrinsics::offset;

    let faulting_stack: *mut u32;
    let kernel_stack: bool;

    asm!(
        "mov    r1, 0                       \n\
         tst    lr, #4                      \n\
         itte   eq                          \n\
         mrseq  r0, msp                     \n\
         addeq  r1, 1                       \n\
         mrsne  r0, psp                     "
        : "={r0}"(faulting_stack), "={r1}"(kernel_stack)
        :
        : "r0", "r1"
        :
        );

    let stacked_r0: u32 = *offset(faulting_stack, 0);
    let stacked_r1: u32 = *offset(faulting_stack, 1);
    let stacked_r2: u32 = *offset(faulting_stack, 2);
    let stacked_r3: u32 = *offset(faulting_stack, 3);
    let stacked_r12: u32 = *offset(faulting_stack, 4);
    let stacked_lr: u32 = *offset(faulting_stack, 5);
    let stacked_pc: u32 = *offset(faulting_stack, 6);
    let stacked_prs: u32 = *offset(faulting_stack, 7);

    let mode_str = if kernel_stack { "Kernel" } else { "Process" };

    let shcsr: u32 = core::intrinsics::volatile_load(0xE000ED24 as *const u32);
    let cfsr: u32 = core::intrinsics::volatile_load(0xE000ED28 as *const u32);
    let hfsr: u32 = core::intrinsics::volatile_load(0xE000ED2C as *const u32);

    panic!("{} HardFault.\n\
           \tr0  0x{:x}\n\
           \tr1  0x{:x}\n\
           \tr2  0x{:x}\n\
           \tr3  0x{:x}\n\
           \tr12 0x{:x}\n\
           \tlr  0x{:x}\n\
           \tpc  0x{:x}\n\
           \tprs 0x{:x}\n\
           \tsp  0x{:x}\n\
           \tSHCSR 0x{:x}\n\
           \tCFSR  0x{:x}\n\
           \tHSFR  0x{:x}\n\
           ", mode_str,
           stacked_r0, stacked_r1, stacked_r2, stacked_r3,
           stacked_r12, stacked_lr, stacked_pc, stacked_prs,
           faulting_stack as u32, shcsr, cfsr, hfsr);
}
